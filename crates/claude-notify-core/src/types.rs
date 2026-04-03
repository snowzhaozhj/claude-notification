use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

// ─── Status ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    TaskComplete,
    ReviewComplete,
    Question,
    PlanReady,
    SessionLimit,
    ApiError,
    ApiOverloaded,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Status::TaskComplete => "task_complete",
            Status::ReviewComplete => "review_complete",
            Status::Question => "question",
            Status::PlanReady => "plan_ready",
            Status::SessionLimit => "session_limit",
            Status::ApiError => "api_error",
            Status::ApiOverloaded => "api_overloaded",
        }
    }

    pub fn default_title(&self) -> &'static str {
        match self {
            Status::TaskComplete => "Task Complete",
            Status::ReviewComplete => "Review Complete",
            Status::Question => "Question",
            Status::PlanReady => "Plan Ready",
            Status::SessionLimit => "Session Limit",
            Status::ApiError => "API Error",
            Status::ApiOverloaded => "API Overloaded",
        }
    }

    pub fn default_icon(&self) -> &'static str {
        match self {
            Status::TaskComplete => "✅",
            Status::ReviewComplete => "👀",
            Status::Question => "❓",
            Status::PlanReady => "📋",
            Status::SessionLimit => "⏱️",
            Status::ApiError => "❌",
            Status::ApiOverloaded => "🔥",
        }
    }
}

// ─── Priority ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low = 0,
    Normal = 1,
    Urgent = 2,
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

// ─── Channel ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Desktop,
    Sound,
    TerminalBell,
    Webhook,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseChannelError(String);

impl std::fmt::Display for ParseChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown channel: {}", self.0)
    }
}

impl std::error::Error for ParseChannelError {}

impl FromStr for Channel {
    type Err = ParseChannelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "desktop" => Ok(Channel::Desktop),
            "sound" => Ok(Channel::Sound),
            "terminal_bell" | "terminalbell" | "bell" => Ok(Channel::TerminalBell),
            "webhook" => Ok(Channel::Webhook),
            other => Err(ParseChannelError(other.to_string())),
        }
    }
}

// ─── ClickAction ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ClickAction {
    FocusTerminal { bundle_id: String },
    RunCommand { command: String },
}

// ─── Notification ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub subtitle: Option<String>,
    pub icon: Option<PathBuf>,
    pub priority: Priority,
    pub click_action: Option<ClickAction>,
    pub thread_id: Option<String>,
    pub timeout: Option<u64>,
}

impl Notification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            subtitle: None,
            icon: None,
            priority: Priority::Normal,
            click_action: None,
            thread_id: None,
            timeout: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }
}

// ─── NotifyEvent ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotifyEvent {
    pub status: Status,
    pub priority: Priority,
    pub notification: Notification,
    pub session_id: Option<String>,
}

// ─── Decision ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Decision {
    Notify {
        channels: Vec<Channel>,
        priority: Priority,
        notification: Notification,
    },
    Suppress {
        reason: String,
    },
    Downgrade {
        from: Priority,
        to: Priority,
        reason: String,
        channels: Vec<Channel>,
        notification: Notification,
    },
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display() {
        assert_eq!(Status::TaskComplete.as_str(), "task_complete");
        assert_eq!(Status::ReviewComplete.as_str(), "review_complete");
        assert_eq!(Status::Question.as_str(), "question");
        assert_eq!(Status::PlanReady.as_str(), "plan_ready");
        assert_eq!(Status::SessionLimit.as_str(), "session_limit");
        assert_eq!(Status::ApiError.as_str(), "api_error");
        assert_eq!(Status::ApiOverloaded.as_str(), "api_overloaded");
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Urgent > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
        assert!(Priority::Urgent > Priority::Low);
        assert!(Priority::Low < Priority::Urgent);
        assert_eq!(Priority::Normal, Priority::Normal);
    }

    #[test]
    fn channel_from_str() {
        assert_eq!("desktop".parse::<Channel>().unwrap(), Channel::Desktop);
        assert_eq!("sound".parse::<Channel>().unwrap(), Channel::Sound);
        assert_eq!(
            "terminal_bell".parse::<Channel>().unwrap(),
            Channel::TerminalBell
        );
        assert_eq!(
            "terminalbell".parse::<Channel>().unwrap(),
            Channel::TerminalBell
        );
        assert_eq!("bell".parse::<Channel>().unwrap(), Channel::TerminalBell);
        assert_eq!("webhook".parse::<Channel>().unwrap(), Channel::Webhook);

        // Invalid input
        assert!("invalid_channel".parse::<Channel>().is_err());
        assert!("".parse::<Channel>().is_err());
    }

    #[test]
    fn notification_builder() {
        let n = Notification::new("Hello", "World");
        assert_eq!(n.title, "Hello");
        assert_eq!(n.body, "World");
        assert_eq!(n.priority, Priority::Normal);
        assert!(n.subtitle.is_none());
        assert!(n.thread_id.is_none());

        let n2 = n
            .with_subtitle("Sub")
            .with_priority(Priority::Urgent)
            .with_thread_id("thread-1");
        assert_eq!(n2.subtitle.as_deref(), Some("Sub"));
        assert_eq!(n2.priority, Priority::Urgent);
        assert_eq!(n2.thread_id.as_deref(), Some("thread-1"));
    }
}
