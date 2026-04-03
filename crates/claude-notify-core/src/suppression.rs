// claude-notify-core: suppression engine

use crate::config::{SuppressionConfig, SuppressionFilter};
use crate::state::{current_unix_secs, SessionState};
use crate::types::Status;

// ─── SuppressionEngine ────────────────────────────────────────────────────────

/// Decides whether a notification should be suppressed based on configuration,
/// session history, and optional cooldown bypass.
pub struct SuppressionEngine<'a> {
    config: &'a SuppressionConfig,
}

impl<'a> SuppressionEngine<'a> {
    /// Create a new engine borrowing the given config.
    pub fn new(config: &'a SuppressionConfig) -> Self {
        Self { config }
    }

    /// Check whether the notification should be suppressed.
    ///
    /// Returns `Some(reason)` if the notification should be suppressed, or
    /// `None` if it should be allowed through.
    ///
    /// Evaluation order:
    /// 1. Filters (always applied, even when `bypass_cooldown` is true).
    /// 2. If `bypass_cooldown` is true, return `None` immediately.
    /// 3. Task-to-question cascade cooldown.
    /// 4. General cooldown (Question only).
    /// 5. Content deduplication.
    pub fn check(
        &self,
        status: &Status,
        content: &str,
        state: &SessionState,
        bypass_cooldown: bool,
    ) -> Option<String> {
        // 1. Filters — always checked
        if let Some(reason) = self.check_filters(status) {
            return Some(reason);
        }

        // 2. Bypass flag skips all time-based checks
        if bypass_cooldown {
            return None;
        }

        let now = current_unix_secs();

        // 3. Task-to-question cascade cooldown
        if *status == Status::Question {
            if let Some(task_time) = state.last_task_complete_time {
                let elapsed = now.saturating_sub(task_time);
                if elapsed < self.config.task_to_question_cooldown {
                    return Some(format!(
                        "task-to-question cascade cooldown ({elapsed}s < {}s)",
                        self.config.task_to_question_cooldown
                    ));
                }
            }
        }

        // 4. General cooldown (Question only)
        if *status == Status::Question {
            if let Some(last_time) = state.last_notification_time {
                let elapsed = now.saturating_sub(last_time);
                if elapsed < self.config.cooldown_seconds {
                    return Some(format!(
                        "cooldown ({elapsed}s < {}s)",
                        self.config.cooldown_seconds
                    ));
                }
            }
        }

        // 5. Content deduplication
        if let Some(last_content) = &state.last_notification_content {
            if last_content == content {
                if let Some(last_time) = state.last_notification_time {
                    let elapsed = now.saturating_sub(last_time);
                    if elapsed < self.config.content_dedup_seconds {
                        return Some(format!(
                            "duplicate content within dedup window ({elapsed}s < {}s)",
                            self.config.content_dedup_seconds
                        ));
                    }
                }
            }
        }

        None
    }

    /// Check whether any filter rule matches the given status.
    ///
    /// Note: `git_branch` and `folder` filters require runtime context that is
    /// not available here, so they are ignored at this layer.
    pub fn check_filters(&self, status: &Status) -> Option<String> {
        for filter in &self.config.filters {
            if filter_matches_status(filter, status) {
                return Some(format!("filter rule matched status '{}'", status.as_str()));
            }
        }
        None
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn filter_matches_status(filter: &SuppressionFilter, status: &Status) -> bool {
    if let Some(ref filter_status) = filter.status {
        filter_status == status.as_str()
    } else {
        false
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SuppressionConfig, SuppressionFilter};
    use crate::state::SessionState;
    use crate::types::Status;

    fn default_engine() -> SuppressionConfig {
        SuppressionConfig {
            cooldown_seconds: 7,
            task_to_question_cooldown: 12,
            content_dedup_seconds: 60,
            filters: vec![],
        }
    }

    // Helper: build a state whose timestamps appear N seconds ago
    fn state_with_task_complete_ago(secs_ago: u64) -> SessionState {
        let past = current_unix_secs().saturating_sub(secs_ago);
        SessionState {
            last_notification_time: Some(past),
            last_notification_status: Some("task_complete".to_string()),
            last_notification_content: Some("done".to_string()),
            last_task_complete_time: Some(past),
        }
    }

    #[test]
    fn no_suppression_on_first_notification() {
        let cfg = default_engine();
        let engine = SuppressionEngine::new(&cfg);
        let state = SessionState::new();

        let result = engine.check(&Status::Question, "hello?", &state, false);
        assert!(
            result.is_none(),
            "first notification should not be suppressed"
        );
    }

    #[test]
    fn cooldown_suppresses_within_window() {
        let cfg = default_engine(); // task_to_question_cooldown = 12s
        let engine = SuppressionEngine::new(&cfg);
        // task_complete happened 5 seconds ago — within the 12s window
        let state = state_with_task_complete_ago(5);

        let result = engine.check(&Status::Question, "follow-up?", &state, false);
        assert!(
            result.is_some(),
            "should suppress: task_complete 5s ago, cooldown is 12s"
        );
        assert!(result.unwrap().contains("cascade cooldown"));
    }

    #[test]
    fn cooldown_allows_after_window() {
        let cfg = default_engine(); // task_to_question_cooldown = 12s
        let engine = SuppressionEngine::new(&cfg);
        // task_complete happened 20 seconds ago — outside the 12s window
        let state = state_with_task_complete_ago(20);

        let result = engine.check(&Status::Question, "follow-up?", &state, false);
        assert!(
            result.is_none(),
            "should allow: task_complete 20s ago, cooldown is 12s"
        );
    }

    #[test]
    fn content_dedup_suppresses_same_content() {
        let cfg = default_engine(); // content_dedup_seconds = 60s
        let engine = SuppressionEngine::new(&cfg);
        // Last notification 10s ago with the same content
        let past = current_unix_secs().saturating_sub(10);
        let state = SessionState {
            last_notification_time: Some(past),
            last_notification_status: Some("question".to_string()),
            last_notification_content: Some("same content".to_string()),
            last_task_complete_time: None,
        };

        let result = engine.check(&Status::TaskComplete, "same content", &state, false);
        assert!(
            result.is_some(),
            "same content within 60s should be suppressed"
        );
        assert!(result.unwrap().contains("duplicate content"));
    }

    #[test]
    fn content_dedup_allows_different_content() {
        let cfg = default_engine();
        let engine = SuppressionEngine::new(&cfg);
        let past = current_unix_secs().saturating_sub(10);
        let state = SessionState {
            last_notification_time: Some(past),
            last_notification_status: Some("task_complete".to_string()),
            last_notification_content: Some("original content".to_string()),
            last_task_complete_time: Some(past),
        };

        let result = engine.check(&Status::TaskComplete, "different content", &state, false);
        assert!(
            result.is_none(),
            "different content should not be suppressed"
        );
    }

    #[test]
    fn bypass_cooldown_ignores_suppression() {
        let cfg = default_engine(); // task_to_question_cooldown = 12s
        let engine = SuppressionEngine::new(&cfg);
        // Would normally suppress (task_complete 2s ago)
        let state = state_with_task_complete_ago(2);

        let result = engine.check(&Status::Question, "urgent?", &state, true);
        assert!(
            result.is_none(),
            "bypass_cooldown=true should skip time-based checks"
        );
    }

    #[test]
    fn filter_suppresses_matching_status() {
        let mut cfg = default_engine();
        cfg.filters.push(SuppressionFilter {
            status: Some("question".to_string()),
            git_branch: None,
            folder: None,
        });
        let engine = SuppressionEngine::new(&cfg);
        let state = SessionState::new();

        let result = engine.check(&Status::Question, "anything", &state, false);
        assert!(
            result.is_some(),
            "filter matching status 'question' should suppress"
        );
        assert!(result.unwrap().contains("filter rule matched"));
    }

    #[test]
    fn filter_does_not_suppress_non_matching_status() {
        let mut cfg = default_engine();
        cfg.filters.push(SuppressionFilter {
            status: Some("question".to_string()),
            git_branch: None,
            folder: None,
        });
        let engine = SuppressionEngine::new(&cfg);
        let state = SessionState::new();

        let result = engine.check(&Status::TaskComplete, "done", &state, false);
        assert!(
            result.is_none(),
            "filter for 'question' should not suppress 'task_complete'"
        );
    }

    #[test]
    fn bypass_still_applies_filters() {
        let mut cfg = default_engine();
        cfg.filters.push(SuppressionFilter {
            status: Some("question".to_string()),
            git_branch: None,
            folder: None,
        });
        let engine = SuppressionEngine::new(&cfg);
        let state = SessionState::new();

        // Even with bypass_cooldown=true, filter should still fire
        let result = engine.check(&Status::Question, "test", &state, true);
        assert!(
            result.is_some(),
            "filters should apply even when bypass_cooldown=true"
        );
    }
}
