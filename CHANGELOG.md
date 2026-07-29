# Changelog

## [0.5.0] — 2026-07-29

### Changed
- **Renamed to WD-40**: crate `rust-cleaner` → `wd40`, app bundle `Rust Cleaner.app` → `WD-40.app`, GUI binary `rust-cleaner` → `wd40-menu`, bundle id `com.wd40.rust-cleaner` → `com.wd40.app`. The `wd40` CLI and `~/.config/wd-40/config.toml` are unchanged.
- **Menu redesign**: rows are attributed strings laid out against shared tab stops, so name / size / usage-bar columns align; group headers carry SF Symbols and a secondary-colored count; the header shows total reclaimable space, app version, and free disk of total.
- **Settings submenu**: auto-clean cadence, age threshold, Launch at Login, automatic update checks, and per-group Scan Rules moved out of the main menu into `Settings ▸`, with native checkmarks instead of inline `✓` text.
- **`Rescan` is ⌘R and `Quit WD-40` is ⌘Q.**

### Added
- **Sparkle auto-update**: `Contents/Frameworks/Sparkle.framework` is loaded at runtime (`src/updater.rs`), so unbundled `cargo run` builds still start — they simply have no update menu item. Daily background checks plus an on-demand `Check for Updates…`.
- **`relay/`**: Cloudflare Worker + R2 that serves `appcast.xml` and signed build archives, authenticated by `UPLOAD_SECRET`.
- **`scripts/fetch-sparkle.sh`, `scripts/bundle.sh`, `scripts/release.sh`**: fetch + cache Sparkle, build a signed `dist/WD-40.app`, and publish an EdDSA-signed release to the feed.
- **Launch at Login toggle** (`src/autostart.rs`) that writes and bootstraps the `com.wd40.app` LaunchAgent from inside the app.

### Removed
- `com.wd40.rust-cleaner.plist` and the `make autostart` / `make no-autostart` targets — the in-app toggle is now the single mechanism.


## [0.4.2] — 2026-04-18

### Fixed
- **Project name in menu bar**: targets under `~/.cargo-target/` now show their path relative to the shared root (e.g. `smart-router`, `smart-router/feat-xxx`) instead of `.cargo-target` or an indistinguishable `smart-router` repeated for every session

## [0.4.1] — 2026-04-18

### Changed
- **Per-session visibility under `~/.cargo-target/`**: list each `<project>/<session>/` session subdir as its own row so users can see the breakdown (previously only the project root was shown)
- **Nested size adjustment**: when a target is an ancestor of another target, the ancestor's reported size now subtracts the descendant's size — no double-counting, total matches actual disk usage

## [0.4.0] — 2026-04-18

### Added
- **Disk-aware size reporting** (`src/disk.rs`): query filesystem capacity via `statvfs`; sizes are clamped so a single target never exceeds the volume's total bytes
- **Shared `~/.cargo-target/` scan**: detects per-project cargo target roots under a shared `CARGO_TARGET_DIR`, with fallback to `<project>/<session>/debug|release` two-level layouts used by Claude Code session wrappers
- **Remaining disk space** surfaced via `AppState::remaining_disk_space`, using the first target's volume (or first scan dir) as reference

### Fixed
- **TOCTOU on delete**: `clean_all` / `clean_old` re-check the target is still a directory (not swapped to a symlink or removed) before `remove_dir_all`
- **Size overflow on clean**: freed-bytes accumulators now use `saturating_add` to avoid wrap on pathological inputs
- **Bulk-size overcount**: cap each directory's reported size at the enclosing volume's total bytes, falling back to `walkdir` when the bulk reader overshoots

### Changed
- `sum_bytes` helper extracted into `disk.rs` and shared between menu/CLI to keep accumulation consistent

## [0.3.0] — 2026-03-16

### Added
- **CLI binary (`wd40`)**: Terminal interface for scanning and cleaning dev artifacts
  - `wd40 scan` — display all artifact directories grouped by type
  - `wd40 clean` / `wd40 clean-old` — remove artifacts with optional `--dry-run`
  - Filter by group (`-g rust/node/build`), custom age threshold (`-d <days>`)
- **Auto-scan**: Menu bar app rescans every 5 minutes automatically
- **Two-phase scan**: Instant directory discovery, then background size computation
- **Shared library**: Core logic (`scanner`, `config`, `cleaner`) shared between GUI and CLI

### Fixed
- **getattrlistbulk infinite loop**: Parse errors in large directories caused 99% CPU hang; now falls back to walkdir on any parse error
- **Symlink following**: Bulk size API and discovery walker no longer follow symlinks (prevents cycles and inflated counts)
- **Size overflow**: Corrupt partial size data no longer reported; full fallback on error
- **SCANNING flag stuck**: Phase 2 thread wrapped in `catch_unwind` to guarantee UI reset

### Security
- Pre-delete symlink check: verify target is still a real directory before `remove_dir_all` (TOCTOU mitigation)
- `dir_size_fallback` no longer follows symlinks (`follow_links(false)`)
- `/tmp/cc-target-*` collection skips symlinks
- `--days 0` rejected to prevent accidental deletion of all targets

## [0.2.0] — 2026-03-15

### Added
- Grouped scan results: Rust, Node Modules, Build Output with per-group totals
- Multi-type artifact detection: `target/`, `node_modules/`, `.next/`, `dist/`, `build/`
- `/tmp/cc-target-*` temporary build directory detection
- Per-group clean and info panels in menu bar
- macOS `getattrlistbulk` API for fast directory sizing (~1,600x fewer syscalls)

## [0.1.0] — 2026-03-14

### Added
- Initial release: native macOS menu bar utility
- Scan `~/Develop` for Rust `target/` directories
- Visual status icon with "rust spots" indicating artifact size
- One-click cleaning of individual or all targets
