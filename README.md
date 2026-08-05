# WD-40

> Dev artifact cleaner for macOS — menu bar app + CLI.

WD-40 finds and cleans build artifact directories (`target/`, `node_modules/`, `.next/`, `dist/`, `build/`, and `/tmp/cc-target-*`) to reclaim disk space.

## Two Ways to Use

### Menu Bar App (`WD-40.app`)

A native macOS status bar utility with zero-config scanning.

- **Visual Status**: Icon gets "rustier" as build artifacts grow
- **Disk Panel**: Free space is the headline number, over a capacity gauge that shows the artifact slice in orange and what cleaning it would leave
- **Grouped Results**: Rust, Node Modules, Build Output — every group keeps rows of its own, with aligned size columns and usage bars
- **Readable Names**: The name column sizes itself to the projects on screen, and same-named projects gain the directory that tells them apart
- **Hover for the Path**: Pointing at a row swaps the short name for its full path and how stale it is
- **One-Click Clean**: Individual projects, by group, all, or old only
- **Auto Scan**: Refreshes every 5 minutes
- **Auto Clean**: Configurable interval (1h/6h/12h/24h) + age threshold
- **Settings Window** (⌘,): Launch at Login, auto-clean cadence, age threshold, artifact types, and update preferences in one panel
- **Auto Update**: Sparkle checks the release feed daily; `Check for Updates…` runs it on demand
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
# Menu bar app → /Applications (downloads Sparkle, signs the bundle)
make install

# CLI → ~/.cargo/bin/wd40
make cli
```

Enable **Launch at Login** from the Settings window; it installs the
`com.wd40.app` LaunchAgent for the current user and takes effect at next login.

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

## Updates

The app ships with [Sparkle](https://sparkle-project.org) in
`Contents/Frameworks` and reads its feed from `SUFeedURL` in `Info.plist`.
Sparkle is loaded at runtime, so `cargo run` still works on an unbundled build —
the update menu item is simply absent there.

Releases are published to a Cloudflare Worker + R2 relay (see [`relay/`](relay)):

```bash
UPLOAD_SECRET=... make release VERSION=0.5.0 NOTES="What changed"
```

`scripts/release.sh` builds the bundle, EdDSA-signs the zip with
`.sparkle/bin/sign_update`, writes `appcast.xml`, and uploads both. The signing
key lives in the login Keychain (service `https://sparkle-project.org`, account
`ed25519`) and is **shared with sibling apps** — never run `generate_keys -f`,
it would overwrite the single global key slot and break every feed.

## Performance

- **Discovery**: Parallel directory walk with smart skip rules (hidden dirs, system dirs, symlinks)
- **Sizing**: macOS `getattrlistbulk` API — ~1,600x fewer syscalls than `stat` per file, with automatic fallback to `walkdir` on parse errors
- **Native**: Pure Rust + AppKit via `objc2` — no Electron, no web views

## License

[MIT](LICENSE)
