// Rust toolchains under RUSTUP_HOME. Unlike every other artifact WD-40 handles,
// a toolchain is global state rustup owns: it comes off with `rustup toolchain
// uninstall`, never with an unlink, and anything still pointing at one keeps it
// off the list entirely.
// Exports: `removable`, `uninstall`, `label`.
// Deps: std, dirs, toml, walkdir.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// How deep under a scan root a `rust-toolchain.toml` is looked for. Pins live
/// at the root of a project, so a shallow pass finds them all; going deeper
/// would cost a second full traversal to learn nothing.
const PIN_DEPTH: usize = 3;

/// Directory names the pin search never enters.
const PIN_SKIP: &[&str] = &["target", "node_modules", "dist", "build"];

/// Toolchains a scan may offer, newest-installed order not guaranteed.
///
/// Fail-safe: if rustup's own settings cannot be read, nothing is offered. The
/// file is what says which toolchain is the default, and without it every
/// candidate would look unused.
pub fn removable(scan_dirs: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let home = rustup_home();
    let Some(pins) = settings_pins(&home) else { return Vec::new() };
    let mut pins = pins;
    pins.extend(project_pins(scan_dirs, max_depth));
    installed(&home)
        .into_iter()
        .filter(|path| !is_pinned(path, &pins))
        .collect()
}

/// Remove one toolchain through rustup, so rustup's own record of what is
/// installed stays true. Deleting the directory would leave it claiming the
/// toolchain is still there.
pub fn uninstall(path: &Path) -> Result<(), String> {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err("toolchain directory has no name".into());
    };
    let Some(rustup) = rustup_bin() else {
        return Err("rustup is not installed, so this toolchain cannot be removed safely".into());
    };
    let output = Command::new(&rustup)
        .args(["toolchain", "uninstall", name])
        .output()
        .map_err(|err| format!("could not run {}: {err}", rustup.display()))?;
    if !output.status.success() {
        return Err(first_line(&String::from_utf8_lossy(&output.stderr))
            .unwrap_or_else(|| format!("rustup toolchain uninstall {name} failed")));
    }
    // rustup exits 0 for a toolchain it decided not to touch, so the directory
    // is the only proof the removal happened.
    match path.exists() {
        true => Err("rustup reported success but the toolchain is still on disk".into()),
        false => Ok(()),
    }
}

/// What a toolchain directory is called with the noise taken off: every
/// toolchain on this Mac ends in `-apple-darwin`, and the architecture only
/// says something when it is not the one this Mac runs.
pub fn label(path: &Path) -> String {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return "toolchain".to_string();
    };
    let native = format!("-{}-apple-darwin", std::env::consts::ARCH);
    if let Some(base) = name.strip_suffix(&native) {
        return base.to_string();
    }
    name.strip_suffix("-apple-darwin").unwrap_or(name).to_string()
}

fn rustup_home() -> PathBuf {
    std::env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".rustup")))
        .unwrap_or_default()
}

/// The rustup binary. A bundled app inherits a bare PATH from launchd, so the
/// usual install locations are tried by hand rather than left to a lookup.
fn rustup_bin() -> Option<PathBuf> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".cargo")));
    cargo_home
        .map(|home| home.join("bin/rustup"))
        .into_iter()
        .chain([PathBuf::from("/opt/homebrew/bin/rustup"), PathBuf::from("/usr/local/bin/rustup")])
        .find(|path| path.is_file())
}

/// Installed toolchain directories. Symlinks are skipped: those are toolchains
/// added with `rustup toolchain link`, which point at a build tree of the
/// user's own that is not ours to measure or remove.
fn installed(home: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(home.join("toolchains")) else { return Vec::new() };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| !path.is_symlink() && path.is_dir())
        .collect()
}

/// Channels rustup itself has spoken for: the default, and every directory
/// override. `None` when the settings file cannot be read or parsed at all.
fn settings_pins(home: &Path) -> Option<BTreeSet<String>> {
    let text = std::fs::read_to_string(home.join("settings.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let mut pins = BTreeSet::new();
    // A settings file with no default is rustup mid-install, not a Mac with no
    // toolchain pinned — refuse to offer anything rather than guess.
    pins.insert(value.get("default_toolchain")?.as_str()?.to_string());
    if let Some(overrides) = value.get("overrides").and_then(toml::Value::as_table) {
        pins.extend(overrides.values().filter_map(toml::Value::as_str).map(str::to_string));
    }
    Some(pins)
}

/// Channels pinned by a `rust-toolchain.toml` or `rust-toolchain` file inside
/// the scan roots.
fn project_pins(scan_dirs: &[PathBuf], max_depth: usize) -> BTreeSet<String> {
    let depth = max_depth.min(PIN_DEPTH);
    scan_dirs
        .iter()
        .filter(|dir| dir.exists())
        .flat_map(|dir| {
            walkdir::WalkDir::new(dir)
                .max_depth(depth)
                .into_iter()
                .filter_entry(|entry| !skip_for_pins(entry))
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_dir())
                .filter_map(|entry| pin_in(entry.path()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn skip_for_pins(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return false;
    }
    if entry.path_is_symlink() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    name.starts_with('.') || PIN_SKIP.contains(&name.as_ref())
}

fn pin_in(dir: &Path) -> Option<String> {
    ["rust-toolchain.toml", "rust-toolchain"]
        .iter()
        .filter_map(|name| std::fs::read_to_string(dir.join(name)).ok())
        .find_map(|text| channel_of(&text))
}

/// The channel a pin file names, in either of the two shapes rustup accepts:
/// a `[toolchain] channel = "…"` table, or a bare channel name on its own.
/// A `path = "…"` toolchain pins nothing that lives under RUSTUP_HOME.
fn channel_of(text: &str) -> Option<String> {
    if let Ok(value) = toml::from_str::<toml::Value>(text) {
        return value
            .get("toolchain")?
            .get("channel")?
            .as_str()
            .map(str::to_string);
    }
    let line = text.lines().map(str::trim).find(|line| !line.is_empty())?;
    (!line.starts_with('#')).then(|| line.to_string())
}

/// Whether a pin claims this toolchain directory. A pin is a channel spec
/// (`stable`, `1.82`, `nightly-2024-01-01`) and the directory carries the host
/// triple as well, so the spec has to match up to a component boundary — and
/// `1.76` has to be read as claiming `1.76.0`, which is what rustup resolved it
/// to when it installed.
fn is_pinned(path: &Path, pins: &BTreeSet<String>) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else { return true };
    pins.iter().any(|pin| {
        name == pin
            || name.strip_prefix(pin.as_str()).is_some_and(|rest| {
                rest.starts_with('-') || rest.starts_with('.')
            })
    })
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{channel_of, is_pinned, label};
    use std::collections::BTreeSet;
    use std::path::Path;

    fn pins(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn a_channel_pin_claims_the_toolchain_it_resolved_to() {
        let set = pins(&["stable", "1.82"]);
        assert!(is_pinned(Path::new("/r/toolchains/stable-aarch64-apple-darwin"), &set));
        assert!(is_pinned(Path::new("/r/toolchains/1.82-aarch64-apple-darwin"), &set));
        assert!(!is_pinned(Path::new("/r/toolchains/1.89.0-aarch64-apple-darwin"), &set));
    }

    /// `rustup override` stores the resolved name, pin files store the channel.
    #[test]
    fn a_fully_resolved_pin_matches_exactly() {
        let set = pins(&["nightly-aarch64-apple-darwin"]);
        assert!(is_pinned(Path::new("/r/toolchains/nightly-aarch64-apple-darwin"), &set));
        assert!(!is_pinned(Path::new("/r/toolchains/nightly-x86_64-apple-darwin"), &set));
    }

    /// A two-part pin was installed as its three-part release, so it has to
    /// hold that one back too.
    #[test]
    fn a_pin_claims_the_point_release_it_installed() {
        let set = pins(&["1.76"]);
        assert!(is_pinned(Path::new("/r/toolchains/1.76.0-x86_64-apple-darwin"), &set));
    }

    /// The red line: a prefix that stops mid-component must not hold an
    /// unrelated toolchain back, or nothing on the list is ever offered.
    #[test]
    fn a_partial_version_does_not_claim_a_different_one() {
        assert!(!is_pinned(Path::new("/r/toolchains/1.76.0-x86_64-apple-darwin"), &pins(&["1.7"])));
        assert!(!is_pinned(Path::new("/r/toolchains/1.95.0-aarch64-apple-darwin"), &pins(&["1.9"])));
        assert!(!is_pinned(Path::new("/r/toolchains/stable-aarch64-apple-darwin"), &pins(&["sta"])));
    }

    /// Anything the app cannot name, it must not offer to delete.
    #[test]
    fn a_nameless_directory_is_treated_as_pinned() {
        assert!(is_pinned(Path::new("/"), &pins(&["stable"])));
    }

    #[test]
    fn both_pin_file_shapes_are_read() {
        assert_eq!(
            channel_of("[toolchain]\nchannel = \"1.82\"\ncomponents = [\"clippy\"]\n").as_deref(),
            Some("1.82")
        );
        assert_eq!(channel_of("nightly-2024-01-01\n").as_deref(), Some("nightly-2024-01-01"));
        assert_eq!(channel_of("\n\nstable\n").as_deref(), Some("stable"));
    }

    /// A pin that points at a local build directory says nothing about what
    /// lives under RUSTUP_HOME.
    #[test]
    fn a_path_toolchain_pins_no_channel() {
        assert_eq!(channel_of("[toolchain]\npath = \"/home/me/rust/build/host/stage1\"\n"), None);
    }

    #[test]
    fn a_label_drops_the_host_triple_but_keeps_a_foreign_arch() {
        let native = format!("1.82-{}-apple-darwin", std::env::consts::ARCH);
        assert_eq!(label(Path::new(&format!("/r/toolchains/{native}"))), "1.82");
        assert_eq!(
            label(Path::new("/r/toolchains/1.76.0-powerpc-apple-darwin")),
            "1.76.0-powerpc"
        );
    }
}
