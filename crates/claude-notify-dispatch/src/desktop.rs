use crate::traits::Dispatcher;
use claude_notify_platform::DesktopNotifier;

pub struct DesktopDispatcher {
    notifier: Box<dyn DesktopNotifier>,
    icon: Option<String>,
    timeout_ms: Option<u32>,
}

impl DesktopDispatcher {
    pub fn new(notifier: Box<dyn DesktopNotifier>, icon: Option<String>, timeout_ms: Option<u32>) -> Self {
        Self { notifier, icon, timeout_ms }
    }
}

impl Dispatcher for DesktopDispatcher {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String> {
        let icon_path = self.icon.as_deref().map(std::path::Path::new);
        let timeout_secs = self.timeout_ms.map(|ms| (ms / 1000) as u64);
        self.notifier.send(title, body, None, icon_path, timeout_secs)
    }
}
