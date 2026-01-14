use gtk4::Label;
use gtk4::gio::SimpleAction;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box, Button, ButtonsType, MessageDialog, MessageType, Notebook,
    Orientation, ResponseType, ScrolledWindow, SearchBar, SearchEntry,
};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use vte4::Terminal;
use vte4::prelude::*;

mod config;
mod palette;

use config::Config;

const APP_ID: &str = "com.nterm.Terminal";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run();
}

/// Create a new terminal tab with the given configuration
fn create_terminal_tab(config: &Config) -> (Box, Terminal, SearchBar) {
    // Create the VTE terminal widget
    let terminal = Terminal::new();

    // Configure the terminal behavior from config
    let cursor_mode = if config.cursor_blink.unwrap_or(true) {
        vte4::CursorBlinkMode::On
    } else {
        vte4::CursorBlinkMode::Off
    };
    terminal.set_cursor_blink_mode(cursor_mode);
    terminal.set_scroll_on_output(true);
    terminal.set_scroll_on_keystroke(true);
    terminal.set_scrollback_lines(config.scrollback_lines.unwrap_or(10000));

    // Apply color palette from config
    let palette = config.get_color_palette();
    let colors = palette.get_palette();
    let color_refs: Vec<&gtk4::gdk::RGBA> = colors.iter().collect();
    terminal.set_colors(
        Some(&palette.get_foreground()),
        Some(&palette.get_background()),
        &color_refs,
    );

    // Launch the shell from config
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
                eprintln!("Error launching shell: {}", e);
            }
        },
    );

    // Create a scrolled window with scrollbar
    let scrolled_window = ScrolledWindow::builder()
        .child(&terminal)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Always)
        .build();

    // Create search bar and entry
    let search_entry = SearchEntry::new();

    // Create previous and next buttons
    let prev_button = Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Previous result (Shift+Enter)")
        .build();

    let next_button = Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text("Next result (Enter)")
        .build();

    // Create a horizontal box for search entry and buttons
    let search_box = Box::new(Orientation::Horizontal, 6);
    search_box.append(&search_entry);
    search_box.append(&prev_button);
    search_box.append(&next_button);

    let search_bar = SearchBar::builder()
        .child(&search_box)
        .show_close_button(true)
        .build();

    // Setup search functionality
    let terminal_for_search = terminal.clone();
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text();
        if !text.is_empty() {
            // Create regex for search with MULTILINE flag (0x08000000 in PCRE2)
            // VTE requires MULTILINE flag for terminal search
            if let Ok(regex) = vte4::Regex::for_search(&text, 0x08000000) {
                terminal_for_search.search_set_regex(Some(&regex), 0);
                terminal_for_search.search_find_next();
            }
        }
    });

    // Handle Enter key to find next
    let terminal_for_next = terminal.clone();
    search_entry.connect_activate(move |_| {
        terminal_for_next.search_find_next();
    });

    // Handle Shift+Enter to find previous using key-press event
    let terminal_for_prev = terminal.clone();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, modifiers| {
        // Check for Shift+Enter (Return key with Shift modifier)
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

    // Connect button clicks
    let terminal_for_prev_btn = terminal.clone();
    prev_button.connect_clicked(move |_| {
        terminal_for_prev_btn.search_find_previous();
    });

    let terminal_for_next_btn = terminal.clone();
    next_button.connect_clicked(move |_| {
        terminal_for_next_btn.search_find_next();
    });

    // Connect search bar to search entry
    search_bar.connect_entry(&search_entry);

    // Create a vertical box to hold search bar and terminal
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&search_bar);
    vbox.append(&scrolled_window);

    // Make the scrolled window expand to fill vertical space
    scrolled_window.set_vexpand(true);
    scrolled_window.set_hexpand(true);

    (vbox, terminal, search_bar)
}

/// Check if a terminal has a foreground process running (not just the shell)
/// Returns Some(program_name) if a non-shell program is running, None otherwise
fn has_foreground_process(terminal: &Terminal) -> Option<String> {
    if let Some(pty) = terminal.pty() {
        let fd = pty.fd().as_raw_fd();

        // Get the foreground process group ID
        // SAFETY: fd is a valid file descriptor from the PTY
        let fg_pgid = unsafe { libc::tcgetpgrp(fd) };

        if fg_pgid > 0 {
            // Read /proc/<pgid>/stat to get the process name
            let stat_path = format!("/proc/{}/stat", fg_pgid);
            if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
                // Parse the stat file to get the command name (between parentheses)
                if let Some(start) = stat_content.find('(') {
                    if let Some(end) = stat_content.rfind(')') {
                        let cmd_name = &stat_content[start + 1..end];

                        // Check if it's a shell
                        let is_shell = matches!(
                            cmd_name,
                            "bash" | "zsh" | "sh" | "fish" | "dash" | "ksh" | "csh" | "tcsh"
                        );

                        // If it's not a shell, return the program name
                        if !is_shell {
                            return Some(cmd_name.to_string());
                        }
                    }
                }
            }
        }
    }

    // If we can't determine or it's a shell, return None
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
        // Check if there's a foreground process running
        if let Some(program_name) = has_foreground_process(&terminal_clone) {
            // Show confirmation dialog
            let dialog = MessageDialog::builder()
                .transient_for(&window_clone)
                .modal(true)
                .message_type(MessageType::Warning)
                .buttons(ButtonsType::None)
                .text("Close tab?")
                .secondary_text(&format!(
                    "The program '{}' is running in this tab. Do you really want to close it?",
                    program_name
                ))
                .build();

            dialog.add_button("Cancel", ResponseType::Cancel);
            dialog.add_button("Close", ResponseType::Accept);
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
            // No foreground process, close immediately
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
                    // Hide tabs if only one tab remains
                    notebook.set_show_tabs(notebook.n_pages() > 1);

                    // Give focus to the current tab's terminal
                    if let Some(current_page) = notebook.current_page() {
                        if let Some(page) = notebook.nth_page(Some(current_page)) {
                            if let Some(vbox) = page.downcast_ref::<Box>() {
                                let mut child = vbox.first_child();
                                while let Some(widget) = child {
                                    if let Some(scrolled) = widget.downcast_ref::<ScrolledWindow>()
                                    {
                                        if let Some(terminal_child) = scrolled.child() {
                                            if let Some(terminal) =
                                                terminal_child.downcast_ref::<Terminal>()
                                            {
                                                terminal.grab_focus();
                                                break;
                                            }
                                        }
                                    }
                                    child = widget.next_sibling();
                                }
                            }
                        }
                    }
                }
                return;
            }
        }
    }
}

/// Set up dynamic tab title updates based on terminal window title
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
            // Find the page number for this terminal
            let n_pages = notebook_clone.n_pages();
            for i in 0..n_pages {
                if let Some(page) = notebook_clone.nth_page(Some(i)) {
                    if page == page_widget_clone {
                        // Update the tab label text (find the Label in the tab_box)
                        if let Some(tab_label_widget) = notebook_clone.tab_label(&page_widget_clone)
                        {
                            if let Some(tab_box) = tab_label_widget.downcast_ref::<Box>() {
                                // Find the Label in the tab_box and update its text
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

                        // Update window title if this is the current tab
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

fn build_ui(app: &Application) {
    // Load configuration
    let config = Rc::new(Config::load());
    let bindings = config.get_bindings();

    // Create a notebook (tabbed interface)
    let notebook = Notebook::builder()
        .scrollable(true)
        .show_tabs(false) // Hide tabs when there's only one tab
        .build();

    // Configure tabs to expand and fill available width
    notebook.set_tab_pos(gtk4::PositionType::Top);

    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("NTerm - Terminal Emulator")
        .default_width(800)
        .default_height(600)
        .child(&notebook)
        .build();

    // Create the first terminal tab
    let (vbox, terminal, _search_bar) = create_terminal_tab(&config);

    // Create tab label with close button
    let tab_box = Box::new(Orientation::Horizontal, 6);
    let tab_label = Label::new(Some("Terminal"));
    tab_label.set_hexpand(true);
    tab_label.set_xalign(0.5);
    let close_button = create_close_button_with_confirmation(&notebook, &vbox, &terminal, &window);

    tab_box.append(&tab_label);
    tab_box.append(&close_button);

    notebook.append_page(&vbox, Some(&tab_box));
    notebook.set_tab_reorderable(&vbox, true);
    notebook.set_tab_detachable(&vbox, false);
    notebook.page(&vbox).set_property("tab-expand", true);

    // Set up dynamic tab title updates
    setup_tab_title_update(&terminal, &notebook, &vbox, &window);

    // Give focus to the terminal
    terminal.grab_focus();

    // Handle tab switching - give focus to the active terminal and update window title
    let window_for_switch = window.clone();
    notebook.connect_switch_page(move |_notebook, page, _page_num| {
        if let Some(vbox) = page.downcast_ref::<Box>() {
            // Find the ScrolledWindow in the Box children
            let mut child = vbox.first_child();
            while let Some(widget) = child {
                if let Some(scrolled) = widget.downcast_ref::<ScrolledWindow>() {
                    if let Some(terminal_child) = scrolled.child() {
                        if let Some(terminal) = terminal_child.downcast_ref::<Terminal>() {
                            terminal.grab_focus();
                            // Update window title with the current terminal's title
                            if let Some(title) = terminal.window_title() {
                                window_for_switch.set_title(Some(&format!("NTerm - {}", title)));
                            }
                            break;
                        }
                    }
                }
                child = widget.next_sibling();
            }
        }
    });

    // Handle tab closure when the first tab's process exits
    let notebook_clone = notebook.clone();
    let vbox_clone = vbox.clone();
    terminal.connect_child_exited(move |_terminal, _| {
        // Find and remove the tab containing this terminal
        let n_pages = notebook_clone.n_pages();
        for i in 0..n_pages {
            if let Some(page) = notebook_clone.nth_page(Some(i)) {
                if page == vbox_clone {
                    notebook_clone.remove_page(Some(i));
                    if notebook_clone.n_pages() == 0 {
                        std::process::exit(0);
                    } else {
                        // Hide tabs if only one tab remains
                        notebook_clone.set_show_tabs(notebook_clone.n_pages() > 1);

                        // Give focus to the current tab's terminal
                        if let Some(current_page) = notebook_clone.current_page() {
                            if let Some(page) = notebook_clone.nth_page(Some(current_page)) {
                                if let Some(vbox) = page.downcast_ref::<Box>() {
                                    let mut child = vbox.first_child();
                                    while let Some(widget) = child {
                                        if let Some(scrolled) =
                                            widget.downcast_ref::<ScrolledWindow>()
                                        {
                                            if let Some(terminal_child) = scrolled.child() {
                                                if let Some(terminal) =
                                                    terminal_child.downcast_ref::<Terminal>()
                                                {
                                                    terminal.grab_focus();
                                                    break;
                                                }
                                            }
                                        }
                                        child = widget.next_sibling();
                                    }
                                }
                            }
                        }
                    }
                    return;
                }
            }
        }
    });

    // Set up keyboard shortcuts using actions
    let notebook_for_action = notebook.clone();
    let config_for_action = config.clone();
    let window_for_action = window.clone();

    let new_tab_action = SimpleAction::new("new-tab", None);
    new_tab_action.connect_activate(move |_, _| {
        // Create a new terminal tab
        let (vbox, terminal, _search_bar) = create_terminal_tab(&config_for_action);

        // Create tab label with close button
        let tab_box = Box::new(Orientation::Horizontal, 6);
        let label = Label::new(Some("Terminal"));
        label.set_hexpand(true);
        label.set_xalign(0.5);
        let close_button = create_close_button_with_confirmation(
            &notebook_for_action,
            &vbox,
            &terminal,
            &window_for_action,
        );

        tab_box.append(&label);
        tab_box.append(&close_button);

        let page_num = notebook_for_action.append_page(&vbox, Some(&tab_box));
        notebook_for_action.set_tab_reorderable(&vbox, true);
        notebook_for_action.set_tab_detachable(&vbox, false);
        notebook_for_action
            .page(&vbox)
            .set_property("tab-expand", true);

        notebook_for_action.set_current_page(Some(page_num));

        // Show tabs if we now have more than one
        notebook_for_action.set_show_tabs(notebook_for_action.n_pages() > 1);

        // Set up dynamic tab title updates
        setup_tab_title_update(&terminal, &notebook_for_action, &vbox, &window_for_action);

        // Give focus to the terminal
        terminal.grab_focus();

        // Handle tab closure when process exits
        let notebook_clone = notebook_for_action.clone();
        terminal.connect_child_exited(move |terminal, _| {
            // Find and remove the tab containing this terminal
            let n_pages = notebook_clone.n_pages();
            for i in 0..n_pages {
                if let Some(page) = notebook_clone.nth_page(Some(i)) {
                    if let Some(vbox) = page.downcast_ref::<Box>() {
                        let mut child = vbox.first_child();
                        let mut found = false;
                        while let Some(widget) = child {
                            if let Some(scrolled) = widget.downcast_ref::<ScrolledWindow>() {
                                if let Some(terminal_child) = scrolled.child() {
                                    if terminal_child.downcast_ref::<Terminal>() == Some(terminal) {
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            child = widget.next_sibling();
                        }
                        if found {
                            notebook_clone.remove_page(Some(i));
                            if notebook_clone.n_pages() == 0 {
                                std::process::exit(0);
                            } else {
                                // Hide tabs if only one tab remains
                                notebook_clone.set_show_tabs(notebook_clone.n_pages() > 1);

                                // Give focus to the current tab's terminal
                                if let Some(current_page) = notebook_clone.current_page() {
                                    if let Some(page) = notebook_clone.nth_page(Some(current_page))
                                    {
                                        if let Some(vbox) = page.downcast_ref::<Box>() {
                                            let mut child = vbox.first_child();
                                            while let Some(widget) = child {
                                                if let Some(scrolled) =
                                                    widget.downcast_ref::<ScrolledWindow>()
                                                {
                                                    if let Some(terminal_child) = scrolled.child() {
                                                        if let Some(terminal) = terminal_child
                                                            .downcast_ref::<Terminal>(
                                                        ) {
                                                            terminal.grab_focus();
                                                            break;
                                                        }
                                                    }
                                                }
                                                child = widget.next_sibling();
                                            }
                                        }
                                    }
                                }
                            }
                            return;
                        }
                    }
                }
            }
        });
    });

    window.add_action(&new_tab_action);
    if let Some(new_tab_binding) = bindings.new_tab {
        app.set_accels_for_action("win.new-tab", &[&new_tab_binding]);
    }

    // Action for previous tab (Ctrl+Alt+Left)
    let notebook_for_prev = notebook.clone();
    let prev_tab_action = SimpleAction::new("prev-tab", None);
    prev_tab_action.connect_activate(move |_, _| {
        let n_pages = notebook_for_prev.n_pages();
        if n_pages > 1 {
            if let Some(current) = notebook_for_prev.current_page() {
                let prev_page = if current == 0 {
                    n_pages - 1
                } else {
                    current - 1
                };
                notebook_for_prev.set_current_page(Some(prev_page));
            }
        }
    });
    window.add_action(&prev_tab_action);
    if let Some(prev_tab_binding) = bindings.prev_tab {
        app.set_accels_for_action("win.prev-tab", &[&prev_tab_binding]);
    }

    // Action for next tab (Ctrl+Alt+Right)
    let notebook_for_next = notebook.clone();
    let next_tab_action = SimpleAction::new("next-tab", None);
    next_tab_action.connect_activate(move |_, _| {
        let n_pages = notebook_for_next.n_pages();
        if n_pages > 1 {
            if let Some(current) = notebook_for_next.current_page() {
                let next_page = if current >= n_pages - 1 {
                    0
                } else {
                    current + 1
                };
                notebook_for_next.set_current_page(Some(next_page));
            }
        }
    });
    window.add_action(&next_tab_action);
    if let Some(next_tab_binding) = bindings.next_tab {
        app.set_accels_for_action("win.next-tab", &[&next_tab_binding]);
    }

    // Action for toggling search in current tab
    let notebook_for_search = notebook.clone();
    let search_action = SimpleAction::new("toggle-search", None);
    search_action.connect_activate(move |_, _| {
        if let Some(current_page) = notebook_for_search.current_page() {
            if let Some(page) = notebook_for_search.nth_page(Some(current_page)) {
                if let Some(vbox) = page.downcast_ref::<Box>() {
                    // Find the SearchBar in the Box children
                    let mut child = vbox.first_child();
                    while let Some(widget) = child {
                        if let Some(search_bar) = widget.downcast_ref::<SearchBar>() {
                            search_bar.set_search_mode(!search_bar.is_search_mode());
                            break;
                        }
                        child = widget.next_sibling();
                    }
                }
            }
        }
    });
    window.add_action(&search_action);
    if let Some(search_binding) = bindings.search {
        app.set_accels_for_action("win.toggle-search", &[&search_binding]);
    }

    // Display the window
    window.present();
}
