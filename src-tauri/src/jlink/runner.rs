//! Low-level JLink CLI execution helper.

use std::io::Write;
use std::process::{Command, Stdio};
use crate::error::{AppError, AppResult};
use crate::process::NoWindow;

/// Execute JLink with given stdin input, return (stdout, stderr).
pub fn run(bin: &str, input: &str) -> AppResult<(String, String)> {
    log::debug!("[jlink] Running: {} -NoGUI 1", bin);
    log::debug!("[jlink] Input:\n{}", input);

    let mut child = Command::new(bin)
        .args(["-NoGUI", "1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .no_window()
        .spawn()
        .map_err(|e| AppError::JLinkNotFound(e.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes())
            .map_err(|e| AppError::JLinkFailed(e.to_string()))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| AppError::JLinkFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    log::debug!("[jlink] stdout: {}", &stdout[..stdout.len().min(500)]);
    if !stderr.is_empty() {
        log::debug!("[jlink] stderr: {}", &stderr[..stderr.len().min(200)]);
    }

    Ok((stdout, stderr))
}

/// Parse SEGGER J-Link Commander version from banner output.
pub fn parse_version(output: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains("SEGGER J-Link Commander") {
            if let Some(v) = line.split('V').nth(1) {
                let ver = v.split_whitespace().next().unwrap_or("").to_string();
                if !ver.is_empty() {
                    return Some(format!("SEGGER J-Link Commander V{}", ver));
                }
            }
        }
    }
    None
}