# Rust Cleaner

> A native macOS menu bar utility to keep your development environment lean by cleaning build artifacts.

<!-- ![Rust Cleaner Screenshot](screenshot.png) -->

Rust Cleaner is a lightweight, native macOS utility for developers. It finds and cleans build artifact directories (`target/`, `node_modules/`, `.next/`, `dist/`, `build/`, and temporary Cargo build dirs) to reclaim disk space.

## Features

- **Native macOS Experience**: Built with pure Rust and native AppKit bindings (`objc2`) — no Electron, no web views, zero bloat.
- **Visual Status**: A status bar icon that gets "rustier" as your accumulated build artifacts grow.
- **Grouped by Type**: Scan results are organized into groups for easy browsing:
  - **Rust** — `target/` directories and `/tmp/cc-target-*` temporary build dirs
  - **Node Modules** — `node_modules/` dependency directories
  - **Build Output** — `.next/`, `dist/`, `build/` framework output directories
- **Info Panel**: Each group has an info button showing the exact scan rules and detection heuristics.
- **Detailed Insights**: View per-project disk usage with integrated bar charts in the menu dropdown.
- **Flexible Cleaning**:
  - One-click clean for individual projects.
  - **Clean by Group**: Remove all artifacts of a specific type (e.g., all Rust targets).
  - **Clean All**: Wipe all detected artifact directories.
  - **Clean Old**: Remove artifacts older than a configurable number of days.
- **Automation**: Configurable auto-clean intervals (1h, 6h, 12h, 24h) and age thresholds (3, 7, 14, 30 days).
- **Interactive Feedback**: Smooth animations during cleaning (🧹) and a celebratory finish (✨).

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
# Directories to scan for projects (default: ["~/Develop"])
scan_dirs = ["/Users/username/Develop", "/Users/username/Projects"]

# Which artifact directory names to scan (default: all known types)
artifact_types = ["target", "node_modules", ".next", "dist", "build"]

# Threshold for "old" artifacts in days (default: 7)
max_age_days = 14

# Maximum directory depth for scanning (default: 5)
max_depth = 4

# Auto-clean interval in hours. Set to 0 to disable (default: 0)
auto_clean_hours = 12
```

## How it Works

Rust Cleaner walks your configured `scan_dirs` looking for known artifact directories. Each type has its own detection heuristic to avoid false positives:

| Directory | Detection Rule |
|-----------|---------------|
| `target/` | Contains `debug/` or `release/` subdirectory |
| `node_modules/` | Contains `.package-lock.json` or `.yarn-integrity` |
| `.next/` | Contains `cache/` or `static/` subdirectory |
| `dist/`, `build/` | Parent has `package.json`, `Cargo.toml`, `build.gradle`, or `platformio.ini` |
| `/tmp/cc-target-*` | Temporary Cargo build dirs (auto-detected) |

Results are grouped by type (Rust, Node Modules, Build Output) with per-group size totals and one-click group cleaning.

The menu bar icon uses a color-tinted SF Symbol that progressively gets "rustier" with spots as total disk usage grows — a visual nudge to clean up.

## License

This project is licensed under the [MIT License](LICENSE).
