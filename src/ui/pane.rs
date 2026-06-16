use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, DrawingArea, Orientation, Overlay, Paned, ScrolledWindow, SearchBar, SearchEntry, Widget};
use vte4::Terminal;

use crate::config::Config;

use super::leaf::{build_leaf, Leaf};
use super::regexes::MatchRegexes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDir {
    /// Side by side (paned laid out horizontally).
    Vertical,
    /// Stacked (paned laid out vertically).
    Horizontal,
}

impl SplitDir {
    pub fn orientation(self) -> Orientation {
        match self {
            SplitDir::Vertical => Orientation::Horizontal,
            SplitDir::Horizontal => Orientation::Vertical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafSlot {
    First,
    Second,
}

/// Maximum depth for Rc-tree traversals. Guards against infinite recursion
/// in the (unlikely) event a cycle is introduced.
pub const MAX_TREE_DEPTH: usize = 1024;

/// Recursive pane tree.
pub enum Pane {
    Leaf(Leaf),
    Split {
        #[allow(dead_code)]
        dir: SplitDir,
        paned: Paned,
        first: Rc<RefCell<Pane>>,
        second: Rc<RefCell<Pane>>,
    },
}

impl Pane {
    /// Return the top-level Widget for this subtree.
    /// For a Leaf: the leaf's vbox (a Box).
    /// For a Split: the Paned directly.
    pub fn widget(&self) -> Widget {
        match self {
            Pane::Leaf(leaf) => leaf.vbox.clone().upcast(),
            Pane::Split { paned, .. } => paned.clone().upcast(),
        }
    }
}

/// Walk the pane tree to find the SearchBar paired with the given terminal.
pub fn search_bar_for_leaf(root: &Rc<RefCell<Pane>>, target: &Terminal) -> Option<SearchBar> {
    search_bar_for_leaf_depth(root, target, 0)
}

fn search_bar_for_leaf_depth(
    node: &Rc<RefCell<Pane>>,
    target: &Terminal,
    depth: usize,
) -> Option<SearchBar> {
    if depth > MAX_TREE_DEPTH {
        return None;
    }
    let (leaf_result, first, second) = {
        let p = node.borrow();
        match &*p {
            Pane::Leaf(leaf) => {
                if leaf.terminal == *target {
                    (Some(leaf.search_bar.clone()), None, None)
                } else {
                    (None, None, None)
                }
            }
            Pane::Split { first, second, .. } => {
                (None, Some(first.clone()), Some(second.clone()))
            }
        }
    };
    if let Some(bar) = leaf_result {
        return Some(bar);
    }
    if let (Some(f), Some(s)) = (first, second) {
        return search_bar_for_leaf_depth(&f, target, depth + 1)
            .or_else(|| search_bar_for_leaf_depth(&s, target, depth + 1));
    }
    None
}

pub fn leaf_contains(node: &Rc<RefCell<Pane>>, target: &Terminal) -> bool {
    leaf_contains_depth(node, target, 0)
}

fn leaf_contains_depth(node: &Rc<RefCell<Pane>>, target: &Terminal, depth: usize) -> bool {
    if depth > MAX_TREE_DEPTH {
        return false;
    }
    let (is_leaf_terminal, first, second) = {
        let p = node.borrow();
        match &*p {
            Pane::Leaf(leaf) => (Some(leaf.terminal == *target), None, None),
            Pane::Split { first, second, .. } => {
                (None, Some(first.clone()), Some(second.clone()))
            }
        }
    };
    if let Some(b) = is_leaf_terminal {
        return b;
    }
    let first = first.unwrap();
    let second = second.unwrap();
    leaf_contains_depth(&first, target, depth + 1)
        || leaf_contains_depth(&second, target, depth + 1)
}

/// Set the paned's position to half its current allocation. Uses both the
/// realize and map signals plus a one-shot idle callback so the divider
/// is correctly placed regardless of when the paned is mapped to screen.
pub fn configure_paned_position(paned: &Paned, dir: SplitDir) {
    use std::rc::Rc;
    let done: Rc<std::cell::Cell<bool>> = Rc::new(std::cell::Cell::new(false));
    let try_set = move |p: &Paned, done: &Rc<std::cell::Cell<bool>>| {
        if done.get() {
            return;
        }
        let w = p.allocated_width();
        let h = p.allocated_height();
        if w > 0 || h > 0 {
            match dir {
                SplitDir::Vertical => p.set_position(w / 2),
                SplitDir::Horizontal => p.set_position(h / 2),
            }
            done.set(true);
        }
    };
    let done_r = done.clone();
    paned.connect_realize(move |p| try_set(p, &done_r));
    let done_r2 = done.clone();
    paned.connect_map(move |p| try_set(p, &done_r2));
    let paned_idle = paned.clone();
    let done_r3 = done;
    glib::idle_add_local_once(move || try_set(&paned_idle, &done_r3));
}

/// Split the leaf that owns `focus` in the given direction. Returns the
/// new terminal on success.
pub fn split_pane(
    page_pane: &Rc<RefCell<Pane>>,
    page_container: &Box,
    focus: &Terminal,
    dir: SplitDir,
    config: &Config,
    regexes: &MatchRegexes,
) -> Option<Terminal> {
    // Build the new leaf.
    let new_leaf = build_leaf(config, regexes);
    let new_terminal = new_leaf.terminal.clone();
    let new_pane = Rc::new(RefCell::new(Pane::Leaf(new_leaf)));

    // Find the path from root to the focused leaf, returning the list of
    // ancestor Split Rcs from outermost to the direct parent of the leaf.
    // If the focused leaf IS the root, the path is empty.
    let path: Vec<Rc<RefCell<Pane>>> = {
        fn walk(
            node: &Rc<RefCell<Pane>>,
            target: &Terminal,
            out: &mut Vec<Rc<RefCell<Pane>>>,
            depth: usize,
        ) -> bool {
            if depth > MAX_TREE_DEPTH {
                return false;
            }
            let p = node.borrow();
            match &*p {
                Pane::Leaf(leaf) => leaf.terminal == *target,
                Pane::Split { first, second, .. } => {
                    if walk(first, target, out, depth + 1) {
                        out.push(node.clone());
                        true
                    } else if walk(second, target, out, depth + 1) {
                        out.push(node.clone());
                        true
                    } else {
                        false
                    }
                }
            }
        }
        let mut out = Vec::new();
        if walk(page_pane, focus, &mut out, 0) {
            out.reverse();
            out
        } else {
            return None;
        }
    };

    // For the root case: extract the leaf from page_pane, then build a new
    // Split in its place with two distinct child Rcs (no self-cycle).
    if path.is_empty() {
        // Take the leaf data out of page_pane, leaving a placeholder.
        let old_leaf_rc = {
            let mut p = page_pane.borrow_mut();
            let placeholder = Pane::Leaf(Leaf {
                terminal: focus.clone(),
                scrolled: ScrolledWindow::new(),
                overlay: Overlay::new(),
                drawing_area: DrawingArea::new(),
                search_bar: SearchBar::new(),
                search_entry: SearchEntry::new(),
                vbox: Box::new(Orientation::Vertical, 0),
            });
            let taken = std::mem::replace(&mut *p, placeholder);
            match taken {
                Pane::Leaf(leaf) => Rc::new(RefCell::new(Pane::Leaf(leaf))),
                _ => return None,
            }
        };
        let old_widget: Widget = old_leaf_rc.borrow().widget();
        page_container.remove(&old_widget);

        let paned = Paned::builder()
            .orientation(dir.orientation())
            .shrink_start_child(false)
            .shrink_end_child(false)
            .resize_start_child(true)
            .resize_end_child(true)
            .build();
        {
            let leaf_widget = old_leaf_rc.borrow().widget();
            paned.set_start_child(Some(&leaf_widget));
        }
        let new_widget = new_pane.borrow().widget();
        paned.set_end_child(Some(&new_widget));
        configure_paned_position(&paned, dir);
        page_container.append(&paned);

        *page_pane.borrow_mut() = Pane::Split {
            dir,
            paned: paned.clone(),
            first: old_leaf_rc,
            second: new_pane,
        };
        return Some(new_terminal);
    }

    // Non-root case: build a new Split Rc that takes the focused leaf as
    // first child and the new leaf as second child, then replace the
    // focused leaf's slot in the direct parent Split with this new Split Rc.
    // The direct parent is the last ancestor in the path (closest to the leaf).
    let parent = path.last().expect("path non-empty in non-root case");
    let focused_first = {
        let p = parent.borrow();
        if let Pane::Split { first, .. } = &*p {
            leaf_contains(first, focus)
        } else {
            false
        }
    };
    let focused_pane_rc: Rc<RefCell<Pane>> = {
        let p = parent.borrow();
        match &*p {
            Pane::Split { first, second, .. } => {
                if focused_first {
                    first.clone()
                } else {
                    second.clone()
                }
            }
            _ => return None,
        }
    };
    // Extract the leaf from focused_pane_rc into a fresh Rc (no self-cycle).
    let old_leaf_rc = {
        let mut p = focused_pane_rc.borrow_mut();
        let placeholder = Pane::Leaf(Leaf {
            terminal: focus.clone(),
            scrolled: ScrolledWindow::new(),
            overlay: Overlay::new(),
            drawing_area: DrawingArea::new(),
            search_bar: SearchBar::new(),
            search_entry: SearchEntry::new(),
            vbox: Box::new(Orientation::Vertical, 0),
        });
        let taken = std::mem::replace(&mut *p, placeholder);
        match taken {
            Pane::Leaf(leaf) => Rc::new(RefCell::new(Pane::Leaf(leaf))),
            _ => return None,
        }
    };
    let focused_widget: Widget = old_leaf_rc.borrow().widget();
    let (outer_paned, slot) = {
        let p = parent.borrow();
        if let Pane::Split { paned, .. } = &*p {
            let slot = if paned.start_child().as_ref() == Some(&focused_widget) {
                LeafSlot::First
            } else {
                LeafSlot::Second
            };
            (paned.clone(), slot)
        } else {
            return None;
        }
    };
    // Detach the focused widget from outer_paned.
    match slot {
        LeafSlot::First => outer_paned.set_start_child(None::<&Widget>),
        LeafSlot::Second => outer_paned.set_end_child(None::<&Widget>),
    }

    // Create a new paned wrapping the focused leaf + new leaf.
    let inner_paned = Paned::builder()
        .orientation(dir.orientation())
        .shrink_start_child(false)
        .shrink_end_child(false)
        .resize_start_child(true)
        .resize_end_child(true)
        .build();
    let old_leaf_widget = old_leaf_rc.borrow().widget();
    let new_leaf_widget = new_pane.borrow().widget();
    inner_paned.set_start_child(Some(&old_leaf_widget));
    inner_paned.set_end_child(Some(&new_leaf_widget));
    configure_paned_position(&inner_paned, dir);

    // Splice inner_paned into the outer paned at the focused slot.
    match slot {
        LeafSlot::First => outer_paned.set_start_child(Some(&inner_paned)),
        LeafSlot::Second => outer_paned.set_end_child(Some(&inner_paned)),
    }

    // Replace focused_pane_rc's content with a Split variant referencing
    // two distinct Rcs (no self-cycle).
    *focused_pane_rc.borrow_mut() = Pane::Split {
        dir,
        paned: inner_paned,
        first: old_leaf_rc,
        second: new_pane,
    };
    Some(new_terminal)
}

/// Close the leaf that owns `focus`. Returns the survivor's first terminal
/// on success, or None if the tab should be closed.
pub fn close_leaf(
    page_pane: &Rc<RefCell<Pane>>,
    page_container: &Box,
    focus: &Terminal,
) -> Result<Option<Terminal>, ()> {
    // Single leaf: if it matches, caller should close the tab.
    {
        let p = page_pane.borrow();
        if let Pane::Leaf(leaf) = &*p {
            if leaf.terminal == *focus {
                return Ok(None);
            } else {
                return Err(());
            }
        }
    }

    // Find the Split that directly contains the focused leaf.
    let path: Vec<Rc<RefCell<Pane>>> = {
        fn walk(
            node: &Rc<RefCell<Pane>>,
            target: &Terminal,
            out: &mut Vec<Rc<RefCell<Pane>>>,
            depth: usize,
        ) -> bool {
            if depth > MAX_TREE_DEPTH {
                return false;
            }
            let p = node.borrow();
            match &*p {
                Pane::Leaf(leaf) => leaf.terminal == *target,
                Pane::Split { first, second, .. } => {
                    if walk(first, target, out, depth + 1) {
                        out.push(node.clone());
                        true
                    } else if walk(second, target, out, depth + 1) {
                        out.push(node.clone());
                        true
                    } else {
                        false
                    }
                }
            }
        }
        let mut out = Vec::new();
        if walk(page_pane, focus, &mut out, 0) {
            out.reverse();
            out
        } else {
            return Err(());
        }
    };
    if path.is_empty() {
        return Err(());
    }
    // The direct parent of the focused leaf is the last ancestor in the path.
    let parent = path.last().unwrap();
    let focused_first = {
        let p = parent.borrow();
        if let Pane::Split { first, .. } = &*p {
            leaf_contains(first, focus)
        } else {
            false
        }
    };
    let survivor_rc: Rc<RefCell<Pane>> = {
        let p = parent.borrow();
        match &*p {
            Pane::Split { first, second, .. } => {
                if focused_first {
                    second.clone()
                } else {
                    first.clone()
                }
            }
            _ => unreachable!(),
        }
    };
    let survivor_widget: Widget = survivor_rc.borrow().widget();
    let outer_paned = {
        let p = parent.borrow();
        if let Pane::Split { paned, .. } = &*p {
            paned.clone()
        } else {
            unreachable!()
        }
    };
    // Detach both children from outer_paned. After this, survivor_widget
    // has no parent, which is required before we can place it in another
    // container.
    outer_paned.set_start_child(None::<&Widget>);
    outer_paned.set_end_child(None::<&Widget>);

    // Replace the parent Rc's content with the survivor subtree. Since
    // Pane is not Clone, we use std::mem::replace to move the survivor out.
    {
        let mut survivor_owned = survivor_rc.borrow_mut();
        let placeholder = Pane::Leaf(Leaf {
            terminal: focus.clone(),
            scrolled: ScrolledWindow::new(),
            overlay: Overlay::new(),
            drawing_area: DrawingArea::new(),
            search_bar: SearchBar::new(),
            search_entry: SearchEntry::new(),
            vbox: Box::new(Orientation::Vertical, 0),
        });
        let taken = std::mem::replace(&mut *survivor_owned, placeholder);
        drop(survivor_owned);
        *parent.borrow_mut() = taken;
    }

    // Find the first terminal in the new parent subtree.
    let new_focus = first_terminal_rc(parent);

    // Place survivor_widget in the destination: either the page_container
    // (if parent is the root) or the enclosing paned (if parent is nested).
    // The outer_paned is discarded (it will be dropped when this function
    // returns and its clone goes out of scope).

    if Rc::ptr_eq(parent, page_pane) {
        // parent is the root. Replace in page_container.
        page_container.remove(&outer_paned);
        page_container.append(&survivor_widget);
    } else {
        // parent is inside another Split. Find the paned that holds
        // outer_paned as a child, and replace it with survivor_widget.
        if let Some((enclosing_paned, enc_slot)) =
            find_enclosing_paned(page_pane, &outer_paned.upcast())
        {
            match enc_slot {
                LeafSlot::First => {
                    enclosing_paned.set_start_child(Some(&survivor_widget));
                }
                LeafSlot::Second => {
                    enclosing_paned.set_end_child(Some(&survivor_widget));
                }
            }
        } else {
            return Err(());
        }
    }

    Ok(new_focus)
}

/// Walk the tree to find the first terminal in any leaf (depth-first, first child first).
pub fn first_terminal_rc(node: &Rc<RefCell<Pane>>) -> Option<Terminal> {
    first_terminal_rc_depth(node, 0)
}

fn first_terminal_rc_depth(node: &Rc<RefCell<Pane>>, depth: usize) -> Option<Terminal> {
    if depth > MAX_TREE_DEPTH {
        return None;
    }
    let (leaf_opt, first) = {
        let p = node.borrow();
        match &*p {
            Pane::Leaf(leaf) => (Some(leaf.terminal.clone()), None),
            Pane::Split { first, .. } => (None, Some(first.clone())),
        }
    };
    if let Some(t) = leaf_opt {
        return Some(t);
    }
    if let Some(f) = first {
        return first_terminal_rc_depth(&f, depth + 1);
    }
    None
}

/// Count the number of leaves in the pane tree.
pub fn count_leaves(node: &Rc<RefCell<Pane>>) -> usize {
    count_leaves_depth(node, 0)
}

fn count_leaves_depth(node: &Rc<RefCell<Pane>>, depth: usize) -> usize {
    if depth > MAX_TREE_DEPTH {
        return 0;
    }
    let (is_leaf, first, second) = {
        let p = node.borrow();
        match &*p {
            Pane::Leaf(_) => (true, None, None),
            Pane::Split { first, second, .. } => {
                (false, Some(first.clone()), Some(second.clone()))
            }
        }
    };
    if is_leaf {
        return 1;
    }
    let f = first.unwrap();
    let s = second.unwrap();
    count_leaves_depth(&f, depth + 1) + count_leaves_depth(&s, depth + 1)
}

/// Find the Paned whose start/end child is `target`. Returns the paned and
/// the slot where target was a child.
pub fn find_enclosing_paned(
    node: &Rc<RefCell<Pane>>,
    target: &Widget,
) -> Option<(Paned, LeafSlot)> {
    find_enclosing_paned_depth(node, target, 0)
}

fn find_enclosing_paned_depth(
    node: &Rc<RefCell<Pane>>,
    target: &Widget,
    depth: usize,
) -> Option<(Paned, LeafSlot)> {
    if depth > MAX_TREE_DEPTH {
        return None;
    }
    let (paned, first, second) = {
        let p = node.borrow();
        match &*p {
            Pane::Split { paned, first, second, .. } => (
                Some(paned.clone()),
                Some(first.clone()),
                Some(second.clone()),
            ),
            _ => (None, None, None),
        }
    };
    if let (Some(paned), Some(first), Some(second)) = (paned, first, second) {
        if paned.start_child().as_ref() == Some(target) {
            return Some((paned, LeafSlot::First));
        }
        if paned.end_child().as_ref() == Some(target) {
            return Some((paned, LeafSlot::Second));
        }
        if let Some(found) = find_enclosing_paned_depth(&first, target, depth + 1) {
            return Some(found);
        }
        if let Some(found) = find_enclosing_paned_depth(&second, target, depth + 1) {
            return Some(found);
        }
    }
    None
}
