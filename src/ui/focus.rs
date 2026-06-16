use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Widget;
use vte4::Terminal;

use crate::pane_tree::{find_best_focus, FocusAxis, Rect};

use super::pane::Pane;

/// Move focus from `from` towards `axis`. Returns the new terminal if any.
pub fn focus_direction(
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
    let from_rect = Rect {
        x: from_rect.x(),
        y: from_rect.y(),
        w: from_rect.width(),
        h: from_rect.height(),
    };
    let candidates: Vec<(Rect, Terminal)> = leaves
        .into_iter()
        .filter(|(t, _)| t != from)
        .map(|(t, r)| {
            (
                Rect {
                    x: r.x(),
                    y: r.y(),
                    w: r.width(),
                    h: r.height(),
                },
                t,
            )
        })
        .collect();
    find_best_focus(&from_rect, &candidates, axis)
}

pub fn collect_leaf_rects(
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
    use super::pane::MAX_TREE_DEPTH;
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
