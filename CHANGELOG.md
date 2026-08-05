# Changelog

## [Unreleased]

### Added
- **Disk panel** (`src/disk_panel.rs`): the menu opens on free space as a headline number, a capacity gauge splitting used / build artifacts / free, and what cleaning would leave. Free disk was a footnote in the header caption before.
- **Hover reveals the path** (`src/hover.rs`): a highlighted project row swaps its short name for the full path and the artifact's age. `NSMenuItem.toolTip` does not render in status-bar menus, so the row itself is the surface.

### Changed
- **The name column sizes itself** (`src/style.rs`): tab stops are measured from the names actually on screen instead of a fixed 28-character budget, so full names show wherever they fit; longer ones elide from the left and stay readable on hover.
- **Rows are shared fairly across groups** (`src/menu_rows.rs`): every non-empty group keeps up to 3 rows before the biggest group takes the rest, so a small group is no longer reduced to a header and "3 more not shown".
- **Same-named projects are disambiguated** (`src/names.rs`): two `web` projects read as `alpha/web` and `beta/web`.
- The app version moved from the header to a caption above `Quit`, leaving the top of the menu to the disk panel.

### Fixed
- `wd40` with no arguments panicked on `args[2..]` — the documented default invocation.

## [0.5.0] — 2026-07-29

### Added
- **Dedicated Settings window** (`src/settings_window.rs`, ⌘,): General (Launch at Login), Cleaning (auto-clean cadence, age threshold), Artifact Types to Scan, and Updates (auto-check + Check Now) in one panel. Artifact types had no UI before — they were config-file only. Replaces the Settings submenu so each setting has exactly one home; the informational Scan Rules submenu stays in the menu.

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

### Fixed
- **Launch at Login no longer fights the running app**: the toggle previously ran `launchctl bootstrap`, which started a *second* menu bar instance beside the running one, and `bootout`, which terminated the app mid-handler and left the plist behind so the toggle showed a stale "on". It now only writes or removes the LaunchAgent — launchd loads `~/Library/LaunchAgents` at login by itself.
- **Release notes can no longer break the update feed**: `]]>` inside `NOTES` closed the CDATA section early and produced a malformed `appcast.xml`, which would have broken updates for every installed copy. The terminator is now split across two sections and `scripts/release.sh` refuses to upload an appcast that fails `xmllint`.
- **`make install` removes the pre-rename install** (`Rust Cleaner.app` and its LaunchAgent), which would otherwise have left two menu bar apps starting at login.

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
