// Launch-at-login control for WD-40, backed by a per-user LaunchAgent.
// Exports: `is_enabled`, `set_enabled`.
// Deps: std, dirs, libc.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LABEL: &str = "com.wd40.app";

fn plist_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("Library/LaunchAgents").join(format!("{LABEL}.plist")))
}

pub fn is_enabled() -> bool {
    plist_path().is_some_and(|path| path.exists())
}

/// Install or remove the LaunchAgent and load/unload it for the current session.
pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let path = plist_path().ok_or("no home directory")?;
    let domain = format!("gui/{}", unsafe { libc::getuid() });

    if !enabled {
        let _ = launchctl(&["bootout", &format!("{domain}/{LABEL}")]);
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
    fs::write(&path, agent_plist(&exe.to_string_lossy())).map_err(|err| err.to_string())?;
    let _ = launchctl(&["bootout", &format!("{domain}/{LABEL}")]);
    launchctl(&["bootstrap", &domain, &path.to_string_lossy()])
}

fn launchctl(args: &[&str]) -> Result<(), String> {
    let output = Command::new("launchctl")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
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
}
