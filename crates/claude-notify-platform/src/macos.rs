#![cfg(target_os = "macos")]

use crate::{DesktopNotifier, UserActivityDetector};
use std::process::Command;

// ── Notifier ─────────────────────────────────────────────────────────────────

/// macOS desktop notifier backed by `mac-notification-sys`.
pub struct MacNotifier;

impl DesktopNotifier for MacNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        mac_notification_sys::send_notification(title, subtitle, body, None)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

// ── Activity Detector ─────────────────────────────────────────────────────────

/// macOS activity detector.
///
/// - Uses `ioreg` to query HID idle time.
/// - Uses `osascript` to check the frontmost application name.
pub struct MacActivityDetector;

impl UserActivityDetector for MacActivityDetector {
    fn idle_seconds(&self) -> u64 {
        // `ioreg` reports idle time in nanoseconds under `HIDIdleTime`.
        let output = Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4"])
            .output();

        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if line.contains("HIDIdleTime") {
                        // Line looks like: "HIDIdleTime" = 12345678900
                        if let Some(eq_pos) = line.rfind('=') {
                            let value_str = line[eq_pos + 1..].trim();
                            if let Ok(nanos) = value_str.parse::<u64>() {
                                return nanos / 1_000_000_000;
                            }
                        }
                    }
                }
                0
            }
            Err(_) => 0,
        }
    }

    fn is_terminal_focused(&self) -> bool {
        // Ask macOS for the frontmost application's name via osascript.
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get name of first process \
                 where it is frontmost",
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let name = String::from_utf8_lossy(&out.stdout).to_lowercase();
                let name = name.trim();
                // Common terminal emulators on macOS.
                matches!(
                    name,
                    "terminal"
                        | "iterm2"
                        | "iterm"
                        | "alacritty"
                        | "kitty"
                        | "warp"
                        | "hyper"
                        | "ghostty"
                )
            }
            _ => false,
        }
    }
}
