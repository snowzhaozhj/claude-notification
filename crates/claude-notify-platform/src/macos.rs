use crate::{DesktopNotifier, UserActivityDetector};
use std::process::Command;

// ── Notifier ─────────────────────────────────────────────────────────────────

/// macOS desktop notifier using `osascript` (AppleScript).
/// More reliable than `mac-notification-sys` on modern macOS versions
/// where NSUserNotification is deprecated.
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
        let mut script = format!(
            "display notification \"{}\" with title \"{}\"",
            escape_applescript(body),
            escape_applescript(title),
        );
        if let Some(sub) = subtitle {
            script.push_str(&format!(" subtitle \"{}\"", escape_applescript(sub)));
        }

        let output = Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| format!("failed to run osascript: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("osascript failed: {}", stderr.trim()))
        }
    }
}

fn escape_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ── Activity Detector ─────────────────────────────────────────────────────────

/// macOS activity detector.
///
/// - Uses `ioreg` to query HID idle time.
/// - Uses `osascript` to check the frontmost application name.
pub struct MacActivityDetector;

impl UserActivityDetector for MacActivityDetector {
    fn idle_seconds(&self) -> u64 {
        let output = Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4"])
            .output();

        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if line.contains("HIDIdleTime") {
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
