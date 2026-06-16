use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gio::SimpleAction;
use gtk4::prelude::*;
use gtk4::{Box, Button, Label, Notebook, Orientation};
use vte4::Terminal;
use vte4::prelude::*;

use crate::pane_tree::FocusAxis;

use super::app::AppCtx;
use super::leaf::deferred_grab_focus;
use super::pane::{split_pane, SplitDir};
use super::process::has_foreground_process;
use super::tabs::{build_initial_pane, create_close_button_with_confirmation, do_close_pane_inner, setup_tab_title_update, TabState};
use super::regexes::MatchRegexes;

pub fn register_actions(ctx: &AppCtx) {
    register_new_tab(ctx);
    register_prev_tab(ctx);
    register_next_tab(ctx);
    register_toggle_search(ctx);
    register_copy(ctx);
    register_paste(ctx);
    register_split_horizontal(ctx);
    register_split_vertical(ctx);
    register_close_pane(ctx);
    register_focus_dir(ctx, "focus-left", FocusAxis::Left, ctx.bindings.focus_left.as_ref());
    register_focus_dir(ctx, "focus-right", FocusAxis::Right, ctx.bindings.focus_right.as_ref());
    register_focus_dir(ctx, "focus-up", FocusAxis::Up, ctx.bindings.focus_up.as_ref());
    register_focus_dir(ctx, "focus-down", FocusAxis::Down, ctx.bindings.focus_down.as_ref());
}

fn register_new_tab(ctx: &AppCtx) {
    let nb = ctx.notebook.clone();
    let cfg = ctx.config.clone();
    let rx = ctx.regexes.clone();
    let win = ctx.window.clone();
    let tabs_a = ctx.tabs.clone();
    let focused_a = ctx.focused.clone();
    let refresh_new_tab = ctx.refresh_close_pane_button.clone();
    let act = SimpleAction::new("new-tab", None);
    act.connect_activate(move |_, _| {
        let (vbox, pane_rc, terminal) = build_initial_pane(&cfg, &rx);

        let tab_box = Box::new(Orientation::Horizontal, 6);
        let label = Label::new(Some(crate::strings::TAB_LABEL_TERMINAL));
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
                super::tabs::handle_leaf_exit(
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
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.new_tab {
        ctx.app.set_accels_for_action("win.new-tab", &[b]);
    }
}

fn register_prev_tab(ctx: &AppCtx) {
    let nb = ctx.notebook.clone();
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
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.prev_tab {
        ctx.app.set_accels_for_action("win.prev-tab", &[b]);
    }
}

fn register_next_tab(ctx: &AppCtx) {
    let nb = ctx.notebook.clone();
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
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.next_tab {
        ctx.app.set_accels_for_action("win.next-tab", &[b]);
    }
}

fn register_toggle_search(ctx: &AppCtx) {
    let tabs_a = ctx.tabs.clone();
    let focused_a = ctx.focused.clone();
    let act = SimpleAction::new("toggle-search", None);
    act.connect_activate(move |_, _| {
        if let Some(focus) = focused_a.borrow().clone() {
            let tabs_borrow = tabs_a.borrow();
            for tab in tabs_borrow.iter() {
                if super::pane::leaf_contains(&tab.pane, &focus) {
                    if let Some(bar) = super::pane::search_bar_for_leaf(&tab.pane, &focus) {
                        bar.set_search_mode(!bar.is_search_mode());
                    }
                    break;
                }
            }
        }
    });
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.search {
        ctx.app.set_accels_for_action("win.toggle-search", &[b]);
    }
}

fn register_copy(ctx: &AppCtx) {
    let focused_a = ctx.focused.clone();
    let act = SimpleAction::new("copy", None);
    act.connect_activate(move |_, _| {
        if let Some(t) = focused_a.borrow().clone() {
            t.copy_clipboard_format(vte4::Format::Text);
        }
    });
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.copy {
        ctx.app.set_accels_for_action("win.copy", &[b]);
    }
}

fn register_paste(ctx: &AppCtx) {
    let focused_a = ctx.focused.clone();
    let act = SimpleAction::new("paste", None);
    act.connect_activate(move |_, _| {
        if let Some(t) = focused_a.borrow().clone() {
            t.paste_clipboard();
        }
    });
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.paste {
        ctx.app.set_accels_for_action("win.paste", &[b]);
    }
}

fn do_split(
    dir: SplitDir,
    notebook: &Notebook,
    tabs: &Rc<RefCell<Vec<TabState>>>,
    focused: &Rc<RefCell<Option<Terminal>>>,
    config: &crate::config::Config,
    regexes: &MatchRegexes,
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
                super::tabs::handle_leaf_exit(
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
        refresh();
    }
}

fn register_split_horizontal(ctx: &AppCtx) {
    let act = SimpleAction::new("split-horizontal", None);
    {
        let nb = ctx.notebook.clone();
        let tabs_a = ctx.tabs.clone();
        let focused_a = ctx.focused.clone();
        let cfg = ctx.config.clone();
        let rx = ctx.regexes.clone();
        let refresh_a = ctx.refresh_close_pane_button.clone();
        act.connect_activate(move |_, _| {
            do_split(
                SplitDir::Horizontal,
                &nb,
                &tabs_a,
                &focused_a,
                &cfg,
                &rx,
                &refresh_a,
            );
        });
    }
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.split_horizontal {
        ctx.app.set_accels_for_action("win.split-horizontal", &[b]);
    }
    // Header button: same handler, no accel.
    let nb2 = ctx.notebook.clone();
    let tabs2 = ctx.tabs.clone();
    let focused2 = ctx.focused.clone();
    let cfg2 = ctx.config.clone();
    let rx2 = ctx.regexes.clone();
    let refresh_b = ctx.refresh_close_pane_button.clone();
    let btn: Button = ctx.split_h_button.clone();
    btn.connect_clicked(move |_| {
        do_split(
            SplitDir::Horizontal,
            &nb2,
            &tabs2,
            &focused2,
            &cfg2,
            &rx2,
            &refresh_b,
        );
    });
}

fn register_split_vertical(ctx: &AppCtx) {
    let act = SimpleAction::new("split-vertical", None);
    {
        let nb = ctx.notebook.clone();
        let tabs_a = ctx.tabs.clone();
        let focused_a = ctx.focused.clone();
        let cfg = ctx.config.clone();
        let rx = ctx.regexes.clone();
        let refresh_a = ctx.refresh_close_pane_button.clone();
        act.connect_activate(move |_, _| {
            do_split(
                SplitDir::Vertical,
                &nb,
                &tabs_a,
                &focused_a,
                &cfg,
                &rx,
                &refresh_a,
            );
        });
    }
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.split_vertical {
        ctx.app.set_accels_for_action("win.split-vertical", &[b]);
    }
    let nb2 = ctx.notebook.clone();
    let tabs2 = ctx.tabs.clone();
    let focused2 = ctx.focused.clone();
    let cfg2 = ctx.config.clone();
    let rx2 = ctx.regexes.clone();
    let refresh_b = ctx.refresh_close_pane_button.clone();
    let btn: Button = ctx.split_v_button.clone();
    btn.connect_clicked(move |_| {
        do_split(
            SplitDir::Vertical,
            &nb2,
            &tabs2,
            &focused2,
            &cfg2,
            &rx2,
            &refresh_b,
        );
    });
}

fn do_close_pane(
    window: &gtk4::ApplicationWindow,
    notebook: &Notebook,
    tabs: &Rc<RefCell<Vec<TabState>>>,
    focused: &Rc<RefCell<Option<Terminal>>>,
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
        let dialog = gtk4::MessageDialog::builder()
            .transient_for(window)
            .modal(true)
            .message_type(gtk4::MessageType::Warning)
            .buttons(gtk4::ButtonsType::None)
            .text(crate::strings::DIALOG_CLOSE_TAB_TITLE)
            .secondary_text(&crate::strings::format_close_tab_message(&program_name))
            .build();
        dialog.add_button(crate::strings::BUTTON_CANCEL, gtk4::ResponseType::Cancel);
        dialog.add_button(crate::strings::BUTTON_CLOSE, gtk4::ResponseType::Accept);
        dialog.set_default_response(gtk4::ResponseType::Cancel);
        let nb_d = notebook.clone();
        let tabs_d = tabs.clone();
        let container_d = container.clone();
        let pane_d = pane.clone();
        let focused_d = focused.clone();
        let refresh_d = refresh.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                do_close_pane_inner(
                    &nb_d,
                    &tabs_d,
                    &container_d,
                    &pane_d,
                    &focused_d,
                    &refresh_d,
                );
            }
            dialog.close();
        });
        dialog.show();
    } else {
        do_close_pane_inner(notebook, tabs, &container, &pane, focused, refresh);
    }
}

fn register_close_pane(ctx: &AppCtx) {
    let act = SimpleAction::new("close-pane", None);
    {
        let win = ctx.window.clone();
        let nb = ctx.notebook.clone();
        let tabs_a = ctx.tabs.clone();
        let focused_a = ctx.focused.clone();
        let refresh_a = ctx.refresh_close_pane_button.clone();
        act.connect_activate(move |_, _| {
            do_close_pane(&win, &nb, &tabs_a, &focused_a, &refresh_a);
        });
    }
    ctx.window.add_action(&act);
    if let Some(b) = &ctx.bindings.close_pane {
        ctx.app.set_accels_for_action("win.close-pane", &[b]);
    }
    let win2 = ctx.window.clone();
    let nb2 = ctx.notebook.clone();
    let tabs2 = ctx.tabs.clone();
    let focused2 = ctx.focused.clone();
    let refresh_b = ctx.refresh_close_pane_button.clone();
    let btn: Button = ctx.close_pane_button.clone();
    btn.connect_clicked(move |_| {
        do_close_pane(&win2, &nb2, &tabs2, &focused2, &refresh_b);
    });
}

fn register_focus_dir(ctx: &AppCtx, name: &'static str, axis: FocusAxis, accel: Option<&String>) {
    let nb = ctx.notebook.clone();
    let tabs_a = ctx.tabs.clone();
    let focused_a = ctx.focused.clone();
    let act = SimpleAction::new(name, None);
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
        if let Some(target) = super::focus::focus_direction(&pane, &focus, axis) {
            deferred_grab_focus(&target);
            *focused_a.borrow_mut() = Some(target);
        }
    });
    ctx.window.add_action(&act);
    let action_name = format!("win.{}", name);
    if let Some(b) = accel {
        ctx.app.set_accels_for_action(&action_name, &[b]);
    }
}
