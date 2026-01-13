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
- ✅ Scrollbar support
- ✅ 10,000 lines scrollback buffer
- ✅ Uses system default shell ($SHELL)
- ✅ Blinking cursor
- ✅ XDG-compliant configuration file

## Keyboard Shortcuts

- `Ctrl+Shift+T` - Create a new terminal tab
- `Ctrl+Left` - Go to previous tab (wraps to last tab)
- `Ctrl+Right` - Go to next tab (wraps to first tab)

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
```

All configuration options are optional. If not specified, sensible defaults will be used.

## License

MIT
