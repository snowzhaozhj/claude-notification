// claude-notify-core: decision engine

use crate::config::Config;
use crate::priority::PriorityEngine;
use crate::state::SessionState;
use crate::suppression::SuppressionEngine;
use crate::types::*;

// ─── UserActivity trait ───────────────────────────────────────────────────────

/// Abstracts user activity detection so the core crate stays platform-agnostic.
pub trait UserActivity {
    /// Seconds since the last observed user input.
    fn idle_seconds(&self) -> u64;
    /// Whether the Claude terminal window is the focused application.
    fn is_terminal_focused(&self) -> bool;
}

// ─── DecisionEngine ───────────────────────────────────────────────────────────

pub struct DecisionEngine<'a> {
    config: &'a Config,
    priority_engine: &'a PriorityEngine,
}

impl<'a> DecisionEngine<'a> {
    pub fn new(config: &'a Config, priority_engine: &'a PriorityEngine) -> Self {
        Self {
            config,
            priority_engine,
        }
    }

    /// Main decision method.
    ///
    /// Steps:
    /// 1. Assess priority via `priority_engine`.
    /// 2. Build a `Notification` from status metadata and `summary`.
    /// 3. Check suppression (bypassed when priority is Urgent).
    /// 4. If not suppressed, check user activity for a potential downgrade:
    ///    - activity enabled AND priority doesn't bypass idle check AND terminal
    ///      focused AND user is NOT idle → Downgrade to Low with TerminalBell
    ///      (or Desktop if bell is disabled).
    /// 5. Otherwise → Notify with channels from `priority_engine`.
    pub fn decide(
        &self,
        status: Status,
        summary: &str,
        activity: &dyn UserActivity,
        state: &SessionState,
    ) -> Decision {
        // 1. Priority
        let priority = self.priority_engine.assess(&status);

        // 2. Notification
        let title = format!("{} {}", status.default_icon(), status.default_title());
        let notification = Notification::new(title, summary).with_priority(priority);

        // 3. Suppression
        let bypass_cooldown = self.priority_engine.bypasses_cooldown(&priority);
        let suppression_engine = SuppressionEngine::new(&self.config.suppression);
        if let Some(reason) = suppression_engine.check(&status, summary, state, bypass_cooldown) {
            return Decision::Suppress { reason };
        }

        // 4. Activity / focus downgrade
        let activity_cfg = &self.config.activity;
        let user_is_idle = activity.idle_seconds() >= activity_cfg.idle_threshold_seconds;
        let bypasses_idle = self.priority_engine.bypasses_idle_check(&priority);

        if activity_cfg.enabled
            && activity_cfg.suppress_when_focused
            && !bypasses_idle
            && activity.is_terminal_focused()
            && !user_is_idle
        {
            // Downgrade: user is actively watching the terminal
            let downgrade_channels = if self.config.terminal_bell.enabled {
                vec![Channel::TerminalBell]
            } else {
                vec![Channel::Desktop]
            };

            let downgraded_notification = notification.with_priority(Priority::Low);

            return Decision::Downgrade {
                from: priority,
                to: Priority::Low,
                reason: "terminal focused and user not idle".to_string(),
                channels: downgrade_channels,
                notification: downgraded_notification,
            };
        }

        // 5. Normal notify
        let channels = self.priority_engine.channels_for(&priority);
        Decision::Notify {
            channels,
            priority,
            notification,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::current_unix_secs;
    use std::collections::HashMap;

    // ── Mock ──────────────────────────────────────────────────────────────────

    struct MockActivity {
        idle_seconds: u64,
        terminal_focused: bool,
    }

    impl UserActivity for MockActivity {
        fn idle_seconds(&self) -> u64 {
            self.idle_seconds
        }
        fn is_terminal_focused(&self) -> bool {
            self.terminal_focused
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn default_engine_pair() -> (Config, PriorityEngine) {
        let config = Config::default();
        let engine = PriorityEngine::new(HashMap::new(), HashMap::new());
        (config, engine)
    }

    fn empty_state() -> SessionState {
        SessionState::new()
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// User idle (60s > 30s threshold) and terminal not focused → Notify with
    /// Normal priority (TaskComplete defaults to Normal).
    #[test]
    fn notify_when_user_idle() {
        let (config, priority_engine) = default_engine_pair();
        let engine = DecisionEngine::new(&config, &priority_engine);

        let activity = MockActivity {
            idle_seconds: 60,
            terminal_focused: false,
        };

        let decision = engine.decide(Status::TaskComplete, "all done", &activity, &empty_state());

        match decision {
            Decision::Notify {
                priority, channels, ..
            } => {
                assert_eq!(priority, Priority::Normal);
                assert!(!channels.is_empty());
            }
            other => panic!("expected Notify, got {other:?}"),
        }
    }

    /// Terminal focused AND user not idle (5s < 30s threshold) → Downgrade to
    /// Low; channels must contain TerminalBell and must NOT contain Sound.
    #[test]
    fn downgrade_when_terminal_focused() {
        let (mut config, priority_engine) = default_engine_pair();
        config.activity.suppress_when_focused = true;
        let engine = DecisionEngine::new(&config, &priority_engine);

        let activity = MockActivity {
            idle_seconds: 5,
            terminal_focused: true,
        };

        let decision = engine.decide(Status::TaskComplete, "task done", &activity, &empty_state());

        match decision {
            Decision::Downgrade { to, channels, .. } => {
                assert_eq!(to, Priority::Low);
                assert!(
                    channels.contains(&Channel::TerminalBell),
                    "downgrade channels should contain TerminalBell"
                );
                assert!(
                    !channels.contains(&Channel::Sound),
                    "downgrade channels must not contain Sound"
                );
            }
            other => panic!("expected Downgrade, got {other:?}"),
        }
    }

    /// ApiError → Urgent priority; Urgent bypasses the idle/focus check.
    /// Even when terminal is focused and user is not idle, result is Notify.
    #[test]
    fn urgent_bypasses_focus_check() {
        let (config, priority_engine) = default_engine_pair();
        let engine = DecisionEngine::new(&config, &priority_engine);

        let activity = MockActivity {
            idle_seconds: 5,
            terminal_focused: true,
        };

        let decision = engine.decide(Status::ApiError, "API failure", &activity, &empty_state());

        match decision {
            Decision::Notify { priority, .. } => {
                assert_eq!(priority, Priority::Urgent);
            }
            other => panic!("expected Notify (Urgent), got {other:?}"),
        }
    }

    /// State has a recent task_complete followed by a Question event within
    /// the cascade cooldown window → Suppress.
    ///
    /// Question defaults to Urgent, which bypasses the cooldown.  To exercise
    /// the cascade-cooldown path we override Question → Normal.
    #[test]
    fn suppress_within_cooldown() {
        let config = Config::default();
        // Override Question priority to Normal so bypass_cooldown=false
        let overrides: HashMap<String, Priority> = [("question".to_string(), Priority::Normal)]
            .into_iter()
            .collect();
        let priority_engine = PriorityEngine::new(overrides, HashMap::new());
        let engine = DecisionEngine::new(&config, &priority_engine);

        // task_complete 2 seconds ago (within 12s cascade cooldown)
        let past = current_unix_secs().saturating_sub(2);
        let state = SessionState {
            last_notification_time: Some(past),
            last_notification_status: Some("task_complete".to_string()),
            last_notification_content: Some("done".to_string()),
            last_task_complete_time: Some(past),
        };

        let activity = MockActivity {
            idle_seconds: 60,
            terminal_focused: false,
        };

        let decision = engine.decide(Status::Question, "what next?", &activity, &state);

        match decision {
            Decision::Suppress { reason } => {
                assert!(
                    reason.contains("cascade cooldown"),
                    "expected cascade cooldown reason, got: {reason}"
                );
            }
            other => panic!("expected Suppress, got {other:?}"),
        }
    }

    /// When activity detection is disabled, the focus/idle check is skipped and
    /// the decision is Notify even when the terminal is focused.
    #[test]
    fn activity_disabled_skips_focus_check() {
        let mut config = Config::default();
        config.activity.enabled = false;

        let priority_engine = PriorityEngine::new(HashMap::new(), HashMap::new());
        let engine = DecisionEngine::new(&config, &priority_engine);

        let activity = MockActivity {
            idle_seconds: 5,
            terminal_focused: true,
        };

        let decision = engine.decide(Status::TaskComplete, "finished", &activity, &empty_state());

        match decision {
            Decision::Notify { .. } => {} // correct
            other => panic!("expected Notify, got {other:?}"),
        }
    }
}
