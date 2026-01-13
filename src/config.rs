use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Keyboard shortcut to create a new tab
    pub new_tab: Option<String>,
    /// Keyboard shortcut to switch to previous tab
    pub prev_tab: Option<String>,
    /// Keyboard shortcut to switch to next tab
    pub next_tab: Option<String>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            new_tab: Some("<Ctrl><Shift>T".to_string()),
            prev_tab: Some("<Ctrl>Left".to_string()),
            next_tab: Some("<Ctrl>Right".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// The shell command to execute (e.g., "/bin/bash", "/usr/bin/zsh")
    pub shell: Option<String>,
    /// Number of scrollback lines to keep in history
    pub scrollback_lines: Option<i64>,
    /// Enable cursor blinking
    pub cursor_blink: Option<bool>,
    /// Keyboard shortcuts configuration
    pub bindings: Option<KeyBindings>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: None,
            scrollback_lines: Some(10000),
            cursor_blink: Some(true),
            bindings: Some(KeyBindings::default()),
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
    pub fn load() -> Self {
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
    pub fn get_shell(&self) -> String {
        self.shell
            .clone()
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/bash".to_string())
    }

    /// Get keyboard bindings (from config or defaults)
    pub fn get_bindings(&self) -> KeyBindings {
        self.bindings.clone().unwrap_or_default()
    }
}
