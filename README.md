# NTerm - Terminal Emulator

A modern terminal emulator written in Rust with GTK4 and VTE4.

## Dependencies

On Linux, you need to install the GTK4 and VTE4 development libraries:

### Debian/Ubuntu
```bash
sudo apt install libgtk-4-dev libvte-2.91-gtk4-dev
```

### Fedora
```bash
sudo dnf install gtk4-devel vte291-gtk4-devel
```

### Arch Linux
```bash
sudo pacman -S gtk4 vte4
```

## Build and Run

```bash
# Build
cargo build --release

# Run
cargo run
```

Alternatively, using the Makefile:

```bash
# Build (debug)
make build

# Build (release)
make release

# Run
make run

# Install to /usr/local/bin
sudo make install

# Clean build artifacts
make clean
```

For all available targets, run `make help`.

## Features

- ✅ Full VTE terminal emulation
- ✅ Multiple tabs support with `Ctrl+Shift+T` to create new tabs
- ✅ Terminal search with regex support
- ✅ Scrollbar support
- ✅ 10,000 lines scrollback buffer
- ✅ Uses system default shell ($SHELL)
- ✅ Blinking cursor
- ✅ XDG-compliant configuration file
- ✅ Configurable keyboard shortcuts
- ✅ Color profiles support (Solarized, Dracula, Monokai, Gruvbox, Nord)
- ✅ Split panes (tmux-style) inside each tab

## Keyboard Shortcuts

### Tab Management
- `Ctrl+Shift+T` - Create a new terminal tab
- `Ctrl+Left` - Go to previous tab (wraps to last tab)
- `Ctrl+Right` - Go to next tab (wraps to first tab)

### Search
- `Ctrl+Shift+F` - Toggle search bar in current terminal
- `Enter` - Find next search result
- `Shift+Enter` - Find previous search result

### Split Panes
Each tab can contain an arbitrary binary tree of panes (similar to tmux).
- `Ctrl+Shift+E` - Split the focused pane vertically (side by side)
- `Ctrl+Shift+D` - Split the focused pane horizontally (top/bottom)
- `Ctrl+Shift+Left/Right/Up/Down` - Move focus to the adjacent pane
- `Ctrl+Shift+W` - Close the focused pane (collapses the split; if it was the
  only pane, the tab is closed)
- The divider between panes is draggable.

Tab navigation (`Ctrl+Left/Right`) and pane focus navigation
(`Ctrl+Shift+Arrow`) are orthogonal: tabs switch the whole tab, pane
navigation moves focus within a tab.

All keyboard shortcuts can be customized in the configuration file.

## Configuration

NTerm uses a configuration file located at `~/.config/nterm/config.toml`. This file will be automatically created with default values on first run.

Example configuration:

```toml
# The shell to execute (optional, defaults to $SHELL or /bin/bash)
shell = "/bin/zsh"

# Number of scrollback lines (optional, default: 10000)
scrollback_lines = 10000

# Enable cursor blinking (optional, default: true)
cursor_blink = true

# Color profile (optional, default: solarized-dark)
# Available: solarized-dark, solarized-light, dracula, monokai, gruvbox-dark, nord
color_profile = "dracula"

# Keyboard shortcuts (optional, all have defaults)
[bindings]
new_tab = "<Ctrl><Shift>T"
prev_tab = "<Ctrl>Left"
next_tab = "<Ctrl>Right"
search = "<Ctrl><Shift>F"
```

### Color Profiles

NTerm includes several built-in color profiles:

- **solarized-dark** (default) - The popular Solarized Dark theme
- **solarized-light** - Solarized Light variant
- **dracula** - The Dracula color scheme
- **monokai** - The classic Monokai theme
- **gruvbox-dark** - Retro groove color scheme
- **nord** - An arctic, north-bluish color palette

You can also define custom colors in the configuration file:

```toml
[colors]
foreground = "#f8f8f2"
background = "#282a36"
black = "#000000"
red = "#ff5555"
green = "#50fa7b"
yellow = "#f1fa8c"
blue = "#bd93f9"
magenta = "#ff79c6"
cyan = "#8be9fd"
white = "#bfbfbf"
bright_black = "#4d4d4d"
bright_red = "#ff6e67"
bright_green = "#5af78e"
bright_yellow = "#f4f99d"
bright_blue = "#caa9fa"
bright_magenta = "#ff92d0"
bright_cyan = "#9aedfe"
bright_white = "#e6e6e6"
```

All configuration options are optional. If not specified, sensible defaults will be used.

## License

MIT
