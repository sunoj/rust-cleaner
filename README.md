# WD-40

> Dev artifact cleaner for macOS — menu bar app + CLI.

WD-40 finds and cleans build artifact directories (`target/`, `node_modules/`, `.next/`, `dist/`, `build/`, and `/tmp/cc-target-*`) to reclaim disk space.

## Two Ways to Use

### Menu Bar App (`rust-cleaner`)

A native macOS status bar utility with zero-config scanning.

- **Visual Status**: Icon gets "rustier" as build artifacts grow
- **Grouped Results**: Rust, Node Modules, Build Output — with per-group totals
- **One-Click Clean**: Individual projects, by group, all, or old only
- **Auto Scan**: Refreshes every 5 minutes
- **Auto Clean**: Configurable interval (1h/6h/12h/24h) + age threshold
- **Two-Phase Scan**: Instant discovery, background size computation

### CLI (`wd40`)

Fast terminal interface for scripting and quick checks.

```
$ wd40
Scanning... found 17 targets

Rust — 2.7G
    764.3M    0d  [cc-target]  /tmp/cc-target-ai-dispatch-270
    338.0M    0d  [cc-target]  /tmp/cc-target-dev-cleaner-269

Node Modules — 1.6G
    351.0M    0d  [node_modules]  ~/Develop/ai/hiboss/node_modules
    308.9M    1d  [node_modules]  ~/Develop/ai/website-store/node_modules

Total: 4.3G in 17 targets
```

**Commands:**

| Command | Description |
|---------|-------------|
| `wd40` / `wd40 scan` | Scan and display all artifacts |
| `wd40 clean` | Remove all artifact directories |
| `wd40 clean-old` | Remove artifacts older than N days |
| `wd40 scan -g rust` | Filter by group: `rust`, `node`, `build` |
| `wd40 clean-old -d 14` | Custom age threshold |
| `wd40 clean --dry-run` | Preview without deleting |

## Installation

### From Source

```bash
# Build both binaries
make build

# Install menu bar app to /Applications
make install

# Install CLI to PATH
cp "$CARGO_TARGET_DIR/release/wd40" ~/.cargo/bin/

# Optional: auto-start menu bar app on login
make autostart
```

## Configuration

`~/.config/wd-40/config.toml` — shared by both app and CLI.

```toml
scan_dirs = ["/Users/username/Develop"]
artifact_types = ["target", "node_modules", ".next", "dist", "build"]
max_age_days = 7
max_depth = 5
auto_clean_hours = 6   # 0 to disable
```

## Detection Rules

| Directory | Heuristic |
|-----------|-----------|
| `target/` | Contains `debug/` or `release/` |
| `node_modules/` | Contains `.package-lock.json` or `.yarn-integrity` |
| `.next/` | Contains `cache/` or `static/` |
| `dist/`, `build/` | Parent has `package.json`, `Cargo.toml`, `build.gradle`, or `platformio.ini` |
| `/tmp/cc-target-*` | Auto-detected temporary Cargo build dirs |

## Performance

- **Discovery**: Parallel directory walk with smart skip rules (hidden dirs, system dirs, symlinks)
- **Sizing**: macOS `getattrlistbulk` API — ~1,600x fewer syscalls than `stat` per file, with automatic fallback to `walkdir` on parse errors
- **Native**: Pure Rust + AppKit via `objc2` — no Electron, no web views

## License

[MIT](LICENSE)
