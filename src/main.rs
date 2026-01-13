use gtk4::Label;
use gtk4::gio::SimpleAction;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Notebook, ScrolledWindow};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use vte4::Terminal;
use vte4::prelude::*;

const APP_ID: &str = "com.nterm.Terminal";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// The shell command to execute (e.g., "/bin/bash", "/usr/bin/zsh")
    shell: Option<String>,
    /// Number of scrollback lines to keep in history
    scrollback_lines: Option<i64>,
    /// Enable cursor blinking
    cursor_blink: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: None,
            scrollback_lines: Some(10000),
            cursor_blink: Some(true),
        }
    }
}

impl Config {
    /// Get the XDG config directory for nterm
    fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("nterm"))
    }

    /// Get the config file path
    fn config_file() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    /// Load configuration from file, or create default if it doesn't exist
    fn load() -> Self {
        let config_file = match Self::config_file() {
            Some(path) => path,
            None => {
                eprintln!("Could not determine config directory, using defaults");
                return Self::default();
            }
        };

        if config_file.exists() {
            match fs::read_to_string(&config_file) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(config) => config,
                    Err(e) => {
                        eprintln!("Failed to parse config file: {}. Using defaults.", e);
                        Self::default()
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read config file: {}. Using defaults.", e);
                    Self::default()
                }
            }
        } else {
            // Create default config file
            let config = Self::default();
            if let Err(e) = config.save() {
                eprintln!("Failed to create default config file: {}", e);
            } else {
                println!("Created default config at: {}", config_file.display());
            }
            config
        }
    }

    /// Save configuration to file
    fn save(&self) -> std::io::Result<()> {
        let config_file = Self::config_file().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            )
        })?;

        // Create config directory if it doesn't exist
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        fs::write(&config_file, contents)?;
        Ok(())
    }

    /// Get the shell to use (from config or system default)
    fn get_shell(&self) -> String {
        self.shell
            .clone()
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string())
    }
}

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
fn setup_tab_title_update(terminal: &Terminal, notebook: &Notebook, page_widget: &ScrolledWindow) {
    let notebook_clone = notebook.clone();
    let page_widget_clone = page_widget.clone();

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
    let notebook = Notebook::builder().scrollable(true).build();

    // Create the first terminal tab
    let (scrolled_window, terminal) = create_terminal_tab(&config);
    let label = Label::new(Some("Terminal"));
    notebook.append_page(&scrolled_window, Some(&label));

    // Set up dynamic tab title updates
    setup_tab_title_update(&terminal, &notebook, &scrolled_window);

    // Handle window closure when the last tab's child process exits
    let notebook_clone = notebook.clone();
    terminal.connect_child_exited(move |_, _| {
        if notebook_clone.n_pages() <= 1 {
            std::process::exit(0);
        }
    });

    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("NTerm - Terminal Emulator")
        .default_width(800)
        .default_height(600)
        .child(&notebook)
        .build();

    // Set up keyboard shortcuts using actions
    let notebook_for_action = notebook.clone();
    let config_for_action = config.clone();

    let new_tab_action = SimpleAction::new("new-tab", None);
    new_tab_action.connect_activate(move |_, _| {
        // Create a new terminal tab
        let (scrolled_window, terminal) = create_terminal_tab(&config_for_action);

        let label = Label::new(Some("Terminal"));

        let page_num = notebook_for_action.append_page(&scrolled_window, Some(&label));
        notebook_for_action.set_current_page(Some(page_num));

        // Set up dynamic tab title updates
        setup_tab_title_update(&terminal, &notebook_for_action, &scrolled_window);

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

    // Display the window
    window.present();
}
