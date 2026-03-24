//! Linux-specific PATH management and search directories.

use std::path::{Path, PathBuf};
use std::io::Write;

pub fn search_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/SEGGER/JLink"),
    ]
}

/// Add dir to /etc/environment or ~/.profile.
pub fn add_to_system_path(dir: &Path) -> bool {
    let dir_str = dir.to_string_lossy().to_string();

    // Try /etc/environment (system-wide, requires pkexec)
    if let Ok(content) = std::fs::read_to_string("/etc/environment") {
        if content.contains(&dir_str) {
            log::info!("[platform] Already in /etc/environment");
            return true;
        }
    }

    // Try writing via pkexec (GUI sudo)
    let script = format!(
        r#"#!/bin/sh
if ! grep -q '{dir}' /etc/environment; then
    echo 'PATH="{dir}:$PATH"' >> /etc/environment
fi"#,
        dir = dir_str
    );

    let script_path = std::env::temp_dir().join("jlink_add_path.sh");
    if std::fs::write(&script_path, script.as_bytes()).is_ok() {
        let result = std::process::Command::new("pkexec")
            .args(["sh", &script_path.to_string_lossy()])
            .status();
        let _ = std::fs::remove_file(&script_path);

        match result {
            Ok(s) if s.success() => {
                log::info!("[platform] Added to /etc/environment: {}", dir_str);
                return true;
            }
            Ok(s) if s.code() == Some(126) => {
                log::warn!("[platform] pkexec cancelled by user");
            }
            _ => {
                log::warn!("[platform] pkexec failed");
            }
        }
    }

    // Fallback: ~/.profile
    add_to_shell_profile(&dir_str)
}

fn add_to_shell_profile(dir_str: &str) -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    let profile = home.join(".profile");
    let line = format!("\nexport PATH=\"{}:$PATH\"\n", dir_str);

    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&profile) {
        if f.write_all(line.as_bytes()).is_ok() {
            log::info!("[platform] Added to ~/.profile: {}", dir_str);
            return true;
        }
    }
    false
}