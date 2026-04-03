//! Platform abstraction layer for desktop notifications and user-activity detection.

pub mod activity;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

// ── Traits ────────────────────────────────────────────────────────────────────

/// Trait for sending desktop notifications.
pub trait DesktopNotifier: Send + Sync {
    /// Send a notification with the given title and body.
    ///
    /// `subtitle` and `icon` are optional and may be ignored on platforms that
    /// do not support them. `timeout` is in seconds.
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        icon: Option<&std::path::Path>,
        timeout: Option<u64>,
    ) -> Result<(), String>;

    /// Whether this notifier supports a click-action callback.
    fn supports_click_action(&self) -> bool {
        false
    }
}

/// Trait for detecting user activity on the desktop.
pub trait UserActivityDetector: Send + Sync {
    /// Seconds since the last user input event (keyboard / mouse).
    fn idle_seconds(&self) -> u64;

    /// Whether a terminal emulator is the currently focused window.
    fn is_terminal_focused(&self) -> bool;
}

// ── Factory functions ─────────────────────────────────────────────────────────

/// Create the best available [`UserActivityDetector`] for the current platform.
pub fn create_activity_detector() -> Box<dyn UserActivityDetector> {
    #[cfg(target_os = "macos")]
    {
        return Box::new(macos::MacActivityDetector);
    }

    #[cfg(target_os = "linux")]
    {
        return Box::new(linux::LinuxActivityDetector);
    }

    #[cfg(target_os = "windows")]
    {
        return Box::new(windows::WindowsActivityDetector);
    }

    #[allow(unreachable_code)]
    Box::new(activity::NoopActivityDetector)
}

/// Create the best available [`DesktopNotifier`] for the current platform.
pub fn create_desktop_notifier() -> Box<dyn DesktopNotifier> {
    #[cfg(target_os = "macos")]
    {
        return Box::new(macos::MacNotifier);
    }

    #[cfg(target_os = "linux")]
    {
        return Box::new(linux::LinuxNotifier);
    }

    #[cfg(target_os = "windows")]
    {
        return Box::new(windows::WindowsNotifier);
    }

    #[allow(unreachable_code)]
    Box::new(activity::NoopNotifier)
}
