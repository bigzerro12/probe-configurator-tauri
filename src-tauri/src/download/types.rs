//! Types for download and install operations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub percent: u32,
    pub transferred: u64,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub cancelled: Option<bool>,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanInstallerResult {
    pub found: bool,
    pub path: String,
    pub message: String,
}

/// Platform-specific download configuration.
pub struct DownloadConfig {
    /// Direct download URL for the installer
    pub url: &'static str,
    /// Save path for download (.tmp extension during download)
    pub save_tmp: PathBuf,
    /// Final path after rename (.exe/.pkg/.deb)
    pub save_final: PathBuf,
    /// Scan path where app looks for existing installer
    pub scan_path: PathBuf,
}

impl DownloadConfig {
    pub fn for_platform() -> Self {
        if cfg!(target_os = "windows") {
            let segger = std::env::var("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default())
                .join("AppData").join("Roaming").join("SEGGER");
            DownloadConfig {
                url: "https://www.segger.com/downloads/jlink/JLink_Windows_x86_64.exe",
                save_tmp: segger.join("JLink_Windows_x86_64.tmp"),
                save_final: segger.join("JLink_Windows_x86_64.exe"),
                scan_path: segger.join("JLink_Windows_x86_64.exe"),
            }
        } else if cfg!(target_os = "macos") {
            let dl = dirs::download_dir().unwrap_or_default();
            DownloadConfig {
                url: "https://www.segger.com/downloads/jlink/JLink_MacOSX_universal.pkg",
                save_tmp: dl.join("JLink_MacOSX_universal.tmp"),
                save_final: dl.join("JLink_MacOSX_universal.pkg"),
                scan_path: dl.join("JLink_MacOSX_universal.pkg"),
            }
        } else {
            let dl = dirs::download_dir().unwrap_or_default();
            DownloadConfig {
                url: "https://www.segger.com/downloads/jlink/JLink_Linux_x86_64.deb",
                save_tmp: dl.join("JLink_Linux_x86_64.tmp"),
                save_final: dl.join("JLink_Linux_x86_64.deb"),
                scan_path: dl.join("JLink_Linux_x86_64.deb"),
            }
        }
    }
}

/// Platform-specific cached installer path in app-local data.
///
/// Linux/macOS use this location as the source of truth after download
/// so cancellation logic can safely clean up Downloads without losing cache.
pub fn cached_installer_path() -> PathBuf {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local").join("share"));
    let filename = if cfg!(target_os = "macos") {
        "JLink_installer.pkg"
    } else if cfg!(target_os = "windows") {
        "JLink_installer.exe"
    } else {
        "JLink_installer.deb"
    };
    data_dir.join("com.probeconfigurator.app").join(filename)
}