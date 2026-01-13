use gtk4::Label;
use gtk4::gio::SimpleAction;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Notebook, ScrolledWindow};
use std::rc::Rc;
use vte4::Terminal;
use vte4::prelude::*;

mod config;
use config::Config;

const APP_ID: &str = "com.nterm.Terminal";

fn main() {
    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);
    app.run();
}

/// Create a new terminal tab with the given configuration
fn create_terminal_tab(config: &Config) -> (ScrolledWindow, Terminal) {
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

    (scrolled_window, terminal)
}

/// Set up dynamic tab title updates based on terminal window title
fn setup_tab_title_update(
    terminal: &Terminal,
    notebook: &Notebook,
    page_widget: &ScrolledWindow,
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
                        // Update the tab label
                        let label = Label::new(Some(&title));
                        notebook_clone.set_tab_label(&page_widget_clone, Some(&label));

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
    let (scrolled_window, terminal) = create_terminal_tab(&config);
    let tab_label = Label::new(Some("Terminal"));
    notebook.append_page(&scrolled_window, Some(&tab_label));
    notebook.set_tab_reorderable(&scrolled_window, true);
    notebook.set_tab_detachable(&scrolled_window, false);
    notebook
        .page(&scrolled_window)
        .set_property("tab-expand", true);

    // Set up dynamic tab title updates
    setup_tab_title_update(&terminal, &notebook, &scrolled_window, &window);

    // Give focus to the terminal
    terminal.grab_focus();

    // Handle tab switching - give focus to the active terminal and update window title
    let window_for_switch = window.clone();
    notebook.connect_switch_page(move |_notebook, page, _page_num| {
        if let Some(scrolled) = page.downcast_ref::<ScrolledWindow>() {
            if let Some(child) = scrolled.child() {
                if let Some(terminal) = child.downcast_ref::<Terminal>() {
                    terminal.grab_focus();
                    // Update window title with the current terminal's title
                    if let Some(title) = terminal.window_title() {
                        window_for_switch.set_title(Some(&format!("NTerm - {}", title)));
                    }
                }
            }
        }
    });

    // Handle tab closure when the first tab's process exits
    let notebook_clone = notebook.clone();
    let scrolled_window_clone = scrolled_window.clone();
    terminal.connect_child_exited(move |_terminal, _| {
        // Find and remove the tab containing this terminal
        let n_pages = notebook_clone.n_pages();
        for i in 0..n_pages {
            if let Some(page) = notebook_clone.nth_page(Some(i)) {
                if page == scrolled_window_clone {
                    notebook_clone.remove_page(Some(i));
                    if notebook_clone.n_pages() == 0 {
                        std::process::exit(0);
                    } else {
                        // Hide tabs if only one tab remains
                        notebook_clone.set_show_tabs(notebook_clone.n_pages() > 1);

                        // Give focus to the current tab's terminal
                        if let Some(current_page) = notebook_clone.current_page() {
                            if let Some(page) = notebook_clone.nth_page(Some(current_page)) {
                                if let Some(scrolled) = page.downcast_ref::<ScrolledWindow>() {
                                    if let Some(child) = scrolled.child() {
                                        if let Some(terminal) = child.downcast_ref::<Terminal>() {
                                            terminal.grab_focus();
                                        }
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
        let (scrolled_window, terminal) = create_terminal_tab(&config_for_action);

        let label = Label::new(Some("Terminal"));

        let page_num = notebook_for_action.append_page(&scrolled_window, Some(&label));
        notebook_for_action.set_tab_reorderable(&scrolled_window, true);
        notebook_for_action.set_tab_detachable(&scrolled_window, false);
        notebook_for_action
            .page(&scrolled_window)
            .set_property("tab-expand", true);

        notebook_for_action.set_current_page(Some(page_num));

        // Show tabs if we now have more than one
        notebook_for_action.set_show_tabs(notebook_for_action.n_pages() > 1);

        // Set up dynamic tab title updates
        setup_tab_title_update(
            &terminal,
            &notebook_for_action,
            &scrolled_window,
            &window_for_action,
        );

        // Give focus to the terminal
        terminal.grab_focus();

        // Handle tab closure when process exits
        let notebook_clone = notebook_for_action.clone();
        terminal.connect_child_exited(move |terminal, _| {
            // Find and remove the tab containing this terminal
            let n_pages = notebook_clone.n_pages();
            for i in 0..n_pages {
                if let Some(page) = notebook_clone.nth_page(Some(i)) {
                    if let Some(scrolled) = page.downcast_ref::<ScrolledWindow>() {
                        if let Some(child) = scrolled.child() {
                            if child.downcast_ref::<Terminal>() == Some(terminal) {
                                notebook_clone.remove_page(Some(i));
                                if notebook_clone.n_pages() == 0 {
                                    std::process::exit(0);
                                } else {
                                    // Hide tabs if only one tab remains
                                    notebook_clone.set_show_tabs(notebook_clone.n_pages() > 1);

                                    // Give focus to the current tab's terminal
                                    if let Some(current_page) = notebook_clone.current_page() {
                                        if let Some(page) =
                                            notebook_clone.nth_page(Some(current_page))
                                        {
                                            if let Some(scrolled) =
                                                page.downcast_ref::<ScrolledWindow>()
                                            {
                                                if let Some(child) = scrolled.child() {
                                                    if let Some(terminal) =
                                                        child.downcast_ref::<Terminal>()
                                                    {
                                                        terminal.grab_focus();
                                                    }
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
            }
        });
    });

    window.add_action(&new_tab_action);
    app.set_accels_for_action("win.new-tab", &["<Ctrl><Shift>T"]);

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
    app.set_accels_for_action("win.prev-tab", &["<Ctrl>Left"]);

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
    app.set_accels_for_action("win.next-tab", &["<Ctrl>Right"]);

    // Display the window
    window.present();
}
