use crate::traits::Dispatcher;

pub struct TerminalBellDispatcher;

impl TerminalBellDispatcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalBellDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher for TerminalBellDispatcher {
    fn dispatch(&self, _title: &str, _body: &str) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::io::Write;
            if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
                let _ = tty.write_all(b"\x07");
            }
        }
        #[cfg(not(unix))]
        {
            print!("\x07");
        }
        Ok(())
    }
}
