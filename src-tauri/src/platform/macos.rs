//! macOS-specific PATH management and search directories.

use std::path::{Path, PathBuf};

pub fn search_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/Applications/SEGGER/JLink"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
    ]
}

/// Add dir to /etc/paths.d/jlink (requires sudo, uses osascript).
pub fn add_to_system_path(dir: &Path) -> bool {
    let dir_str = dir.to_string_lossy().to_string();
    let paths_d = "/etc/paths.d/jlink";

    // Check if already set
    if let Ok(content) = std::fs::read_to_string(paths_d) {
        if content.trim() == dir_str {
            log::info!("[platform] Already in /etc/paths.d/jlink");
            return true;
        }
    }

    // Write via osascript (prompts admin password)
    let cmd = format!(
        "do shell script \"echo '{}' > /etc/paths.d/jlink\" with administrator privileges",
        dir_str
    );

    match std::process::Command::new("osascript").args(["-e", &cmd]).status() {
        Ok(s) if s.success() => {
            log::info!("[platform] Added to /etc/paths.d/jlink: {}", dir_str);
            true
        }
        _ => {
            log::warn!("[platform] Could not write /etc/paths.d/jlink — trying ~/.zprofile");
            add_to_shell_profile(&dir_str)
        }
    }
}

fn add_to_shell_profile(dir_str: &str) -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    let profile = home.join(".zprofile");
    let line = format!("\nexport PATH=\"{}:$PATH\"\n", dir_str);

    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&profile) {
        if f.write_all(line.as_bytes()).is_ok() {
            log::info!("[platform] Added to ~/.zprofile: {}", dir_str);
            return true;
        }
    }
    false
}