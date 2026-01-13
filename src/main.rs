use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, ScrolledWindow};
use vte4::prelude::*;
use vte4::Terminal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        let config_file = Self::config_file()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory"
            ))?;

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
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    // Load configuration
    let config = Config::load();
    
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
    
    // Handle window closure when the child process exits
    terminal.connect_child_exited(|_, _| {
        std::process::exit(0);
    });
    
    // Create a scrolled window with scrollbar
    let scrolled_window = ScrolledWindow::builder()
        .child(&terminal)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Always)
        .build();
    
    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("NTerm - Terminal Emulator")
        .default_width(800)
        .default_height(600)
        .child(&scrolled_window)
        .build();
    
    // Display the window
    window.present();
}
