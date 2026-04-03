use crate::{DesktopNotifier, UserActivityDetector};
use std::path::{Path, PathBuf};
use std::process::Command;

// ── Notifier ─────────────────────────────────────────────────────────────────

/// macOS desktop notifier.
///
/// Prefers the bundled ClaudeNotifier.app (native UNUserNotificationCenter
/// with custom icon and click-to-focus). Falls back to osascript if the
/// app is not available.
pub struct MacNotifier {
    /// Path to ClaudeNotifier.app (set from CLAUDE_PLUGIN_ROOT).
    app_path: Option<PathBuf>,
}

impl Default for MacNotifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MacNotifier {
    pub fn new() -> Self {
        // Look for ClaudeNotifier.app relative to CLAUDE_PLUGIN_ROOT
        let app_path = std::env::var("CLAUDE_PLUGIN_ROOT")
            .ok()
            .map(|root| {
                Path::new(&root)
                    .join("swift-notifier")
                    .join("ClaudeNotifier.app")
            })
            .filter(|p| p.join("Contents/MacOS/ClaudeNotifier").exists());

        Self { app_path }
    }
}

impl DesktopNotifier for MacNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        _icon: Option<&Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        if let Some(app) = &self.app_path {
            send_via_app(app, title, body, subtitle)
        } else {
            send_via_osascript(title, body, subtitle)
        }
    }
}

/// Send notification via ClaudeNotifier.app (preferred).
fn send_via_app(
    app_path: &Path,
    title: &str,
    body: &str,
    subtitle: Option<&str>,
) -> Result<(), String> {
    let mut args = vec![
        "-W".to_string(),
        "-n".to_string(),
        app_path.to_string_lossy().to_string(),
        "--args".to_string(),
        "-title".to_string(),
        title.to_string(),
        "-message".to_string(),
        body.to_string(),
    ];

    if let Some(sub) = subtitle {
        args.push("-subtitle".to_string());
        args.push(sub.to_string());
    }

    // Pass session ID as group for thread grouping
    if let Ok(session) = std::env::var("CLAUDE_SESSION_ID") {
        args.push("-group".to_string());
        args.push(session);
    }

    let output = Command::new("open")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to launch ClaudeNotifier.app: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        // Fallback to osascript if app launch fails
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("ClaudeNotifier.app failed ({stderr}), falling back to osascript");
        send_via_osascript(title, body, subtitle)
    }
}

/// Fallback: send via osascript (shows Script Editor icon).
fn send_via_osascript(title: &str, body: &str, subtitle: Option<&str>) -> Result<(), String> {
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
        .map_err(|e| format!("failed to run osascript: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("osascript failed: {}", stderr.trim()))
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
