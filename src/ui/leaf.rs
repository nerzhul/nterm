use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box, Button, DrawingArea, Orientation, Overlay, ScrolledWindow, SearchBar, SearchEntry};
use vte4::Terminal;
use vte4::prelude::*;

use crate::config::Config;
use crate::explosion::ExplosionState;
use crate::strings as s;

use super::regexes::MatchRegexes;

/// Payload of a leaf in the pane tree.
pub struct Leaf {
    pub terminal: Terminal,
    #[allow(dead_code)]
    pub scrolled: ScrolledWindow,
    #[allow(dead_code)]
    pub overlay: Overlay,
    #[allow(dead_code)]
    pub drawing_area: DrawingArea,
    pub search_bar: SearchBar,
    #[allow(dead_code)]
    pub search_entry: SearchEntry,
    pub vbox: Box,
}

/// Build a new leaf with a single VTE terminal widget.
pub fn build_leaf(config: &Config, regexes: &MatchRegexes) -> Leaf {
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
pub fn deferred_grab_focus(terminal: &Terminal) {
    let t = terminal.clone();
    glib::idle_add_local_once(move || {
        t.grab_focus();
    });
}
