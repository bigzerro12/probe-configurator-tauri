//! JLink CLI command script builders.
//!
//! All JLink stdin command sequences are defined here.
//! This makes it easy to track, modify, and test command flows in one place.
//!
//! JLink Commander accepts commands via stdin when launched with `-NoGUI 1`.
//! Commands are newline-separated. `exit` terminates the session.

// ─── Detection ────────────────────────────────────────────────────────────────

/// Probe version banner — used to detect JLink and parse version string.
pub fn detect() -> &'static str {
    "\n\nExit\n"
}

// ─── Scan ─────────────────────────────────────────────────────────────────────

/// List all connected probes.
pub fn show_emu_list() -> &'static str {
    "ShowEmuList\nExit\n"
}

/// Fetch firmware dates for N probes by selecting each one by index.
pub fn fetch_firmware_dates(count: usize) -> String {
    let mut s = String::from("exec DisableAutoUpdateFW\n");
    for i in 0..count {
        s.push_str(&format!("selectprobe\n{}\n", i));
    }
    s.push_str("exit\n");
    s
}

// ─── Nickname ─────────────────────────────────────────────────────────────────

/// Session 1: select probe and write nickname, then exit.
///
/// Must be a separate session from reboot — when piping stdin (non-interactive),
/// reboot in the same session as setnickname exits before the probe completes
/// its power-cycle, causing the nickname to not apply.
///
/// Expected output: "was set" / "was unset"
/// Error output:    "is not a valid nickname"
pub fn set_nickname_write(probe_index: usize, nickname: &str) -> String {
    let nickname = nickname.trim();
    let set_cmd = if nickname.is_empty() {
        "setnickname ".to_string()
    } else {
        format!("setnickname {}", nickname)
    };
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\n{}\nsleep 100\nexit\n",
        probe_index, set_cmd
    )
}

/// Session 2: select probe and reboot so nickname takes effect.
///
/// Run this in a fresh JLink session after set_nickname_write completes.
/// A fresh session ensures JLink waits for the full reboot cycle.
///
/// Expected output: "Rebooted successfully"
pub fn set_nickname_reboot(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nreboot\nsleep 100\nexit\n",
        probe_index
    )
}

// ─── Firmware ─────────────────────────────────────────────────────────────────

/// Trigger firmware update for a probe at the given index.
/// `exec EnableAutoUpdateFW` tells JLink to check and flash if newer firmware is available.
///
/// Expected output if updated: "New firmware booted successfully"
/// Expected output if current: contains "Firmware: ... compiled ..."
pub fn update_firmware(probe_index: usize) -> String {
    format!(
        "exec EnableAutoUpdateFW\nselectprobe\n{}\nexit\n",
        probe_index
    )
}

// ─── USB Driver ───────────────────────────────────────────────────────────────

/// Session 1: switch probe to WinUSB driver via WebUSBEnable.
///
/// Must be followed by set_usb_driver_reboot in a separate session.
/// Expected output: "Probe configured successfully."
pub fn set_usb_driver_webusb(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nWebUSBEnable\nsleep 100\nexit\n",
        probe_index
    )
}

/// Session 1: switch probe back to SEGGER USB driver via WebUSBDisable.
///
/// Must be followed by set_usb_driver_reboot in a separate session.
/// Expected output: "Probe configured successfully."
pub fn set_usb_driver_segger(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nWebUSBDisable\nsleep 100\nexit\n",
        probe_index
    )
}

/// Session 1 fallback: switch probe to WinUSB driver via WinUSBEnable.
///
/// Used when WebUSBEnable is not supported by the installed J-Link version.
pub fn set_usb_driver_winusb_enable(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nWinUSBEnable\nsleep 100\nexit\n",
        probe_index
    )
}

/// Session 1 fallback: switch probe back to SEGGER USB driver via WinUSBDisable.
///
/// Used when WebUSBDisable is not supported by the installed J-Link version.
pub fn set_usb_driver_winusb_disable(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nWinUSBDisable\nsleep 100\nexit\n",
        probe_index
    )
}

/// Session 2: reboot probe so USB driver change takes effect.
///
/// Same as set_nickname_reboot — must be a fresh session.
pub fn set_usb_driver_reboot(probe_index: usize) -> String {
    format!(
        "exec DisableAutoUpdateFW\nselectprobe\n{}\nreboot\nsleep 100\nexit\n",
        probe_index
    )
}