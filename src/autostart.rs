// Launch-at-login control for WD-40, backed by a per-user LaunchAgent.
// launchd loads ~/Library/LaunchAgents at login, so writing the plist is the
// entire mechanism. Deliberately no launchctl: bootstrap would spawn a second
// instance alongside the running app, and bootout would terminate this one
// mid-handler, leaving the plist behind and the toggle showing a stale state.
// Exports: `is_enabled`, `set_enabled`.
// Deps: std, dirs.

use std::fs;
use std::path::PathBuf;

const LABEL: &str = "com.wd40.app";

fn plist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

pub fn is_enabled() -> bool {
    plist_path().is_some_and(|path| path.exists())
}

/// Install or remove the LaunchAgent. Takes effect at the next login; the
/// running instance is left alone either way.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = plist_path().ok_or("no home directory")?;

    if !enabled {
        return match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        };
    }

    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&path, agent_plist(&exe.to_string_lossy())).map_err(|err| err.to_string())
}

fn agent_plist(program: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#,
        escape_xml(program)
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::{agent_plist, escape_xml};

    #[test]
    fn escapes_xml_metacharacters_in_paths() {
        assert_eq!(escape_xml("/a&b/<c>"), "/a&amp;b/&lt;c&gt;");
    }

    #[test]
    fn agent_plist_embeds_program_path() {
        let plist = agent_plist("/Applications/WD-40.app/Contents/MacOS/wd40-menu");
        assert!(plist.contains("<string>/Applications/WD-40.app/Contents/MacOS/wd40-menu</string>"));
        assert!(plist.contains("<string>com.wd40.app</string>"));
    }

    #[test]
    fn agent_plist_is_valid_xml_for_awkward_paths() {
        let plist = agent_plist("/Users/a&b/WD-40.app/Contents/MacOS/wd40-menu");
        assert!(plist.contains("/Users/a&amp;b/"));
        assert!(!plist.contains("/Users/a&b/"));
    }
}
