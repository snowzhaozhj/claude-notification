// claude-notify-dispatch: notification routing and audio dispatch

pub mod traits;
pub mod desktop;
pub mod sound;
pub mod terminal_bell;
pub mod webhook;

pub use traits::Dispatcher;
pub use desktop::DesktopDispatcher;
pub use sound::SoundDispatcher;
pub use terminal_bell::TerminalBellDispatcher;
pub use webhook::{WebhookDispatcher, WebhookPreset};

/// Aggregated result from dispatching to multiple channels.
pub struct DispatchReport {
    pub successes: usize,
    pub failures: usize,
    pub errors: Vec<String>,
}

/// Routes a notification to one or more dispatchers.
pub struct NotifyRouter;

impl NotifyRouter {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch_to(
        &self,
        dispatchers: &[&dyn Dispatcher],
        title: &str,
        body: &str,
    ) -> DispatchReport {
        let mut report = DispatchReport {
            successes: 0,
            failures: 0,
            errors: Vec::new(),
        };

        for dispatcher in dispatchers {
            match dispatcher.dispatch(title, body) {
                Ok(()) => {
                    report.successes += 1;
                }
                Err(e) => {
                    report.failures += 1;
                    report.errors.push(e.clone());
                    tracing::warn!("Dispatcher failed: {}", e);
                }
            }
        }

        report
    }
}

impl Default for NotifyRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    struct FakeDispatcher {
        should_fail: bool,
        was_called: Arc<AtomicBool>,
    }

    impl FakeDispatcher {
        fn new(should_fail: bool) -> Self {
            Self {
                should_fail,
                was_called: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl Dispatcher for FakeDispatcher {
        fn dispatch(&self, _title: &str, _body: &str) -> Result<(), String> {
            self.was_called.store(true, Ordering::SeqCst);
            if self.should_fail {
                Err("intentional failure".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn router_dispatches_to_all_channels() {
        let router = NotifyRouter::new();
        let d1 = FakeDispatcher::new(false);
        let d2 = FakeDispatcher::new(false);
        let dispatchers: &[&dyn Dispatcher] = &[&d1, &d2];

        let report = router.dispatch_to(dispatchers, "Test Title", "Test body");

        assert_eq!(report.successes, 2);
        assert_eq!(report.failures, 0);
        assert!(report.errors.is_empty());
        assert!(d1.was_called.load(Ordering::SeqCst));
        assert!(d2.was_called.load(Ordering::SeqCst));
    }

    #[test]
    fn router_continues_on_failure() {
        let router = NotifyRouter::new();
        let d1 = FakeDispatcher::new(true);  // fails
        let d2 = FakeDispatcher::new(false); // succeeds
        let dispatchers: &[&dyn Dispatcher] = &[&d1, &d2];

        let report = router.dispatch_to(dispatchers, "Test Title", "Test body");

        assert_eq!(report.successes, 1);
        assert_eq!(report.failures, 1);
        assert_eq!(report.errors.len(), 1);
        assert!(d1.was_called.load(Ordering::SeqCst));
        assert!(d2.was_called.load(Ordering::SeqCst));
    }
}
