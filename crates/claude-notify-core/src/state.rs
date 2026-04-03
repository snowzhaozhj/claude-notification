// claude-notify-core: session state persistence

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::NotifyError;

// ─── SessionState ─────────────────────────────────────────────────────────────

/// Persisted state for a single Claude session, used by the suppression engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    /// Unix timestamp (seconds) of the last notification sent
    pub last_notification_time: Option<u64>,
    /// Status string of the last notification (e.g. "task_complete")
    pub last_notification_status: Option<String>,
    /// Content (body) of the last notification
    pub last_notification_content: Option<String>,
    /// Unix timestamp (seconds) of the last task_complete notification
    pub last_task_complete_time: Option<u64>,
}

impl SessionState {
    /// Create a new default (empty) state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load state from a JSON file at `path`.
    ///
    /// Returns `Default::default()` if the file is missing or cannot be parsed.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(NotifyError::Io)?;
        let state: Self = serde_json::from_str(&text).unwrap_or_default();
        Ok(state)
    }

    /// Persist state to a JSON file at `path`.  Parent directories are created
    /// automatically.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(NotifyError::Io)?;
        }
        let json = serde_json::to_string(self)?;
        std::fs::write(path, json).map_err(NotifyError::Io)?;
        Ok(())
    }

    /// Canonical path for a session's state file.
    pub fn state_path(session_id: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/claude-notify-state-{session_id}.json"))
    }

    /// Update all fields after a notification has been successfully dispatched.
    pub fn update_after_notification(&mut self, status: &str, content: &str) {
        let now = current_unix_secs();
        self.last_notification_time = Some(now);
        self.last_notification_status = Some(status.to_string());
        self.last_notification_content = Some(content.to_string());
        if status == "task_complete" {
            self.last_task_complete_time = Some(now);
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Returns the current time as Unix seconds (u64).
pub fn current_unix_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn save_and_load_state() {
        let tmp = NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_path_buf();

        let mut state = SessionState::new();
        state.update_after_notification("task_complete", "job done");

        state.save(&path).expect("save should succeed");

        let loaded = SessionState::load(&path).expect("load should succeed");

        assert!(loaded.last_notification_time.is_some());
        assert_eq!(loaded.last_notification_status.as_deref(), Some("task_complete"));
        assert_eq!(loaded.last_notification_content.as_deref(), Some("job done"));
        assert!(loaded.last_task_complete_time.is_some());
        assert_eq!(
            loaded.last_notification_time,
            loaded.last_task_complete_time,
            "task_complete should set both timestamps"
        );
    }

    #[test]
    fn load_missing_file_returns_default() {
        let path = PathBuf::from("/tmp/nonexistent-claude-notify-state-xyz.json");
        // Ensure it really doesn't exist
        let _ = std::fs::remove_file(&path);

        let state = SessionState::load(&path).expect("should return default, not error");
        assert!(state.last_notification_time.is_none());
        assert!(state.last_notification_status.is_none());
        assert!(state.last_notification_content.is_none());
        assert!(state.last_task_complete_time.is_none());
    }

    #[test]
    fn state_path_contains_session_id() {
        let path = SessionState::state_path("abc-123");
        assert_eq!(
            path,
            PathBuf::from("/tmp/claude-notify-state-abc-123.json")
        );
    }

    #[test]
    fn update_non_task_status_does_not_set_task_complete_time() {
        let mut state = SessionState::new();
        state.update_after_notification("question", "what should I do?");
        assert!(state.last_notification_time.is_some());
        assert!(state.last_task_complete_time.is_none());
    }
}
