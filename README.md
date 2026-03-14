# Rust Cleaner

> A native macOS menu bar utility to keep your development environment lean by cleaning Rust compilation artifacts.

<!-- ![Rust Cleaner Screenshot](screenshot.png) -->

Rust Cleaner is a lightweight, native macOS utility for Rust developers. It finds and cleans `target/` directories to reclaim disk space.

## Features

- **Native macOS Experience**: Built with pure Rust and native AppKit bindings (`objc2`) — no Electron, no web views, zero bloat.
- **Visual Status**: A status bar icon that gets "rustier" as your accumulated `target/` directories grow.
- **Detailed Insights**: View per-project disk usage with integrated bar charts in the menu dropdown.
- **Flexible Cleaning**:
  - One-click clean for individual projects.
  - **Clean All**: Wipe all detected `target/` directories.
  - **Clean Old**: Remove artifacts older than a configurable number of days.
- **Automation**: Configurable auto-clean intervals (1h, 6h, 12h, 24h) and age thresholds (3, 7, 14, 30 days).
- **Interactive Feedback**: Enjoy smooth animations during the cleaning process (🧹) and a celebratory finish (✨).

## Installation

### From Source

Ensure you have the Rust toolchain installed.

1. **Build the binary**:
   ```bash
   make build
   ```

2. **Create the .app bundle**:
   ```bash
   make bundle
   ```

3. **Install to /Applications**:
   ```bash
   make install
   ```

4. **Enable Auto-start on Login**:
   ```bash
   make autostart
   ```

## Configuration

Rust Cleaner stores its configuration in `~/.config/wd-40/config.toml`.

```toml
# Directories to scan for Rust projects (default: ["~/Develop"])
scan_dirs = ["/Users/username/Develop", "/Users/username/Projects"]

# Threshold for "old" artifacts in days (default: 7)
max_age_days = 14

# Maximum directory depth for scanning (default: 5)
max_depth = 4

# Auto-clean interval in hours. Set to 0 to disable (default: 0)
auto_clean_hours = 12
```

## How it Works

Rust Cleaner walks your configured `scan_dirs` looking for directories named `target/` that contain `debug/` or `release/` subdirectories (confirming they're Rust compilation artifacts). It calculates size and last modification time for each.

The menu bar icon uses a color-tinted SF Symbol that progressively gets "rustier" with spots as total disk usage grows — a visual nudge to clean up.

## License

This project is licensed under the [MIT License](LICENSE).
