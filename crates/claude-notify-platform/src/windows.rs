use crate::{DesktopNotifier, UserActivityDetector};

// ── Notifier ─────────────────────────────────────────────────────────────────

/// Windows desktop notifier backed by `winrt-notification`.
pub struct WindowsNotifier;

impl DesktopNotifier for WindowsNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        _subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        use winrt_notification::{Duration, Toast};

        Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .duration(Duration::Short)
            .show()
            .map_err(|e| e.to_string())
    }
}

// ── Activity Detector ─────────────────────────────────────────────────────────

/// Windows activity detector (placeholder implementation).
pub struct WindowsActivityDetector;

impl UserActivityDetector for WindowsActivityDetector {
    fn idle_seconds(&self) -> u64 {
        0
    }

    fn is_terminal_focused(&self) -> bool {
        false
    }
}
