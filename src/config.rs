use gtk4::gdk::RGBA;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorPalette {
    pub foreground: String,
    pub background: String,
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl ColorPalette {
    /// Convert a hex color string to RGBA
    fn hex_to_rgba(hex: &str) -> Option<RGBA> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;

        Some(RGBA::new(r, g, b, 1.0))
    }

    /// Get the 16-color palette as RGBA array
    pub fn get_palette(&self) -> Vec<RGBA> {
        vec![
            Self::hex_to_rgba(&self.black).unwrap_or(RGBA::BLACK),
            Self::hex_to_rgba(&self.red).unwrap_or(RGBA::new(0.8, 0.0, 0.0, 1.0)),
            Self::hex_to_rgba(&self.green).unwrap_or(RGBA::new(0.0, 0.8, 0.0, 1.0)),
            Self::hex_to_rgba(&self.yellow).unwrap_or(RGBA::new(0.8, 0.8, 0.0, 1.0)),
            Self::hex_to_rgba(&self.blue).unwrap_or(RGBA::new(0.0, 0.0, 0.8, 1.0)),
            Self::hex_to_rgba(&self.magenta).unwrap_or(RGBA::new(0.8, 0.0, 0.8, 1.0)),
            Self::hex_to_rgba(&self.cyan).unwrap_or(RGBA::new(0.0, 0.8, 0.8, 1.0)),
            Self::hex_to_rgba(&self.white).unwrap_or(RGBA::new(0.8, 0.8, 0.8, 1.0)),
            Self::hex_to_rgba(&self.bright_black).unwrap_or(RGBA::new(0.4, 0.4, 0.4, 1.0)),
            Self::hex_to_rgba(&self.bright_red).unwrap_or(RGBA::new(1.0, 0.0, 0.0, 1.0)),
            Self::hex_to_rgba(&self.bright_green).unwrap_or(RGBA::new(0.0, 1.0, 0.0, 1.0)),
            Self::hex_to_rgba(&self.bright_yellow).unwrap_or(RGBA::new(1.0, 1.0, 0.0, 1.0)),
            Self::hex_to_rgba(&self.bright_blue).unwrap_or(RGBA::new(0.0, 0.0, 1.0, 1.0)),
            Self::hex_to_rgba(&self.bright_magenta).unwrap_or(RGBA::new(1.0, 0.0, 1.0, 1.0)),
            Self::hex_to_rgba(&self.bright_cyan).unwrap_or(RGBA::new(0.0, 1.0, 1.0, 1.0)),
            Self::hex_to_rgba(&self.bright_white).unwrap_or(RGBA::WHITE),
        ]
    }

    /// Get foreground color as RGBA
    pub fn get_foreground(&self) -> RGBA {
        Self::hex_to_rgba(&self.foreground).unwrap_or(RGBA::WHITE)
    }

    /// Get background color as RGBA
    pub fn get_background(&self) -> RGBA {
        Self::hex_to_rgba(&self.background).unwrap_or(RGBA::BLACK)
    }

    /// Solarized Dark theme
    pub fn solarized_dark() -> Self {
        Self {
            foreground: "#839496".to_string(),
            background: "#002b36".to_string(),
            black: "#073642".to_string(),
            red: "#dc322f".to_string(),
            green: "#859900".to_string(),
            yellow: "#b58900".to_string(),
            blue: "#268bd2".to_string(),
            magenta: "#d33682".to_string(),
            cyan: "#2aa198".to_string(),
            white: "#eee8d5".to_string(),
            bright_black: "#002b36".to_string(),
            bright_red: "#cb4b16".to_string(),
            bright_green: "#586e75".to_string(),
            bright_yellow: "#657b83".to_string(),
            bright_blue: "#839496".to_string(),
            bright_magenta: "#6c71c4".to_string(),
            bright_cyan: "#93a1a1".to_string(),
            bright_white: "#fdf6e3".to_string(),
        }
    }

    /// Solarized Light theme
    pub fn solarized_light() -> Self {
        Self {
            foreground: "#657b83".to_string(),
            background: "#fdf6e3".to_string(),
            black: "#073642".to_string(),
            red: "#dc322f".to_string(),
            green: "#859900".to_string(),
            yellow: "#b58900".to_string(),
            blue: "#268bd2".to_string(),
            magenta: "#d33682".to_string(),
            cyan: "#2aa198".to_string(),
            white: "#eee8d5".to_string(),
            bright_black: "#002b36".to_string(),
            bright_red: "#cb4b16".to_string(),
            bright_green: "#586e75".to_string(),
            bright_yellow: "#657b83".to_string(),
            bright_blue: "#839496".to_string(),
            bright_magenta: "#6c71c4".to_string(),
            bright_cyan: "#93a1a1".to_string(),
            bright_white: "#fdf6e3".to_string(),
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            foreground: "#f8f8f2".to_string(),
            background: "#282a36".to_string(),
            black: "#000000".to_string(),
            red: "#ff5555".to_string(),
            green: "#50fa7b".to_string(),
            yellow: "#f1fa8c".to_string(),
            blue: "#bd93f9".to_string(),
            magenta: "#ff79c6".to_string(),
            cyan: "#8be9fd".to_string(),
            white: "#bfbfbf".to_string(),
            bright_black: "#4d4d4d".to_string(),
            bright_red: "#ff6e67".to_string(),
            bright_green: "#5af78e".to_string(),
            bright_yellow: "#f4f99d".to_string(),
            bright_blue: "#caa9fa".to_string(),
            bright_magenta: "#ff92d0".to_string(),
            bright_cyan: "#9aedfe".to_string(),
            bright_white: "#e6e6e6".to_string(),
        }
    }

    /// Monokai theme
    pub fn monokai() -> Self {
        Self {
            foreground: "#f8f8f2".to_string(),
            background: "#272822".to_string(),
            black: "#272822".to_string(),
            red: "#f92672".to_string(),
            green: "#a6e22e".to_string(),
            yellow: "#f4bf75".to_string(),
            blue: "#66d9ef".to_string(),
            magenta: "#ae81ff".to_string(),
            cyan: "#a1efe4".to_string(),
            white: "#f8f8f2".to_string(),
            bright_black: "#75715e".to_string(),
            bright_red: "#f92672".to_string(),
            bright_green: "#a6e22e".to_string(),
            bright_yellow: "#f4bf75".to_string(),
            bright_blue: "#66d9ef".to_string(),
            bright_magenta: "#ae81ff".to_string(),
            bright_cyan: "#a1efe4".to_string(),
            bright_white: "#f9f8f5".to_string(),
        }
    }

    /// Gruvbox Dark theme
    pub fn gruvbox_dark() -> Self {
        Self {
            foreground: "#ebdbb2".to_string(),
            background: "#282828".to_string(),
            black: "#282828".to_string(),
            red: "#cc241d".to_string(),
            green: "#98971a".to_string(),
            yellow: "#d79921".to_string(),
            blue: "#458588".to_string(),
            magenta: "#b16286".to_string(),
            cyan: "#689d6a".to_string(),
            white: "#a89984".to_string(),
            bright_black: "#928374".to_string(),
            bright_red: "#fb4934".to_string(),
            bright_green: "#b8bb26".to_string(),
            bright_yellow: "#fabd2f".to_string(),
            bright_blue: "#83a598".to_string(),
            bright_magenta: "#d3869b".to_string(),
            bright_cyan: "#8ec07c".to_string(),
            bright_white: "#ebdbb2".to_string(),
        }
    }

    /// Nord theme
    pub fn nord() -> Self {
        Self {
            foreground: "#d8dee9".to_string(),
            background: "#2e3440".to_string(),
            black: "#3b4252".to_string(),
            red: "#bf616a".to_string(),
            green: "#a3be8c".to_string(),
            yellow: "#ebcb8b".to_string(),
            blue: "#81a1c1".to_string(),
            magenta: "#b48ead".to_string(),
            cyan: "#88c0d0".to_string(),
            white: "#e5e9f0".to_string(),
            bright_black: "#4c566a".to_string(),
            bright_red: "#bf616a".to_string(),
            bright_green: "#a3be8c".to_string(),
            bright_yellow: "#ebcb8b".to_string(),
            bright_blue: "#81a1c1".to_string(),
            bright_magenta: "#b48ead".to_string(),
            bright_cyan: "#8fbcbb".to_string(),
            bright_white: "#eceff4".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    /// Keyboard shortcut to create a new tab
    pub new_tab: Option<String>,
    /// Keyboard shortcut to switch to previous tab
    pub prev_tab: Option<String>,
    /// Keyboard shortcut to switch to next tab
    pub next_tab: Option<String>,
    /// Keyboard shortcut to toggle search in current terminal
    pub search: Option<String>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            new_tab: Some("<Ctrl><Shift>T".to_string()),
            prev_tab: Some("<Ctrl>Left".to_string()),
            next_tab: Some("<Ctrl>Right".to_string()),
            search: Some("<Ctrl><Shift>F".to_string()),
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
    /// Color profile name (e.g., "solarized-dark", "dracula", "monokai")
    pub color_profile: Option<String>,
    /// Custom color palette (overrides color_profile if both are set)
    pub colors: Option<ColorPalette>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: None,
            scrollback_lines: Some(10000),
            cursor_blink: Some(true),
            bindings: Some(KeyBindings::default()),
            color_profile: None,
            colors: None,
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

    /// Get color palette (from custom colors, profile, or default)
    pub fn get_color_palette(&self) -> ColorPalette {
        // If custom colors are defined, use them
        if let Some(colors) = &self.colors {
            return colors.clone();
        }

        // Otherwise use the color profile
        match self.color_profile.as_deref() {
            Some("solarized-dark") => ColorPalette::solarized_dark(),
            Some("solarized-light") => ColorPalette::solarized_light(),
            Some("dracula") => ColorPalette::dracula(),
            Some("monokai") => ColorPalette::monokai(),
            Some("gruvbox-dark") | Some("gruvbox") => ColorPalette::gruvbox_dark(),
            Some("nord") => ColorPalette::nord(),
            _ => ColorPalette::solarized_dark(), // Default to Solarized Dark
        }
    }
}
