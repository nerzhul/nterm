use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, Box, Button, ButtonsType, Label, MessageDialog, MessageType, Notebook,
    Orientation, ResponseType,
};
use vte4::Terminal;

use crate::config::Config;
use crate::strings as s;

use super::leaf::build_leaf;
use super::pane::{close_leaf, leaf_contains, Pane};
use super::process::has_foreground_process;
use super::regexes::MatchRegexes;

/// Information needed to manipulate a single tab's pane tree.
pub struct TabState {
    pub container: Box,
    pub pane: Rc<RefCell<Pane>>,
    /// Terminal used for tab title updates and the tab-close confirmation.
    pub primary_terminal: Terminal,
}

/// Create the first leaf and wrap it in a stable container Box.
pub fn build_initial_pane(
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

/// Recursively find a VTE terminal widget in a widget subtree.
pub fn find_terminal_in_widget(widget: &gtk4::Widget) -> Option<Terminal> {
    if let Some(terminal) = widget.downcast_ref::<Terminal>() {
        return Some(terminal.clone());
    }
    if let Some(scrolled) = widget.downcast_ref::<gtk4::ScrolledWindow>() {
        return scrolled
            .child()
            .and_then(|c| find_terminal_in_widget(&c));
    }
    if let Some(overlay) = widget.downcast_ref::<gtk4::Overlay>() {
        return overlay
            .child()
            .and_then(|c| find_terminal_in_widget(&c));
    }
    if let Some(paned) = widget.downcast_ref::<gtk4::Paned>() {
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

/// Find the VTE terminal widget in the current notebook tab.
#[allow(dead_code)]
pub fn get_current_terminal(notebook: &Notebook) -> Option<Terminal> {
    let page = notebook.nth_page(Some(notebook.current_page()?))?;
    find_terminal_in_widget(&page)
}

/// Create a close button with icon for a tab.
pub fn create_close_button_with_confirmation(
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

/// Close a tab and handle cleanup.
pub fn close_tab(notebook: &Notebook, page_widget: &Box) {
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
pub fn setup_tab_title_update(
    terminal: &Terminal,
    notebook: &Notebook,
    page_widget: &Box,
    window: &ApplicationWindow,
) {
    let notebook_clone = notebook.clone();
    let page_widget_clone = page_widget.clone();
    let window_clone = window.clone();

    terminal.connect_notify_local(Some("window-title"), move |terminal, _pspec| {
        let title = terminal.property::<Option<String>>("window-title");
        if let Some(title) = title {
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

pub fn do_close_pane_inner(
    notebook: &Notebook,
    tabs: &Rc<RefCell<Vec<TabState>>>,
    container: &Box,
    pane: &Rc<RefCell<Pane>>,
    focused: &Rc<RefCell<Option<Terminal>>>,
    refresh: &Rc<dyn Fn()>,
) {
    let focus = match focused.borrow().clone() {
        Some(t) => t,
        None => return,
    };
    match close_leaf(pane, container, &focus) {
        Ok(Some(new_focus)) => {
            super::leaf::deferred_grab_focus(&new_focus);
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
    refresh();
}

/// Handle a child_exited: close the leaf and collapse or close the tab.
pub fn handle_leaf_exit(
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
            super::leaf::deferred_grab_focus(&new_focus);
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
