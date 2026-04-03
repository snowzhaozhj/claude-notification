#![cfg(target_os = "linux")]

use crate::{DesktopNotifier, UserActivityDetector};
use std::process::Command;

// ── Notifier ─────────────────────────────────────────────────────────────────

/// Linux desktop notifier backed by `notify-rust`.
pub struct LinuxNotifier;

impl DesktopNotifier for LinuxNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        timeout: Option<u64>,
    ) -> Result<(), String> {
        use notify_rust::Notification;

        let full_body = match subtitle {
            Some(s) => format!("{s}\n{body}"),
            None => body.to_string(),
        };

        let mut n = Notification::new();
        n.summary(title).body(&full_body);

        if let Some(secs) = timeout {
            use notify_rust::Timeout;
            n.timeout(Timeout::Milliseconds((secs * 1000) as u32));
        }

        n.show().map(|_| ()).map_err(|e| e.to_string())
    }
}

// ── Activity Detector ─────────────────────────────────────────────────────────

/// Linux activity detector.
///
/// - Uses `xprintidle` for idle time (returns milliseconds).
/// - Uses `xdotool` for the active window name.
pub struct LinuxActivityDetector;

impl UserActivityDetector for LinuxActivityDetector {
    fn idle_seconds(&self) -> u64 {
        let output = Command::new("xprintidle").output();
        match output {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout);
                text.trim()
                    .parse::<u64>()
                    .map(|ms| ms / 1000)
                    .unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn is_terminal_focused(&self) -> bool {
        // xdotool getactivewindow getwindowname
        let output = Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let name = String::from_utf8_lossy(&out.stdout).to_lowercase();
                let name = name.trim();
                name.contains("terminal")
                    || name.contains("konsole")
                    || name.contains("gnome-terminal")
                    || name.contains("xterm")
                    || name.contains("alacritty")
                    || name.contains("kitty")
                    || name.contains("tilix")
            }
            _ => false,
        }
    }
}
