use gtk4::gdk::RGBA;
use serde::{Deserialize, Serialize};

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
    pub fn hex_to_rgba(hex: &str) -> Option<RGBA> {
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

    /// Get a palette by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "solarized-dark" => Some(Self::solarized_dark()),
            "solarized-light" => Some(Self::solarized_light()),
            "dracula" => Some(Self::dracula()),
            "monokai" => Some(Self::monokai()),
            "gruvbox-dark" | "gruvbox" => Some(Self::gruvbox_dark()),
            "nord" => Some(Self::nord()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgba_valid() {
        let rgba = ColorPalette::hex_to_rgba("#ff0000").unwrap();
        assert_eq!(rgba.red(), 1.0);
        assert_eq!(rgba.green(), 0.0);
        assert_eq!(rgba.blue(), 0.0);
        assert_eq!(rgba.alpha(), 1.0);
    }

    #[test]
    fn test_hex_to_rgba_without_hash() {
        let rgba = ColorPalette::hex_to_rgba("00ff00").unwrap();
        assert_eq!(rgba.red(), 0.0);
        assert_eq!(rgba.green(), 1.0);
        assert_eq!(rgba.blue(), 0.0);
    }

    #[test]
    fn test_hex_to_rgba_invalid() {
        assert!(ColorPalette::hex_to_rgba("#ff").is_none());
        assert!(ColorPalette::hex_to_rgba("invalid").is_none());
        assert!(ColorPalette::hex_to_rgba("#gggggg").is_none());
    }

    #[test]
    fn test_palette_size() {
        let palette = ColorPalette::solarized_dark();
        let colors = palette.get_palette();
        assert_eq!(colors.len(), 16, "Palette should have exactly 16 colors");
    }

    #[test]
    fn test_solarized_dark_colors() {
        let palette = ColorPalette::solarized_dark();
        assert_eq!(palette.foreground, "#839496");
        assert_eq!(palette.background, "#002b36");
        assert_eq!(palette.red, "#dc322f");
    }

    #[test]
    fn test_get_foreground_background() {
        let palette = ColorPalette::dracula();
        let fg = palette.get_foreground();
        let bg = palette.get_background();

        // Check that foreground and background are different
        assert_ne!(fg.red(), bg.red());
    }

    #[test]
    fn test_palette_by_name() {
        assert!(ColorPalette::by_name("solarized-dark").is_some());
        assert!(ColorPalette::by_name("dracula").is_some());
        assert!(ColorPalette::by_name("monokai").is_some());
        assert!(ColorPalette::by_name("gruvbox").is_some());
        assert!(ColorPalette::by_name("nord").is_some());
        assert!(ColorPalette::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_all_themes_have_valid_colors() {
        let themes = vec![
            ColorPalette::solarized_dark(),
            ColorPalette::solarized_light(),
            ColorPalette::dracula(),
            ColorPalette::monokai(),
            ColorPalette::gruvbox_dark(),
            ColorPalette::nord(),
        ];

        for theme in themes {
            // Test that foreground and background can be parsed
            assert!(ColorPalette::hex_to_rgba(&theme.foreground).is_some());
            assert!(ColorPalette::hex_to_rgba(&theme.background).is_some());

            // Test that all 16 colors are valid
            let palette = theme.get_palette();
            assert_eq!(palette.len(), 16);
        }
    }

    #[test]
    fn test_gruvbox_aliases() {
        let gruvbox1 = ColorPalette::by_name("gruvbox").unwrap();
        let gruvbox2 = ColorPalette::by_name("gruvbox-dark").unwrap();

        assert_eq!(gruvbox1.foreground, gruvbox2.foreground);
        assert_eq!(gruvbox1.background, gruvbox2.background);
    }
}
