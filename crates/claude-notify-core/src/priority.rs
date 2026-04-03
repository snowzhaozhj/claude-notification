use std::collections::HashMap;

use crate::types::{Channel, Priority, Status};

// ─── PriorityEngine ───────────────────────────────────────────────────────────

pub struct PriorityEngine {
    /// Per-status priority overrides keyed by status.as_str().
    overrides: HashMap<String, Priority>,
    /// Per-priority channel enable/disable overrides.
    /// Outer key: priority variant name (e.g. "urgent"), inner key: channel
    /// name (e.g. "sound"), value: enabled flag.
    channel_overrides: HashMap<String, HashMap<String, bool>>,
}

impl PriorityEngine {
    pub fn new(
        overrides: HashMap<String, Priority>,
        channel_overrides: HashMap<String, HashMap<String, bool>>,
    ) -> Self {
        Self { overrides, channel_overrides }
    }

    /// Return the priority for a given status, consulting overrides first.
    pub fn assess(&self, status: &Status) -> Priority {
        if let Some(&p) = self.overrides.get(status.as_str()) {
            return p;
        }
        default_priority(status)
    }

    /// Return the list of channels for the given priority, applying any
    /// channel-level overrides.
    pub fn channels_for(&self, priority: &Priority) -> Vec<Channel> {
        let mut channels = default_channels(priority);

        let key = priority_key(priority);
        if let Some(ch_map) = self.channel_overrides.get(key) {
            for (ch_name, &enabled) in ch_map {
                let channel = channel_from_key(ch_name);
                if let Some(ch) = channel {
                    if enabled {
                        if !channels.contains(&ch) {
                            channels.push(ch);
                        }
                    } else {
                        channels.retain(|c| c != &ch);
                    }
                }
            }
        }

        channels
    }

    pub fn bypasses_idle_check(&self, priority: &Priority) -> bool {
        *priority == Priority::Urgent
    }

    pub fn bypasses_cooldown(&self, priority: &Priority) -> bool {
        *priority == Priority::Urgent
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn default_priority(status: &Status) -> Priority {
    match status {
        Status::ApiError | Status::SessionLimit | Status::Question => Priority::Urgent,
        Status::TaskComplete | Status::PlanReady | Status::ApiOverloaded => Priority::Normal,
        Status::ReviewComplete => Priority::Low,
    }
}

fn default_channels(priority: &Priority) -> Vec<Channel> {
    match priority {
        Priority::Urgent => vec![
            Channel::Desktop,
            Channel::Sound,
            Channel::TerminalBell,
            Channel::Webhook,
        ],
        Priority::Normal => vec![Channel::Desktop, Channel::Sound, Channel::Webhook],
        Priority::Low => vec![Channel::Desktop],
    }
}

fn priority_key(priority: &Priority) -> &'static str {
    match priority {
        Priority::Urgent => "urgent",
        Priority::Normal => "normal",
        Priority::Low => "low",
    }
}

fn channel_from_key(key: &str) -> Option<Channel> {
    match key.to_lowercase().as_str() {
        "desktop" => Some(Channel::Desktop),
        "sound" => Some(Channel::Sound),
        "terminal_bell" | "terminalbell" | "bell" => Some(Channel::TerminalBell),
        "webhook" => Some(Channel::Webhook),
        _ => None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_default() -> PriorityEngine {
        PriorityEngine::new(HashMap::new(), HashMap::new())
    }

    #[test]
    fn default_priorities() {
        let e = engine_default();

        assert_eq!(e.assess(&Status::ApiError),      Priority::Urgent);
        assert_eq!(e.assess(&Status::SessionLimit),  Priority::Urgent);
        assert_eq!(e.assess(&Status::Question),      Priority::Urgent);

        assert_eq!(e.assess(&Status::TaskComplete),  Priority::Normal);
        assert_eq!(e.assess(&Status::PlanReady),     Priority::Normal);
        assert_eq!(e.assess(&Status::ApiOverloaded), Priority::Normal);

        assert_eq!(e.assess(&Status::ReviewComplete), Priority::Low);
    }

    #[test]
    fn priority_override() {
        let overrides: HashMap<String, Priority> = [
            ("review_complete".to_string(), Priority::Normal),
            ("question".to_string(),        Priority::Low),
        ]
        .into_iter()
        .collect();

        let e = PriorityEngine::new(overrides, HashMap::new());

        // Overridden
        assert_eq!(e.assess(&Status::ReviewComplete), Priority::Normal);
        assert_eq!(e.assess(&Status::Question),       Priority::Low);

        // Unchanged
        assert_eq!(e.assess(&Status::ApiError),      Priority::Urgent);
        assert_eq!(e.assess(&Status::TaskComplete),  Priority::Normal);
    }

    #[test]
    fn channels_for_urgent() {
        let e = engine_default();
        let channels = e.channels_for(&Priority::Urgent);

        assert!(channels.contains(&Channel::Desktop));
        assert!(channels.contains(&Channel::Sound));
        assert!(channels.contains(&Channel::TerminalBell));
        assert!(channels.contains(&Channel::Webhook));
        assert_eq!(channels.len(), 4);
    }

    #[test]
    fn channels_for_low() {
        let e = engine_default();
        let channels = e.channels_for(&Priority::Low);

        assert_eq!(channels, vec![Channel::Desktop]);
    }

    #[test]
    fn channel_override() {
        // Disable sound for urgent.
        let ch_overrides: HashMap<String, HashMap<String, bool>> = [(
            "urgent".to_string(),
            [("sound".to_string(), false)].into_iter().collect(),
        )]
        .into_iter()
        .collect();

        let e = PriorityEngine::new(HashMap::new(), ch_overrides);
        let channels = e.channels_for(&Priority::Urgent);

        assert!(channels.contains(&Channel::Desktop));
        assert!(!channels.contains(&Channel::Sound));
        assert!(channels.contains(&Channel::TerminalBell));
        assert!(channels.contains(&Channel::Webhook));
        assert_eq!(channels.len(), 3);
    }
}
