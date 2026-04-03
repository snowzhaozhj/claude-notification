/// No-op fallback implementations for platforms without native support.

use crate::{DesktopNotifier, UserActivityDetector};

/// A no-op activity detector that always reports idle_seconds=0
/// and is_terminal_focused=false.
pub struct NoopActivityDetector;

impl UserActivityDetector for NoopActivityDetector {
    fn idle_seconds(&self) -> u64 {
        0
    }

    fn is_terminal_focused(&self) -> bool {
        false
    }
}

/// A no-op notifier that silently discards every notification.
pub struct NoopNotifier;

impl DesktopNotifier for NoopNotifier {
    fn send(
        &self,
        _title: &str,
        _body: &str,
        _subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_activity_returns_defaults() {
        let detector = NoopActivityDetector;
        assert_eq!(detector.idle_seconds(), 0);
        assert!(!detector.is_terminal_focused());
    }

    #[test]
    fn noop_notifier_returns_ok() {
        let notifier = NoopNotifier;
        let result = notifier.send("title", "body", None, None, None);
        assert!(result.is_ok());
    }
}
