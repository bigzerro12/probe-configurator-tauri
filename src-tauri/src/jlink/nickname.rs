//! Probe nickname operations.

use crate::jlink::{runner, scan, scripts, types::NicknameResult};

pub fn set(bin: &str, probe_index: usize, nickname: &str) -> NicknameResult {
    let nickname_trimmed = nickname.trim();
    log::info!("[jlink] Setting nickname for probe[{}] to \"{}\"", probe_index, nickname_trimmed);

    // ── Session 1: write nickname ──────────────────────────────────────────────
    // setnickname and reboot must be in separate JLink sessions.
    // When piping stdin (non-interactive), running reboot in the same session as
    // setnickname causes JLink to exit before the probe completes its power-cycle,
    // so the nickname never applies.
    let write_input = scripts::set_nickname_write(probe_index, nickname_trimmed);
    let write_stdout = match runner::run(bin, &write_input) {
        Ok((stdout, _)) => stdout,
        Err(e) => {
            return NicknameResult {
                success: false,
                error: Some(e.to_string()),
                warning: None,
            };
        }
    };

    log::debug!("[jlink] set_nickname write stdout:\n{}", write_stdout);

    if write_stdout.contains("is not a valid nickname") {
        return NicknameResult {
            success: false,
            error: Some("Invalid nickname: only ASCII characters, no double quotes.".to_string()),
            warning: None,
        };
    }

    if write_failed(&write_stdout) {
        return NicknameResult {
            success: false,
            error: Some(format!(
                "J-Link rejected the nickname command: {}",
                last_non_empty_line(&write_stdout)
            )),
            warning: None,
        };
    }

    if !write_succeeded(&write_stdout) {
        return NicknameResult {
            success: false,
            error: Some(format!(
                "Unexpected response from J-Link after setting nickname: {}",
                last_non_empty_line(&write_stdout)
            )),
            warning: None,
        };
    }

    log::info!("[jlink] Probe[{}] nickname written, starting reboot session", probe_index);

    // ── Session 2: reboot probe ────────────────────────────────────────────────
    let reboot_input = scripts::set_nickname_reboot(probe_index);
    let mut reboot_unsupported = false;
    match runner::run(bin, &reboot_input) {
        Ok((stdout, _)) => {
            log::debug!("[jlink] set_nickname reboot stdout:\n{}", stdout);
            if stdout.contains("Rebooted successfully") {
                log::info!("[jlink] Probe[{}] rebooted successfully", probe_index);
            } else if stdout.contains("Command not supported by connected probe.") {
                reboot_unsupported = true;
                log::warn!("[jlink] Probe[{}] reboot command not supported", probe_index);
            } else {
                log::warn!("[jlink] Probe[{}] reboot not confirmed", probe_index);
            }
        }
        Err(e) => {
            // Reboot failed but nickname was already written — warn but don't fail.
            // User can manually re-plug the probe.
            log::warn!("[jlink] Probe[{}] reboot session failed: {}", probe_index, e);
        }
    }

    // ── Session 3: verify nickname actually took effect ────────────────────────
    // Some probe models accept setnickname but only apply after a true power-cycle.
    // Verify by rescanning, so UI can provide accurate guidance.
    let applied = match scan::scan_probes(bin) {
        Ok(probes) => {
            if let Some(probe) = probes.get(probe_index) {
                let expected = nickname_trimmed;
                let actual = probe.nick_name.trim();
                actual == expected
            } else {
                false
            }
        }
        Err(e) => {
            log::warn!("[jlink] nickname verify scan failed: {}", e);
            false
        }
    };

    if !applied {
        if reboot_unsupported {
            let warning = if nickname_trimmed.is_empty() {
                "Nickname was cleared, but this probe firmware does not support reboot via command. Unplug and replug the probe, then press Refresh list to apply the change."
                    .to_string()
            } else {
                format!(
                    "Nickname was set to '{}', but this probe firmware does not support reboot via command. Unplug and replug the probe, then press Refresh list to apply the change.",
                    nickname_trimmed
                )
            };
            return NicknameResult {
                success: true,
                error: None,
                warning: Some(warning),
            };
        }

        return NicknameResult {
            success: false,
            error: Some(
                "Nickname command was accepted, but the new name is not visible yet. Unplug and re-plug the probe, then refresh the list."
                    .to_string(),
            ),
            warning: None,
        };
    }

    NicknameResult {
        success: true,
        error: None,
        warning: None,
    }
}

fn write_succeeded(stdout: &str) -> bool {
    // J-Link wording can vary between versions. Accept multiple known success patterns.
    stdout.contains("was set")
        || stdout.contains("was unset")
        || stdout.contains("Probe configured successfully")
        || stdout.contains("Nickname set")
        || stdout.contains("Nickname unset")
}

fn write_failed(stdout: &str) -> bool {
    stdout.contains("Unknown command")
        || stdout.contains("Syntax error")
        || stdout.contains("not supported")
        || stdout.contains("No emulator connected")
        || stdout.contains("is not a valid nickname")
}

fn last_non_empty_line(stdout: &str) -> String {
    stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("(no output)")
        .trim()
        .to_string()
}