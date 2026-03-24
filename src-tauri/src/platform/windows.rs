//! Windows-specific PATH management and search directories.

use std::path::{Path, PathBuf};

pub fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Ok(profile) = std::env::var("USERPROFILE") {
        dirs.push(PathBuf::from(&profile).join("AppData").join("Roaming").join("SEGGER"));
    }
    dirs.push(PathBuf::from(r"C:\Program Files\SEGGER"));
    dirs.push(PathBuf::from(r"C:\Program Files (x86)\SEGGER"));
    dirs
}

/// Add dir to HKLM system PATH via UAC-elevated PowerShell.
/// Uses `DoNotExpandEnvironmentNames` to preserve `REG_EXPAND_SZ` type.
pub fn add_to_system_path(dir: &Path) -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let dir_str = dir.to_string_lossy().to_string();
    let reg_path = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

    // Check if already present
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey_with_flags(reg_path, KEY_READ) {
        if let Ok(current) = key.get_value::<String, _>("Path") {
            if current.to_lowercase().contains(&dir_str.to_lowercase()) {
                log::info!("[platform] Already in system PATH: {}", dir_str);
                return true;
            }
            log::info!("[platform] Current HKLM PATH has {} entries",
                current.split(';').filter(|s| !s.is_empty()).count());
        }
    }

    log::info!("[platform] Updating system PATH via UAC...");
    add_via_elevated_ps(&dir_str)
}

fn add_via_elevated_ps(dir_str: &str) -> bool {
    let script_path = std::env::temp_dir().join("jlink_path_update.ps1");

    // Read raw PATH preserving REG_EXPAND_SZ, append dir, write back
    let script = format!(r#"
$reg = 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
$raw = (Get-Item -LiteralPath $reg).GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
if ($null -eq $raw -or $raw.Length -eq 0) {{ Write-Host 'ERROR: empty PATH'; exit 1 }}
$dir = '{dir}'
if ($raw -notlike "*$dir*") {{
    $new = $raw.TrimEnd(';') + ';' + $dir
    Set-ItemProperty -LiteralPath $reg -Name 'Path' -Value $new
    $r = [UIntPtr]::Zero
    Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class B{{[DllImport("user32.dll")]public static extern IntPtr SendMessageTimeout(IntPtr h,uint m,UIntPtr w,string l,uint f,uint t,out UIntPtr r);}}'
    [B]::SendMessageTimeout([IntPtr]0xFFFF,0x1A,[UIntPtr]::Zero,'Environment',2,5000,[ref]$r)
    Write-Host 'Updated'
}} else {{ Write-Host 'AlreadyPresent' }}
"#, dir = dir_str);

    if std::fs::write(&script_path, script.as_bytes()).is_err() {
        return false;
    }

    let script_str = script_path.to_string_lossy().to_string();
    let ps_cmd = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -NonInteractive -ExecutionPolicy Bypass -File \"{}\"' -Verb RunAs -Wait",
        script_str
    );

    let result = std::process::Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
        .status();

    let _ = std::fs::remove_file(&script_path);

    match result {
        Ok(s) if s.success() => {
            log::info!("[platform] System PATH updated via UAC: {}", dir_str);
            true
        }
        _ => {
            log::warn!("[platform] UAC denied or failed");
            false
        }
    }
}