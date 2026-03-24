//! Platform-specific J-Link installation.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::download::types::InstallResult;
use crate::platform;
use crate::process::NoWindow;

pub async fn install(
    installer_path: &str,
    cancelled: &'static AtomicBool,
) -> Result<InstallResult, String> {
    if cfg!(target_os = "windows") {
        install_windows(installer_path, cancelled).await
    } else if cfg!(target_os = "macos") {
        install_macos(installer_path).await
    } else {
        install_linux(installer_path).await
    }
}

// ─── Windows ──────────────────────────────────────────────────────────────────

async fn install_windows(
    installer_path: &str,
    cancelled: &'static AtomicBool,
) -> Result<InstallResult, String> {
    let dest = std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default())
        .join("AppData").join("Roaming").join("SEGGER");

    let installer = installer_path.to_string();
    let dest_str = dest.to_string_lossy().to_string();

    // Spawn PowerShell with -Wait so we can cancel by killing the child
    let ps_cmd = format!(
        "Start-Process -FilePath '{}' -ArgumentList '/S','/D={}' -Verb RunAs -Wait",
        installer.replace('\'', "`'"),
        dest_str.replace('\'', "`'")
    );

    let mut child = match std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
        .no_window()
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Ok(InstallResult {
            success: false, cancelled: Some(true),
            message: format!("Failed to launch installer: {}", e), path: None,
        }),
    };

    let install_start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let start = std::time::Instant::now();
    let mut dll_check_ticker: u64 = 0;

    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Handle cancel
        if cancelled.load(Ordering::SeqCst) {
            let pid = child.id();
            let _ = child.kill();
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .no_window()
                .status();
            log::info!("[install] Cancelled (pid={})", pid);
            return Ok(InstallResult {
                success: false, cancelled: Some(true),
                message: "Installation cancelled.".to_string(), path: None,
            });
        }

        // Check if PowerShell exited
        match child.try_wait() {
            Ok(Some(s)) if !s.success() => {
                return Ok(InstallResult {
                    success: false, cancelled: Some(true),
                    message: "UAC denied or installation failed.".to_string(), path: None,
                });
            }
            Ok(Some(_)) => {} // Exited successfully — keep polling for DLL
            Ok(None) => {}    // Still running
            Err(e) => return Ok(InstallResult {
                success: false, cancelled: None,
                message: format!("Process error: {}", e), path: None,
            }),
        }

        // Check DLL every ~1s (not every 300ms)
        dll_check_ticker += 1;
        if dll_check_ticker % 3 == 0 {
            if let Some(dir) = platform::find_jlink_in_search_dirs() {
                let dll = dir.join("JLink_x64.dll");
                if let Ok(meta) = std::fs::metadata(&dll) {
                    if let Ok(mtime) = meta.modified() {
                        let mtime_ms = mtime
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis();
                        if mtime_ms >= install_start_ms {
                            let dir_str = dir.to_string_lossy().to_string();
                            platform::add_to_system_path(&dir);
                            log::info!("[install] Complete: {}", dir_str);
                            return Ok(InstallResult {
                                success: true, cancelled: None,
                                message: "J-Link installed successfully.".to_string(),
                                path: Some(dir_str),
                            });
                        }
                    }
                }
            }
        }

        if start.elapsed().as_secs() > 120 {
            let _ = child.kill();
            return Ok(InstallResult {
                success: false, cancelled: None,
                message: "Installation timed out.".to_string(), path: None,
            });
        }
    }
}

// ─── macOS ────────────────────────────────────────────────────────────────────

async fn install_macos(installer_path: &str) -> Result<InstallResult, String> {
    let cmd = format!(
        "do shell script \"installer -pkg \\\"{}\\\" -target /\" with administrator privileges",
        installer_path
    );
    match std::process::Command::new("osascript").args(["-e", &cmd]).status() {
        Ok(s) if s.success() => Ok(InstallResult {
            success: true, cancelled: None,
            message: "J-Link installed successfully.".to_string(),
            path: platform::find_jlink_in_search_dirs()
                .map(|d| d.to_string_lossy().to_string()),
        }),
        Ok(_) => Ok(InstallResult {
            success: false, cancelled: Some(true),
            message: "Installation cancelled.".to_string(), path: None,
        }),
        Err(e) => Ok(InstallResult {
            success: false, cancelled: None,
            message: format!("Install failed: {}", e), path: None,
        }),
    }
}

// ─── Linux ───────────────────────────────────────────────────────────────────

async fn install_linux(installer_path: &str) -> Result<InstallResult, String> {
    match std::process::Command::new("pkexec")
        .args(["dpkg", "-i", installer_path])
        .status()
    {
        Ok(s) if s.success() => Ok(InstallResult {
            success: true, cancelled: None,
            message: "J-Link installed successfully.".to_string(),
            path: platform::find_jlink_in_search_dirs()
                .map(|d| d.to_string_lossy().to_string()),
        }),
        Ok(s) if s.code() == Some(126) => Ok(InstallResult {
            success: false, cancelled: Some(true),
            message: "Installation cancelled.".to_string(), path: None,
        }),
        _ => Ok(InstallResult {
            success: false, cancelled: None,
            message: "Installation failed.".to_string(), path: None,
        }),
    }
}