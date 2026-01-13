# NTerm Makefile

.PHONY: all build run release clean install uninstall help

# Default target
all: build

# Build the project in debug mode
build:
	@echo "Building nterm (debug)..."
	@cargo build

# Build the project in release mode
release:
	@echo "Building nterm (release)..."
	@cargo build --release
	@echo "Release binary available at: target/release/nterm"

# Run the project
run:
	@cargo run

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean

# Install to /usr/local/bin (requires sudo)
install: release
	@echo "Installing nterm to /usr/local/bin..."
	@sudo install -Dm755 target/release/nterm /usr/local/bin/nterm
	@echo "Installing desktop file..."
	@sudo install -Dm644 data/nterm.desktop /usr/share/applications/nterm.desktop
	@echo "Installing icon..."
	@sudo install -Dm644 data/terminal.svg /usr/share/icons/hicolor/scalable/apps/nterm.svg
	@sudo gtk-update-icon-cache /usr/share/icons/hicolor/ -f -t 2>/dev/null || true
	@echo "Installation complete!"

# Uninstall from /usr/local/bin (requires sudo)
uninstall:
	@echo "Uninstalling nterm from /usr/local/bin..."
	@sudo rm -f /usr/local/bin/nterm
	@echo "Removing desktop file..."
	@sudo rm -f /usr/share/applications/nterm.desktop
	@echo "Removing icon..."
	@sudo rm -f /usr/share/icons/hicolor/scalable/apps/nterm.svg
	@sudo gtk-update-icon-cache /usr/share/icons/hicolor/ -f -t 2>/dev/null || true
	@echo "Uninstallation complete!"

# Run tests
test:
	@cargo test

# Check code without building
check:
	@cargo check

# Format code
fmt:
	@cargo fmt

# Run clippy linter
clippy:
	@cargo clippy -- -D warnings

# Help target
help:
	@echo "NTerm Makefile targets:"
	@echo "  make build     - Build the project in debug mode"
	@echo "  make release   - Build the project in release mode"
	@echo "  make run       - Run the project"
	@echo "  make clean     - Clean build artifacts"
	@echo "  make install   - Install to /usr/local/bin (requires sudo)"
	@echo "  make uninstall - Remove from /usr/local/bin (requires sudo)"
	@echo "  make test      - Run tests"
	@echo "  make check     - Check code without building"
	@echo "  make fmt       - Format code with rustfmt"
	@echo "  make clippy    - Run clippy linter"
	@echo "  make help      - Show this help message"
