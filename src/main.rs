use gtk4::Label;
use gtk4::gio::SimpleAction;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, ButtonsType, DrawingArea, HeaderBar,
    MessageDialog, MessageType, Notebook, Orientation, Overlay, Paned, ResponseType, ScrolledWindow,
    SearchBar, SearchEntry, Widget,
};
use std::cell::{Cell, RefCell};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use vte4::Format;
use vte4::Terminal;
use vte4::prelude::*;

mod config;
mod explosion;
mod palette;
mod strings;

use config::Config;
use explosion::ExplosionState;
use strings as s;

fn main() {
    let app = Application::builder().application_id(s::APP_ID).build();

    app.connect_activate(build_ui);
    app.run();
}

/// Compiled regex patterns for URL and email matching
struct MatchRegexes {
    http_regex: vte4::Regex,
    email_regex: vte4::Regex,
}

impl MatchRegexes {
    fn compile() -> Self {
        let http_regex = vte4::Regex::for_match(
            r#"(ftp|http)s?://[^ \t\n\b()<>{}«»\[\]'"]+[^.]"#,
            0x00000008, // PCRE2_CASELESS
        )
        .expect(s::ERROR_COMPILE_HTTP_REGEX);

        let email_regex = vte4::Regex::for_match(
            r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,24}",
            0x00000008, // PCRE2_CASELESS
        )
        .expect(s::ERROR_COMPILE_EMAIL_REGEX);

        Self {
            http_regex,
            email_regex,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitDir {
    /// Side by side (paned laid out horizontally).
    Vertical,
    /// Stacked (paned laid out vertically).
    Horizontal,
}

impl SplitDir {
    fn orientation(self) -> Orientation {
        match self {
            SplitDir::Vertical => Orientation::Horizontal,
            SplitDir::Horizontal => Orientation::Vertical,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusAxis {
    Left,
    Right,
    Up,
    Down,
}

/// Payload of a leaf in the pane tree.
struct Leaf {
    terminal: Terminal,
    #[allow(dead_code)]
    scrolled: ScrolledWindow,
    #[allow(dead_code)]
    overlay: Overlay,
    #[allow(dead_code)]
    drawing_area: DrawingArea,
    search_bar: SearchBar,
    #[allow(dead_code)]
    search_entry: SearchEntry,
    vbox: Box,
}

/// Recursive pane tree.
enum Pane {
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
    fn widget(&self) -> Widget {
        match self {
            Pane::Leaf(leaf) => leaf.vbox.clone().upcast(),
            Pane::Split { paned, .. } => paned.clone().upcast(),
        }
    }
}

/// Recursively find a VTE terminal widget in a widget subtree
fn find_terminal_in_widget(widget: &gtk4::Widget) -> Option<Terminal> {
    if let Some(terminal) = widget.downcast_ref::<Terminal>() {
        return Some(terminal.clone());
    }
    if let Some(scrolled) = widget.downcast_ref::<ScrolledWindow>() {
        return scrolled
            .child()
            .and_then(|c| find_terminal_in_widget(&c));
    }
    if let Some(overlay) = widget.downcast_ref::<Overlay>() {
        return overlay
            .child()
            .and_then(|c| find_terminal_in_widget(&c));
    }
    if let Some(paned) = widget.downcast_ref::<Paned>() {
        if let Some(start) = paned.start_child() {
            if let Some(t) = find_terminal_in_widget(&start) {
                return Some(t);
            }
        }
        if let Some(end) = paned.end_child() {
            if let Some(t) = find_terminal_in_widget(&end) {
                return Some(t);
            }
        }
        return None;
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        if let Some(terminal) = find_terminal_in_widget(&c) {
            return Some(terminal);
        }
        child = c.next_sibling();
    }
    None
}

/// Walk the pane tree to find the SearchBar paired with the given terminal.
fn search_bar_for_leaf(root: &Rc<RefCell<Pane>>, target: &Terminal) -> Option<SearchBar> {
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

/// Find the VTE terminal widget in the current notebook tab
#[allow(dead_code)]
fn get_current_terminal(notebook: &Notebook) -> Option<Terminal> {
    let page = notebook.nth_page(Some(notebook.current_page()?))?;
    find_terminal_in_widget(&page)
}

/// Maximum depth for Rc-tree traversals. Guards against infinite recursion
/// in the (unlikely) event a cycle is introduced.
const MAX_TREE_DEPTH: usize = 1024;

fn leaf_contains(node: &Rc<RefCell<Pane>>, target: &Terminal) -> bool {
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

/// Build a new leaf with a single VTE terminal widget.
fn build_leaf(config: &Config, regexes: &MatchRegexes) -> Leaf {
    let terminal = Terminal::new();

    let cursor_mode = if config.cursor_blink.unwrap_or(true) {
        vte4::CursorBlinkMode::On
    } else {
        vte4::CursorBlinkMode::Off
    };
    terminal.set_cursor_blink_mode(cursor_mode);
    terminal.set_audible_bell(config.audible_bell.unwrap_or(false));
    terminal.set_mouse_autohide(true);
    terminal.set_scroll_on_output(true);
    terminal.set_scroll_on_keystroke(true);
    terminal.set_scrollback_lines(config.scrollback_lines.unwrap_or(10000));
    terminal.set_enable_sixel(false);

    terminal.match_add_regex(&regexes.http_regex, 0);
    terminal.match_add_regex(&regexes.email_regex, 0);

    let terminal_clone = terminal.clone();
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(1);
    gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);

    gesture.connect_pressed(move |gesture, _n_press, x, y| {
        let ctrl_pressed = gesture.current_event().is_some_and(|e| {
            e.modifier_state()
                .contains(gtk4::gdk::ModifierType::CONTROL_MASK)
        });
        if !ctrl_pressed {
            return;
        }

        let (matched, _tag) = terminal_clone.check_match_at(x, y);
        if let Some(url_str) = matched {
            gesture.set_state(gtk4::EventSequenceState::Claimed);

            let url = if url_str.starts_with("http://")
                || url_str.starts_with("https://")
                || url_str.starts_with("ftp://")
                || url_str.starts_with("ftps://")
            {
                url_str.to_string()
            } else if url_str.contains('@') {
                format!("mailto:{}", url_str)
            } else {
                url_str.to_string()
            };

            let _ = gtk4::gio::AppInfo::launch_default_for_uri(
                &url,
                None::<&gtk4::gio::AppLaunchContext>,
            )
            .map_err(|e| {
                eprintln!(
                    "{}",
                    s::ERROR_FAILED_TO_OPEN_URL
                        .replace("{}", &url)
                        .replace("{}", &e.to_string())
                );
            });
        }
    });
    terminal.add_controller(gesture);

    let palette = config.get_color_palette();
    let colors = palette.get_palette();
    let color_refs: Vec<&gtk4::gdk::RGBA> = colors.iter().collect();
    terminal.set_colors(
        Some(&palette.get_foreground()),
        Some(&palette.get_background()),
        &color_refs,
    );

    let shell = config.get_shell();
    terminal.spawn_async(
        vte4::PtyFlags::DEFAULT,
        None,
        &[&shell],
        &[],
        gtk4::glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        None::<&gtk4::gio::Cancellable>,
        |result| {
            if let Err(e) = result {
                eprintln!(
                    "{}",
                    s::format_single(s::ERROR_LAUNCHING_SHELL, &e.to_string())
                );
            }
        },
    );

    let scrolled_window = ScrolledWindow::builder()
        .child(&terminal)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Always)
        .build();

    let search_entry = SearchEntry::new();

    let prev_button = Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text(s::TOOLTIP_SEARCH_PREVIOUS)
        .build();

    let next_button = Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text(s::TOOLTIP_SEARCH_NEXT)
        .build();

    let search_box = Box::new(Orientation::Horizontal, 6);
    search_box.append(&search_entry);
    search_box.append(&prev_button);
    search_box.append(&next_button);

    let search_bar = SearchBar::builder()
        .child(&search_box)
        .show_close_button(true)
        .build();

    let terminal_for_search = terminal.clone();
    let entry_for_search = search_entry.clone();
    let pending_source: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));
    search_entry.connect_search_changed(move |_entry| {
        if let Some(id) = pending_source.take() {
            id.remove();
        }
        let terminal = terminal_for_search.clone();
        let entry = entry_for_search.clone();
        let pending = pending_source.clone();
        let id = glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            pending.set(None);
            let text = entry.text();
            if text.is_empty() {
                terminal.search_set_regex(None, 0);
            } else {
                if let Ok(regex) = vte4::Regex::for_search(&text, 0x08000000) {
                    terminal.search_set_regex(Some(&regex), 0);
                    terminal.search_find_next();
                }
            }
            glib::ControlFlow::Break
        });
        pending_source.set(Some(id));
    });

    let terminal_for_next = terminal.clone();
    search_entry.connect_activate(move |_| {
        terminal_for_next.search_find_next();
    });

    let terminal_for_prev = terminal.clone();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, modifiers| {
        if keyval == gtk4::gdk::Key::Return
            && modifiers.contains(gtk4::gdk::ModifierType::SHIFT_MASK)
        {
            terminal_for_prev.search_find_previous();
            gtk4::glib::Propagation::Stop
        } else {
            gtk4::glib::Propagation::Proceed
        }
    });
    search_entry.add_controller(key_controller);

    let terminal_for_prev_btn = terminal.clone();
    prev_button.connect_clicked(move |_| {
        terminal_for_prev_btn.search_find_previous();
    });

    let terminal_for_next_btn = terminal.clone();
    next_button.connect_clicked(move |_| {
        terminal_for_next_btn.search_find_next();
    });

    search_bar.connect_entry(&search_entry);

    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&search_bar);

    let overlay = Overlay::new();
    overlay.set_child(Some(&scrolled_window));

    let drawing_area = DrawingArea::new();
    drawing_area.set_can_target(false);
    drawing_area.set_hexpand(true);
    drawing_area.set_vexpand(true);
    overlay.add_overlay(&drawing_area);

    let explosion_state = ExplosionState::new();
    let state_for_draw = explosion_state.clone();
    drawing_area.set_draw_func(move |_da, cr, _w, _h| {
        state_for_draw.borrow().draw(cr);
    });

    if config.bell_effect.unwrap_or(true) {
        let explosion_for_bell = explosion_state.clone();
        let da_for_bell = drawing_area.clone();
        let timer_active = Rc::new(Cell::new(false));
        terminal.connect_bell(move |terminal| {
            let (col, row) = terminal.cursor_position();
            let char_w = terminal.char_width() as f64;
            let char_h = terminal.char_height() as f64;
            let scroll_offset = terminal
                .vadjustment()
                .map(|adj| adj.value() as i64)
                .unwrap_or(0);
            let visible_row = row - scroll_offset;
            let px = (col as f64 + 0.5) * char_w;
            let py = (visible_row as f64 + 0.5) * char_h;

            explosion_for_bell.borrow_mut().trigger(px, py);
            da_for_bell.queue_draw();

            if !timer_active.get() {
                timer_active.set(true);
                let state_for_tick = explosion_for_bell.clone();
                let da_for_tick = da_for_bell.clone();
                let flag = timer_active.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
                    let active = state_for_tick.borrow_mut().tick();
                    da_for_tick.queue_draw();
                    if active {
                        glib::ControlFlow::Continue
                    } else {
                        flag.set(false);
                        glib::ControlFlow::Break
                    }
                });
            }
        });
    }

    vbox.append(&overlay);

    scrolled_window.set_vexpand(true);
    scrolled_window.set_hexpand(true);

    Leaf {
        terminal,
        scrolled: scrolled_window,
        overlay,
        drawing_area,
        search_bar,
        search_entry,
        vbox,
    }
}

/// Defer grab_focus to the next idle callback. This avoids GTK warnings
/// like "gtk_paned_set_focus_child was called on widget (nil) which is not
/// child of paned" when focusing a terminal inside a Paned that is being
/// modified in the same call stack.
fn deferred_grab_focus(terminal: &Terminal) {
    let t = terminal.clone();
    glib::idle_add_local_once(move || {
        t.grab_focus();
    });
}

/// Set the paned's position to half its current allocation. Uses both the
/// realize and map signals plus a one-shot idle callback so the divider
/// is correctly placed regardless of when the paned is mapped to screen.
fn configure_paned_position(paned: &Paned, dir: SplitDir) {
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
fn split_pane(
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
        let _ = page_container.remove(&old_widget);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeafSlot {
    First,
    Second,
}

/// Close the leaf that owns `focus`. Returns the survivor's first terminal
/// on success, or None if the tab should be closed.
fn close_leaf(
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
    let (focused_rc, survivor_rc): (Rc<RefCell<Pane>>, Rc<RefCell<Pane>>) = {
        let p = parent.borrow();
        match &*p {
            Pane::Split { first, second, .. } => {
                if focused_first {
                    (first.clone(), second.clone())
                } else {
                    (second.clone(), first.clone())
                }
            }
            _ => unreachable!(),
        }
    };
    let survivor_widget: Widget = survivor_rc.borrow().widget();
    let _focused_widget: Widget = focused_rc.borrow().widget();
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
        let _ = page_container.remove(&outer_paned);
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
fn first_terminal_rc(node: &Rc<RefCell<Pane>>) -> Option<Terminal> {
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
fn count_leaves(node: &Rc<RefCell<Pane>>) -> usize {
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
fn find_enclosing_paned(
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

/// Move focus from `from` towards `axis`. Returns the new terminal if any.
fn focus_direction(
    root: &Rc<RefCell<Pane>>,
    from: &Terminal,
    axis: FocusAxis,
) -> Option<Terminal> {
    // Collect (terminal, rect) for each leaf, where rect is in the page
    // container's coordinate system. translate_coordinates is GTK4's
    // canonical way to do this.
    let page_widget = root.borrow().widget();
    let mut leaves: Vec<(Terminal, gtk4::gdk::Rectangle)> = Vec::new();
    collect_leaf_rects(root, &page_widget, &mut leaves);
    let from_rect = leaves.iter().find(|(t, _)| t == from)?.1;
    let from_cx = from_rect.x() + from_rect.width() / 2;
    let from_cy = from_rect.y() + from_rect.height() / 2;

    let mut best: Option<(i64, Terminal)> = None;
    for (term, rect) in &leaves {
        if *term == *from {
            continue;
        }
        let cx = rect.x() + rect.width() / 2;
        let cy = rect.y() + rect.height() / 2;
        let dx = cx - from_cx;
        let dy = cy - from_cy;
        let candidate = match axis {
            FocusAxis::Left => {
                if dx >= 0 {
                    continue;
                }
                let y_overlap = rect.y() < from_rect.y() + from_rect.height()
                    && from_rect.y() < rect.y() + rect.height();
                if !y_overlap {
                    continue;
                }
                (dx.abs() as i64) * 1000 + (dy.abs() as i64)
            }
            FocusAxis::Right => {
                if dx <= 0 {
                    continue;
                }
                let y_overlap = rect.y() < from_rect.y() + from_rect.height()
                    && from_rect.y() < rect.y() + rect.height();
                if !y_overlap {
                    continue;
                }
                (dx.abs() as i64) * 1000 + (dy.abs() as i64)
            }
            FocusAxis::Up => {
                if dy >= 0 {
                    continue;
                }
                let x_overlap = rect.x() < from_rect.x() + from_rect.width()
                    && from_rect.x() < rect.x() + rect.width();
                if !x_overlap {
                    continue;
                }
                (dy.abs() as i64) * 1000 + (dx.abs() as i64)
            }
            FocusAxis::Down => {
                if dy <= 0 {
                    continue;
                }
                let x_overlap = rect.x() < from_rect.x() + from_rect.width()
                    && from_rect.x() < rect.x() + rect.width();
                if !x_overlap {
                    continue;
                }
                (dy.abs() as i64) * 1000 + (dx.abs() as i64)
            }
        };
        let is_better = match &best {
            Some((s, _)) => candidate < *s,
            None => true,
        };
        if is_better {
            best = Some((candidate, term.clone()));
        }
    }
    best.map(|(_, t)| t)
}

fn collect_leaf_rects(
    node: &Rc<RefCell<Pane>>,
    page_widget: &Widget,
    out: &mut Vec<(Terminal, gtk4::gdk::Rectangle)>,
) {
    collect_leaf_rects_depth(node, page_widget, out, 0);
}

fn collect_leaf_rects_depth(
    node: &Rc<RefCell<Pane>>,
    page_widget: &Widget,
    out: &mut Vec<(Terminal, gtk4::gdk::Rectangle)>,
    depth: usize,
) {
    if depth > MAX_TREE_DEPTH {
        return;
    }
    enum Kind {
        Leaf(Terminal),
        Split(Rc<RefCell<Pane>>, Rc<RefCell<Pane>>),
    }
    let (kind, widget) = {
        let p = node.borrow();
        let k = match &*p {
            Pane::Leaf(leaf) => Kind::Leaf(leaf.terminal.clone()),
            Pane::Split { first, second, .. } => Kind::Split(first.clone(), second.clone()),
        };
        let w: Widget = match &*p {
            Pane::Leaf(leaf) => leaf.vbox.clone().upcast(),
            Pane::Split { paned, .. } => paned.clone().upcast(),
        };
        (k, w)
    };
    if let Some((x, y)) = widget.translate_coordinates(page_widget, 0.0, 0.0) {
        let alloc = widget.allocation();
        let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, alloc.width(), alloc.height());
        match kind {
            Kind::Leaf(t) => out.push((t, rect)),
            Kind::Split(first, second) => {
                collect_leaf_rects_depth(&first, page_widget, out, depth + 1);
                collect_leaf_rects_depth(&second, page_widget, out, depth + 1);
            }
        }
    }
}

/// Check if a terminal has a foreground process running (not just the shell)
fn has_foreground_process(terminal: &Terminal) -> Option<String> {
    if let Some(pty) = terminal.pty() {
        let fd = pty.fd().as_raw_fd();
        let fg_pgid = unsafe { libc::tcgetpgrp(fd) };
        if fg_pgid > 0 {
            let stat_path = format!("/proc/{}/stat", fg_pgid);
            if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                if let Some(start) = stat_content.find('(') {
                    if let Some(end) = stat_content.rfind(')') {
                        let cmd_name = &stat_content[start + 1..end];
                        let is_shell = matches!(
                            cmd_name,
                            "bash" | "zsh" | "sh" | "fish" | "dash" | "ksh" | "csh" | "tcsh"
                        );
                        if !is_shell {
                            return Some(cmd_name.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Create a close button with icon for a tab
fn create_close_button_with_confirmation(
    notebook: &Notebook,
    page_widget: &Box,
    terminal: &Terminal,
    window: &ApplicationWindow,
) -> Button {
    let close_button = Button::builder()
        .icon_name("window-close-symbolic")
        .has_frame(false)
        .build();

    let notebook_clone = notebook.clone();
    let page_widget_clone = page_widget.clone();
    let terminal_clone = terminal.clone();
    let window_clone = window.clone();

    close_button.connect_clicked(move |_| {
        if let Some(program_name) = has_foreground_process(&terminal_clone) {
            let dialog = MessageDialog::builder()
                .transient_for(&window_clone)
                .modal(true)
                .message_type(MessageType::Warning)
                .buttons(ButtonsType::None)
                .text(s::DIALOG_CLOSE_TAB_TITLE)
                .secondary_text(&s::format_close_tab_message(&program_name))
                .build();

            dialog.add_button(s::BUTTON_CANCEL, ResponseType::Cancel);
            dialog.add_button(s::BUTTON_CLOSE, ResponseType::Accept);
            dialog.set_default_response(ResponseType::Cancel);

            let notebook_for_dialog = notebook_clone.clone();
            let page_widget_for_dialog = page_widget_clone.clone();

            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    close_tab(&notebook_for_dialog, &page_widget_for_dialog);
                }
                dialog.close();
            });

            dialog.show();
        } else {
            close_tab(&notebook_clone, &page_widget_clone);
        }
    });

    close_button
}

/// Close a tab and handle cleanup
fn close_tab(notebook: &Notebook, page_widget: &Box) {
    let n_pages = notebook.n_pages();
    for i in 0..n_pages {
        if let Some(page) = notebook.nth_page(Some(i)) {
            if page == *page_widget {
                notebook.remove_page(Some(i));
                if notebook.n_pages() == 0 {
                    std::process::exit(0);
                } else {
                    notebook.set_show_tabs(notebook.n_pages() > 1);
                    if let Some(current_page) = notebook.current_page() {
                        if let Some(page) = notebook.nth_page(Some(current_page)) {
                            if let Some(terminal) = find_terminal_in_widget(&page) {
    terminal.grab_focus();
                            }
                        }
                    }
                }
                return;
            }
        }
    }
}

/// Set up dynamic tab title updates based on terminal window title.
fn setup_tab_title_update(
    terminal: &Terminal,
    notebook: &Notebook,
    page_widget: &Box,
    window: &ApplicationWindow,
) {
    let notebook_clone = notebook.clone();
    let page_widget_clone = page_widget.clone();
    let window_clone = window.clone();

    terminal.connect_window_title_changed(move |terminal| {
        if let Some(title) = terminal.window_title() {
            let n_pages = notebook_clone.n_pages();
            for i in 0..n_pages {
                if let Some(page) = notebook_clone.nth_page(Some(i)) {
                    if page == page_widget_clone {
                        if let Some(tab_label_widget) = notebook_clone.tab_label(&page_widget_clone)
                        {
                            if let Some(tab_box) = tab_label_widget.downcast_ref::<Box>() {
                                let mut child = tab_box.first_child();
                                while let Some(widget) = child {
                                    if let Some(label) = widget.downcast_ref::<Label>() {
                                        label.set_text(&title);
                                        break;
                                    }
                                    child = widget.next_sibling();
                                }
                            }
                        }

                        if Some(i) == notebook_clone.current_page() {
                            window_clone.set_title(Some(&format!("NTerm - {}", title)));
                        }
                        break;
                    }
                }
            }
        }
    });
}

/// Information needed to manipulate a single tab's pane tree.
struct TabState {
    container: Box,
    pane: Rc<RefCell<Pane>>,
    /// Terminal used for tab title updates and the tab-close confirmation.
    primary_terminal: Terminal,
}

/// Create the first leaf and wrap it in a stable container Box.
fn build_initial_pane(
    config: &Config,
    regexes: &MatchRegexes,
) -> (Box, Rc<RefCell<Pane>>, Terminal) {
    let leaf = build_leaf(config, regexes);
    let terminal = leaf.terminal.clone();
    let pane = Rc::new(RefCell::new(Pane::Leaf(leaf)));
    let container = Box::new(Orientation::Vertical, 0);
    let widget = pane.borrow().widget();
    container.append(&widget);
    (container, pane, terminal)
}

fn build_ui(app: &Application) {
    let config = Rc::new(Config::load());
    let bindings = config.get_bindings();
    let regexes = Rc::new(MatchRegexes::compile());

    let notebook = Notebook::builder()
        .scrollable(true)
        .show_tabs(false)
        .build();
    notebook.set_tab_pos(gtk4::PositionType::Top);

    let header_bar = HeaderBar::new();
    header_bar.set_show_title_buttons(true);
    header_bar.set_title_widget(Some(&Label::new(Some(s::APP_TITLE))));

    let search_button = Button::builder()
        .icon_name("edit-find-symbolic")
        .tooltip_text(s::TOOLTIP_SEARCH)
        .build();
    header_bar.pack_end(&search_button);

    let close_pane_button = Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text(s::TOOLTIP_CLOSE_PANE)
        .sensitive(false)
        .build();
    header_bar.pack_end(&close_pane_button);

    let split_h_button = Button::builder()
        .icon_name("view-split-top-bottom-symbolic")
        .tooltip_text(s::TOOLTIP_SPLIT_HORIZONTAL)
        .build();
    header_bar.pack_end(&split_h_button);

    let split_v_button = Button::builder()
        .icon_name("view-split-left-right-symbolic")
        .tooltip_text(s::TOOLTIP_SPLIT_VERTICAL)
        .build();
    header_bar.pack_end(&split_v_button);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(800)
        .default_height(600)
        .child(&notebook)
        .build();
    window.set_titlebar(Some(&header_bar));

    let focused_terminal: Rc<RefCell<Option<Terminal>>> = Rc::new(RefCell::new(None));
    let tabs: Rc<RefCell<Vec<TabState>>> = Rc::new(RefCell::new(Vec::new()));

    // Create the first terminal tab.
    let (vbox, pane_rc, terminal) = build_initial_pane(&config, &regexes);

    let tab_box = Box::new(Orientation::Horizontal, 6);
    let tab_label = Label::new(Some(s::TAB_LABEL_TERMINAL));
    tab_label.set_hexpand(true);
    tab_label.set_xalign(0.5);
    let close_button = create_close_button_with_confirmation(&notebook, &vbox, &terminal, &window);
    tab_box.append(&tab_label);
    tab_box.append(&close_button);

    notebook.append_page(&vbox, Some(&tab_box));
    notebook.set_tab_reorderable(&vbox, true);
    notebook.set_tab_detachable(&vbox, false);
    notebook.page(&vbox).set_property("tab-expand", true);

    setup_tab_title_update(&terminal, &notebook, &vbox, &window);

    {
        let focused = focused_terminal.clone();
        terminal.connect_has_focus_notify(move |t| {
            if t.has_focus() {
                *focused.borrow_mut() = Some(t.clone());
            }
        });
    }
    *focused_terminal.borrow_mut() = Some(terminal.clone());

    tabs.borrow_mut().push(TabState {
        container: vbox.clone(),
        pane: pane_rc,
        primary_terminal: terminal.clone(),
    });

    // Closure to update the close-pane button sensitivity based on the
    // number of leaves in the current tab. The button is sensitive only
    // when the active tab has more than one pane.
    let refresh_close_pane_button: Rc<dyn Fn()> = {
        let notebook_r = notebook.clone();
        let tabs_r = tabs.clone();
        let btn = close_pane_button.clone();
        Rc::new(move || {
            let current = notebook_r.current_page().unwrap_or(0) as usize;
            let page = notebook_r.nth_page(Some(current as u32));
            let tabs_b = tabs_r.borrow();
            let count = page
                .and_then(|p| tabs_b.iter().find(|t| t.container == p))
                .map(|t| count_leaves(&t.pane))
                .unwrap_or(1);
            btn.set_sensitive(count > 1);
        })
    };
    refresh_close_pane_button();

    // Wire child_exited on the initial terminal.
    {
        let tabs_inner = tabs.clone();
        let notebook_inner = notebook.clone();
        let container_inner = vbox.clone();
        let terminal_inner = terminal.clone();
        let refresh_inner = refresh_close_pane_button.clone();
        terminal.connect_child_exited(move |_t, _| {
            handle_leaf_exit(
                &tabs_inner,
                &notebook_inner,
                &container_inner,
                &terminal_inner,
                &refresh_inner,
            );
        });
    }

    terminal.grab_focus();

    // Search button: toggle search in focused leaf.
    {
        let tabs_sb = tabs.clone();
        let focused_sb = focused_terminal.clone();
        search_button.connect_clicked(move |_| {
            if let Some(focus) = focused_sb.borrow().clone() {
                let tabs_borrow = tabs_sb.borrow();
                for tab in tabs_borrow.iter() {
                    if leaf_contains(&tab.pane, &focus) {
                        if let Some(bar) = search_bar_for_leaf(&tab.pane, &focus) {
                            bar.set_search_mode(!bar.is_search_mode());
                        }
                        break;
                    }
                }
            }
        });
    }

    // Tab switch: update focused terminal and window title.
    let window_for_switch = window.clone();
    let focused_for_switch = focused_terminal.clone();
    let refresh_for_switch = refresh_close_pane_button.clone();
    notebook.connect_switch_page(move |_notebook, page, _page_num| {
        if let Some(terminal) = find_terminal_in_widget(page) {
            terminal.grab_focus();
            let title = terminal.window_title();
            *focused_for_switch.borrow_mut() = Some(terminal);
            if let Some(title) = title {
                window_for_switch.set_title(Some(&format!("NTerm - {}", title)));
            }
        }
        refresh_for_switch();
    });

    // --- Action: new tab ---
    {
        let nb = notebook.clone();
        let cfg = config.clone();
        let rx = regexes.clone();
        let win = window.clone();
        let tabs_a = tabs.clone();
        let focused_a = focused_terminal.clone();
        let refresh_new_tab = refresh_close_pane_button.clone();
        let act = SimpleAction::new("new-tab", None);
        act.connect_activate(move |_, _| {
            let (vbox, pane_rc, terminal) = build_initial_pane(&cfg, &rx);

            let tab_box = Box::new(Orientation::Horizontal, 6);
            let label = Label::new(Some(s::TAB_LABEL_TERMINAL));
            label.set_hexpand(true);
            label.set_xalign(0.5);
            let close_button = create_close_button_with_confirmation(&nb, &vbox, &terminal, &win);
            tab_box.append(&label);
            tab_box.append(&close_button);

            let page_num = nb.append_page(&vbox, Some(&tab_box));
            nb.set_tab_reorderable(&vbox, true);
            nb.set_tab_detachable(&vbox, false);
            nb.page(&vbox).set_property("tab-expand", true);

            nb.set_current_page(Some(page_num));
            nb.set_show_tabs(nb.n_pages() > 1);

            setup_tab_title_update(&terminal, &nb, &vbox, &win);

            {
                let focused = focused_a.clone();
                terminal.connect_has_focus_notify(move |t| {
                    if t.has_focus() {
                        *focused.borrow_mut() = Some(t.clone());
                    }
                });
            }
            {
                let tabs_inner = tabs_a.clone();
                let nb_inner = nb.clone();
                let container_inner = vbox.clone();
                let terminal_inner = terminal.clone();
                let refresh_inner = refresh_new_tab.clone();
                terminal.connect_child_exited(move |_t, _| {
                    handle_leaf_exit(
                        &tabs_inner,
                        &nb_inner,
                        &container_inner,
                        &terminal_inner,
                        &refresh_inner,
                    );
                });
            }
            *focused_a.borrow_mut() = Some(terminal.clone());

            tabs_a.borrow_mut().push(TabState {
                container: vbox.clone(),
                pane: pane_rc,
                primary_terminal: terminal.clone(),
            });

            terminal.grab_focus();
            refresh_new_tab();
        });
        window.add_action(&act);
        if let Some(b) = bindings.new_tab {
            app.set_accels_for_action("win.new-tab", &[&b]);
        }
    }

    // --- Action: prev tab ---
    {
        let nb = notebook.clone();
        let act = SimpleAction::new("prev-tab", None);
        act.connect_activate(move |_, _| {
            let n = nb.n_pages();
            if n > 1 {
                if let Some(cur) = nb.current_page() {
                    let prev = if cur == 0 { n - 1 } else { cur - 1 };
                    nb.set_current_page(Some(prev));
                }
            }
        });
        window.add_action(&act);
        if let Some(b) = bindings.prev_tab {
            app.set_accels_for_action("win.prev-tab", &[&b]);
        }
    }

    // --- Action: next tab ---
    {
        let nb = notebook.clone();
        let act = SimpleAction::new("next-tab", None);
        act.connect_activate(move |_, _| {
            let n = nb.n_pages();
            if n > 1 {
                if let Some(cur) = nb.current_page() {
                    let next = if cur >= n - 1 { 0 } else { cur + 1 };
                    nb.set_current_page(Some(next));
                }
            }
        });
        window.add_action(&act);
        if let Some(b) = bindings.next_tab {
            app.set_accels_for_action("win.next-tab", &[&b]);
        }
    }

    // --- Action: toggle search ---
    {
        let tabs_a = tabs.clone();
        let focused_a = focused_terminal.clone();
        let act = SimpleAction::new("toggle-search", None);
        act.connect_activate(move |_, _| {
            if let Some(focus) = focused_a.borrow().clone() {
                let tabs_borrow = tabs_a.borrow();
                for tab in tabs_borrow.iter() {
                    if leaf_contains(&tab.pane, &focus) {
                        if let Some(bar) = search_bar_for_leaf(&tab.pane, &focus) {
                            bar.set_search_mode(!bar.is_search_mode());
                        }
                        break;
                    }
                }
            }
        });
        window.add_action(&act);
        if let Some(b) = bindings.search {
            app.set_accels_for_action("win.toggle-search", &[&b]);
        }
    }

    // --- Action: copy ---
    {
        let focused_a = focused_terminal.clone();
        let act = SimpleAction::new("copy", None);
        act.connect_activate(move |_, _| {
            if let Some(t) = focused_a.borrow().clone() {
                t.copy_clipboard_format(Format::Text);
            }
        });
        window.add_action(&act);
        if let Some(b) = bindings.copy {
            app.set_accels_for_action("win.copy", &[&b]);
        }
    }

    // --- Action: paste ---
    {
        let focused_a = focused_terminal.clone();
        let act = SimpleAction::new("paste", None);
        act.connect_activate(move |_, _| {
            if let Some(t) = focused_a.borrow().clone() {
                t.paste_clipboard();
            }
        });
        window.add_action(&act);
        if let Some(b) = bindings.paste {
            app.set_accels_for_action("win.paste", &[&b]);
        }
    }

    // --- Helper: perform a split in the current tab ---
    fn do_split(
        dir: SplitDir,
        notebook: &Notebook,
        tabs: &Rc<RefCell<Vec<TabState>>>,
        focused: &Rc<RefCell<Option<Terminal>>>,
        config: &Config,
        regexes: &MatchRegexes,
        window: &ApplicationWindow,
        refresh: &Rc<dyn Fn()>,
    ) {
        let current_idx = match notebook.current_page() {
            Some(i) => i as usize,
            None => return,
        };
        let page = match notebook.nth_page(Some(current_idx as u32)) {
            Some(p) => p,
            None => return,
        };
        let focus = match focused.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let (container, pane, tab_idx) = {
            let tb = tabs.borrow();
            let idx = match tb.iter().position(|t| t.container == page) {
                Some(i) => i,
                None => return,
            };
            (tb[idx].container.clone(), tb[idx].pane.clone(), idx)
        };
        if let Some(new_terminal) = split_pane(&pane, &container, &focus, dir, config, regexes) {
            {
                let focused_inner = focused.clone();
                new_terminal.connect_has_focus_notify(move |t| {
                    if t.has_focus() {
                        *focused_inner.borrow_mut() = Some(t.clone());
                    }
                });
            }
            deferred_grab_focus(&new_terminal);
            *focused.borrow_mut() = Some(new_terminal.clone());
            // Wire child_exited on the new terminal.
            {
                let tabs_inner = tabs.clone();
                let nb_inner = notebook.clone();
                let container_inner = container.clone();
                let terminal_inner = new_terminal.clone();
                let refresh_inner = (*refresh).clone();
                new_terminal.connect_child_exited(move |_t, _| {
                    handle_leaf_exit(
                        &tabs_inner,
                        &nb_inner,
                        &container_inner,
                        &terminal_inner,
                        &refresh_inner,
                    );
                });
            }
            {
                let mut tabs_mut = tabs.borrow_mut();
                if let Some(t) = tabs_mut.get_mut(tab_idx) {
                    t.primary_terminal = new_terminal.clone();
                }
            }
            let _ = window; // silence
            refresh();
        }
    }

    // --- Action: split horizontal (Ctrl+Shift+D) ---
    {
        let nb = notebook.clone();
        let tabs_a = tabs.clone();
        let focused_a = focused_terminal.clone();
        let cfg = config.clone();
        let rx = regexes.clone();
        let win = window.clone();
        let refresh_a = refresh_close_pane_button.clone();
        let act = SimpleAction::new("split-horizontal", None);
        act.connect_activate(move |_, _| {
            do_split(SplitDir::Horizontal, &nb, &tabs_a, &focused_a, &cfg, &rx, &win, &refresh_a);
        });
        window.add_action(&act);
        if let Some(b) = bindings.split_horizontal {
            app.set_accels_for_action("win.split-horizontal", &[&b]);
        }
        let nb2 = notebook.clone();
        let tabs2 = tabs.clone();
        let focused2 = focused_terminal.clone();
        let cfg2 = config.clone();
        let rx2 = regexes.clone();
        let win2 = window.clone();
        let refresh_b = refresh_close_pane_button.clone();
        split_h_button.connect_clicked(move |_| {
            do_split(SplitDir::Horizontal, &nb2, &tabs2, &focused2, &cfg2, &rx2, &win2, &refresh_b);
        });
    }

    // --- Action: split vertical (Ctrl+Shift+E) ---
    {
        let nb = notebook.clone();
        let tabs_a = tabs.clone();
        let focused_a = focused_terminal.clone();
        let cfg = config.clone();
        let rx = regexes.clone();
        let win = window.clone();
        let refresh_a = refresh_close_pane_button.clone();
        let act = SimpleAction::new("split-vertical", None);
        act.connect_activate(move |_, _| {
            do_split(SplitDir::Vertical, &nb, &tabs_a, &focused_a, &cfg, &rx, &win, &refresh_a);
        });
        window.add_action(&act);
        if let Some(b) = bindings.split_vertical {
            app.set_accels_for_action("win.split-vertical", &[&b]);
        }
        let nb2 = notebook.clone();
        let tabs2 = tabs.clone();
        let focused2 = focused_terminal.clone();
        let cfg2 = config.clone();
        let rx2 = regexes.clone();
        let win2 = window.clone();
        let refresh_b = refresh_close_pane_button.clone();
        split_v_button.connect_clicked(move |_| {
            do_split(SplitDir::Vertical, &nb2, &tabs2, &focused2, &cfg2, &rx2, &win2, &refresh_b);
        });
    }

    // --- Helper: perform a close-pane ---
    fn do_close_pane(
        notebook: &Notebook,
        tabs: &Rc<RefCell<Vec<TabState>>>,
        focused: &Rc<RefCell<Option<Terminal>>>,
        window: &ApplicationWindow,
        refresh: &Rc<dyn Fn()>,
    ) {
        let current_idx = match notebook.current_page() {
            Some(i) => i as usize,
            None => return,
        };
        let page = match notebook.nth_page(Some(current_idx as u32)) {
            Some(p) => p,
            None => return,
        };
        let focus = match focused.borrow().clone() {
            Some(t) => t,
            None => return,
        };
        let (container, pane) = {
            let tb = tabs.borrow();
            let idx = match tb.iter().position(|t| t.container == page) {
                Some(i) => i,
                None => return,
            };
            (tb[idx].container.clone(), tb[idx].pane.clone())
        };
        if let Some(program_name) = has_foreground_process(&focus) {
            let dialog = MessageDialog::builder()
                .transient_for(window)
                .modal(true)
                .message_type(MessageType::Warning)
                .buttons(ButtonsType::None)
                .text(s::DIALOG_CLOSE_TAB_TITLE)
                .secondary_text(&s::format_close_tab_message(&program_name))
                .build();
            dialog.add_button(s::BUTTON_CANCEL, ResponseType::Cancel);
            dialog.add_button(s::BUTTON_CLOSE, ResponseType::Accept);
            dialog.set_default_response(ResponseType::Cancel);
            let nb_d = notebook.clone();
            let tabs_d = tabs.clone();
            let container_d = container.clone();
            let pane_d = pane.clone();
            let focused_d = focused.clone();
            let win_d = window.clone();
            let refresh_d = refresh.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    do_close_pane_inner(
                        &nb_d,
                        &tabs_d,
                        &container_d,
                        &pane_d,
                        &focused_d,
                        &win_d,
                        &refresh_d,
                    );
                }
                dialog.close();
            });
            dialog.show();
        } else {
            do_close_pane_inner(notebook, tabs, &container, &pane, focused, window, refresh);
        }
    }

    // --- Action: close pane (Ctrl+Shift+W) ---
    {
        let nb = notebook.clone();
        let tabs_a = tabs.clone();
        let focused_a = focused_terminal.clone();
        let win = window.clone();
        let refresh_a = refresh_close_pane_button.clone();
        let act = SimpleAction::new("close-pane", None);
        act.connect_activate(move |_, _| {
            do_close_pane(&nb, &tabs_a, &focused_a, &win, &refresh_a);
        });
        window.add_action(&act);
        if let Some(b) = bindings.close_pane {
            app.set_accels_for_action("win.close-pane", &[&b]);
        }
        let nb2 = notebook.clone();
        let tabs2 = tabs.clone();
        let focused2 = focused_terminal.clone();
        let win2 = window.clone();
        let refresh_b = refresh_close_pane_button.clone();
        close_pane_button.connect_clicked(move |_| {
            do_close_pane(&nb2, &tabs2, &focused2, &win2, &refresh_b);
        });
    }

    // --- Action: focus left/right/up/down ---
    macro_rules! focus_dir_action {
        ($name:literal, $axis:expr, $accel:expr) => {{
            let nb = notebook.clone();
            let tabs_a = tabs.clone();
            let focused_a = focused_terminal.clone();
            let act = SimpleAction::new($name, None);
            act.connect_activate(move |_, _| {
                let current_idx = match nb.current_page() {
                    Some(i) => i as usize,
                    None => return,
                };
                let page = match nb.nth_page(Some(current_idx as u32)) {
                    Some(p) => p,
                    None => return,
                };
                let focus = match focused_a.borrow().clone() {
                    Some(t) => t,
                    None => return,
                };
                let pane = {
                    let tb = tabs_a.borrow();
                    let idx = match tb.iter().position(|t| t.container == page) {
                        Some(i) => i,
                        None => return,
                    };
                    tb[idx].pane.clone()
                };
                if let Some(target) = focus_direction(&pane, &focus, $axis) {
                    deferred_grab_focus(&target);
                    *focused_a.borrow_mut() = Some(target);
                }
            });
            window.add_action(&act);
            if let Some(b) = $accel {
                app.set_accels_for_action(concat!("win.", $name), &[&b]);
            }
        }};
    }
    focus_dir_action!("focus-left", FocusAxis::Left, bindings.focus_left.as_ref());
    focus_dir_action!("focus-right", FocusAxis::Right, bindings.focus_right.as_ref());
    focus_dir_action!("focus-up", FocusAxis::Up, bindings.focus_up.as_ref());
    focus_dir_action!("focus-down", FocusAxis::Down, bindings.focus_down.as_ref());

    window.present();
}

fn do_close_pane_inner(
    notebook: &Notebook,
    tabs: &Rc<RefCell<Vec<TabState>>>,
    container: &Box,
    pane: &Rc<RefCell<Pane>>,
    focused: &Rc<RefCell<Option<Terminal>>>,
    window: &ApplicationWindow,
    refresh: &Rc<dyn Fn()>,
) {
    let focus = match focused.borrow().clone() {
        Some(t) => t,
        None => return,
    };
    match close_leaf(pane, container, &focus) {
        Ok(Some(new_focus)) => {
            deferred_grab_focus(&new_focus);
            *focused.borrow_mut() = Some(new_focus.clone());
            {
                let mut tabs_mut = tabs.borrow_mut();
                if let Some(pos) = tabs_mut.iter().position(|t| t.container == *container) {
                    tabs_mut[pos].primary_terminal = new_focus;
                }
            }
        }
        Ok(None) => {
            {
                let mut tabs_mut = tabs.borrow_mut();
                if let Some(pos) = tabs_mut.iter().position(|t| t.container == *container) {
                    tabs_mut.remove(pos);
                }
            }
            close_tab(notebook, container);
        }
        Err(()) => {}
    }
    let _ = window; // silence unused
    refresh();
}

/// Handle a child_exited: close the leaf and collapse or close the tab.
fn handle_leaf_exit(
    tabs: &Rc<RefCell<Vec<TabState>>>,
    notebook: &Notebook,
    container: &Box,
    terminal: &Terminal,
    refresh: &Rc<dyn Fn()>,
) {
    let (pane, container_clone) = {
        let tb = tabs.borrow();
        match tb.iter().find(|t| t.container == *container) {
            Some(t) => (t.pane.clone(), t.container.clone()),
            None => return,
        }
    };
    if !leaf_contains(&pane, terminal) {
        return;
    }
    // PTY is already closed by the time child_exited fires; nothing to do.
    match close_leaf(&pane, &container_clone, terminal) {
        Ok(Some(new_focus)) => {
            deferred_grab_focus(&new_focus);
            {
                let mut tabs_mut = tabs.borrow_mut();
                if let Some(pos) = tabs_mut.iter().position(|t| t.container == *container) {
                    tabs_mut[pos].primary_terminal = new_focus;
                }
            }
        }
        Ok(None) => {
            {
                let mut tabs_mut = tabs.borrow_mut();
                if let Some(pos) = tabs_mut.iter().position(|t| t.container == *container) {
                    tabs_mut.remove(pos);
                }
            }
            close_tab(notebook, container);
        }
        Err(()) => {}
    }
    refresh();
}
