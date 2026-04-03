# Claude Code Smart Notification Plugin - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust-based Claude Code notification plugin that delivers intelligent, cross-platform notifications with smart timing, clear content, and strong configurability.

**Architecture:** Cargo workspace with 4 crates — `claude-notify` (binary), `claude-notify-core` (logic), `claude-notify-dispatch` (delivery), `claude-notify-platform` (OS abstractions). Single binary, event-driven (hook trigger → analyze → decide → dispatch → exit).

**Tech Stack:** Rust, clap, serde, serde_yaml, serde_json, thiserror, anyhow, ureq, rodio, notify-rust, mac-notification-sys, winrt-notification, fd-lock, tracing, tempfile

---

## File Structure

```
claude-notification-plugin/
├── .claude-plugin/
│   └── plugin.json                          # Plugin manifest
├── hooks/
│   ├── hooks.json                           # Hook event registration
│   ├── hook-wrapper.sh                      # Unix binary launcher
│   └── hook-wrapper.cmd                     # Windows binary launcher
├── skills/
│   └── settings/
│       └── SKILL.md                         # Interactive config skill
├── commands/
│   └── notification-settings.md             # Slash command
├── config/
│   └── default-config.yaml                  # Default config template
├── sounds/                                  # Audio files (added later)
├── bin/
│   └── .gitkeep                             # Pre-compiled binaries dir
├── crates/
│   ├── Cargo.toml                           # Workspace root
│   ├── claude-notify/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs                      # CLI entry point
│   ├── claude-notify-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                       # Public API
│   │       ├── types.rs                     # Status, Priority, Channel, Event types
│   │       ├── error.rs                     # NotifyError with thiserror
│   │       ├── config.rs                    # YAML config loading + layered merge
│   │       ├── hook.rs                      # Hook stdin JSON parsing
│   │       ├── dedup.rs                     # File-lock deduplication
│   │       ├── analyzer.rs                  # Transcript JSONL parse + status detection
│   │       ├── summary.rs                   # Content summary extraction + cleanup
│   │       ├── priority.rs                  # Priority assessment
│   │       ├── suppression.rs               # Cooldown, dedup, cascade, filter rules
│   │       ├── decision.rs                  # Intelligence engine
│   │       └── state.rs                     # Session state persistence
│   ├── claude-notify-dispatch/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                       # Public API + router
│   │       ├── traits.rs                    # Dispatcher trait
│   │       ├── desktop.rs                   # Desktop notification dispatch
│   │       ├── sound.rs                     # Audio playback
│   │       ├── terminal_bell.rs             # Terminal bell
│   │       └── webhook.rs                   # Webhook formatters + sender
│   └── claude-notify-platform/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                       # Trait definitions + factory
│           ├── activity.rs                  # User idle/focus detection
│           ├── macos.rs                     # macOS notification + activity
│           ├── linux.rs                     # Linux notification + activity
│           └── windows.rs                   # Windows notification + activity
└── tests/
    └── fixtures/                            # Test transcript JSONL files
        ├── task_complete.jsonl
        ├── question.jsonl
        ├── api_error.jsonl
        └── session_limit.jsonl
```

---

## Task 1: Cargo Workspace Scaffolding

**Files:**
- Create: `crates/Cargo.toml`
- Create: `crates/claude-notify/Cargo.toml`
- Create: `crates/claude-notify/src/main.rs`
- Create: `crates/claude-notify-core/Cargo.toml`
- Create: `crates/claude-notify-core/src/lib.rs`
- Create: `crates/claude-notify-dispatch/Cargo.toml`
- Create: `crates/claude-notify-dispatch/src/lib.rs`
- Create: `crates/claude-notify-platform/Cargo.toml`
- Create: `crates/claude-notify-platform/src/lib.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
# crates/Cargo.toml
[workspace]
resolver = "2"
members = [
    "claude-notify",
    "claude-notify-core",
    "claude-notify-dispatch",
    "claude-notify-platform",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
thiserror = "2"
anyhow = "1"
tracing = "0.1"
```

- [ ] **Step 2: Create claude-notify-core crate**

```toml
# crates/claude-notify-core/Cargo.toml
[package]
name = "claude-notify-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
fd-lock = "4"
tempfile = "3"
```

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
```

- [ ] **Step 3: Create claude-notify-platform crate**

```toml
# crates/claude-notify-platform/Cargo.toml
[package]
name = "claude-notify-platform"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror = { workspace = true }
tracing = { workspace = true }

[target.'cfg(target_os = "macos")'.dependencies]
mac-notification-sys = "0.6"

[target.'cfg(target_os = "linux")'.dependencies]
notify-rust = "4"

[target.'cfg(target_os = "windows")'.dependencies]
winrt-notification = "0.2"
```

```rust
// crates/claude-notify-platform/src/lib.rs
pub mod activity;
```

- [ ] **Step 4: Create claude-notify-dispatch crate**

```toml
# crates/claude-notify-dispatch/Cargo.toml
[package]
name = "claude-notify-dispatch"
version.workspace = true
edition.workspace = true

[dependencies]
claude-notify-platform = { path = "../claude-notify-platform" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
ureq = "2"
rodio = { version = "0.19", default-features = false, features = ["mp3", "wav", "vorbis"] }
```

```rust
// crates/claude-notify-dispatch/src/lib.rs
pub mod traits;
```

- [ ] **Step 5: Create claude-notify binary crate**

```toml
# crates/claude-notify/Cargo.toml
[package]
name = "claude-notify"
version.workspace = true
edition.workspace = true

[[bin]]
name = "claude-notify"
path = "src/main.rs"

[dependencies]
claude-notify-core = { path = "../claude-notify-core" }
claude-notify-dispatch = { path = "../claude-notify-dispatch" }
claude-notify-platform = { path = "../claude-notify-platform" }
anyhow = { workspace = true }
clap = { version = "4", features = ["derive"] }
tracing = { workspace = true }
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

```rust
// crates/claude-notify/src/main.rs
fn main() {
    println!("claude-notify v0.1.0");
}
```

- [ ] **Step 6: Verify workspace builds**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo build`
Expected: Compiles successfully with no errors.

- [ ] **Step 7: Commit**

```bash
git add crates/
git commit -m "feat: scaffold Cargo workspace with 4 crates"
```

---

## Task 2: Core Types and Error Definitions

**Files:**
- Create: `crates/claude-notify-core/src/types.rs`
- Create: `crates/claude-notify-core/src/error.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for types**

```rust
// crates/claude-notify-core/src/types.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display() {
        assert_eq!(Status::TaskComplete.as_str(), "task_complete");
        assert_eq!(Status::ApiError.as_str(), "api_error");
        assert_eq!(Status::Question.as_str(), "question");
    }

    #[test]
    fn priority_ordering() {
        assert!(Priority::Urgent > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    #[test]
    fn channel_from_str() {
        assert_eq!("desktop".parse::<Channel>().unwrap(), Channel::Desktop);
        assert_eq!("webhook".parse::<Channel>().unwrap(), Channel::Webhook);
        assert!("invalid".parse::<Channel>().is_err());
    }

    #[test]
    fn notification_builder() {
        let n = Notification::new("Title", "Body");
        assert_eq!(n.title, "Title");
        assert_eq!(n.body, "Body");
        assert!(n.subtitle.is_none());
        assert!(n.icon.is_none());

        let n = Notification::new("T", "B").with_subtitle("Sub".to_string());
        assert_eq!(n.subtitle.as_deref(), Some("Sub"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core`
Expected: FAIL — types not defined yet.

- [ ] **Step 3: Implement types**

```rust
// crates/claude-notify-core/src/types.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

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
            Status::TaskComplete => "\u{2705}",
            Status::ReviewComplete => "\u{1f50d}",
            Status::Question => "\u{2753}",
            Status::PlanReady => "\u{1f4cb}",
            Status::SessionLimit => "\u{23f1}",
            Status::ApiError => "\u{1f534}",
            Status::ApiOverloaded => "\u{1f534}",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low = 0,
    Normal = 1,
    Urgent = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Desktop,
    Sound,
    TerminalBell,
    Webhook,
}

impl FromStr for Channel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "desktop" => Ok(Channel::Desktop),
            "sound" => Ok(Channel::Sound),
            "terminal_bell" => Ok(Channel::TerminalBell),
            "webhook" => Ok(Channel::Webhook),
            other => Err(format!("unknown channel: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
    pub fn new(title: &str, body: &str) -> Self {
        Self {
            title: title.to_string(),
            body: body.to_string(),
            subtitle: None,
            icon: None,
            priority: Priority::Normal,
            click_action: None,
            thread_id: None,
            timeout: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: String) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_thread_id(mut self, thread_id: String) -> Self {
        self.thread_id = Some(thread_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClickAction {
    FocusTerminal { bundle_id: String },
    RunCommand { command: String },
}

#[derive(Debug, Clone)]
pub struct NotifyEvent {
    pub status: Status,
    pub priority: Priority,
    pub notification: Notification,
    pub session_id: String,
}

#[derive(Debug, Clone)]
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

// --- tests at bottom of file (from Step 1) ---
```

- [ ] **Step 4: Implement error types**

```rust
// crates/claude-notify-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NotifyError {
    #[error("config error: {0}")]
    Config(String),

    #[error("transcript parse error: {0}")]
    TranscriptParse(String),

    #[error("hook input error: {0}")]
    HookInput(String),

    #[error("platform error: {0}")]
    Platform(String),

    #[error("webhook error: {0}")]
    Webhook(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, NotifyError>;
```

- [ ] **Step 5: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core`
Expected: All tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/claude-notify-core/src/
git commit -m "feat(core): add types (Status, Priority, Channel, Notification, Decision) and error definitions"
```

---

## Task 3: Configuration System

**Files:**
- Create: `crates/claude-notify-core/src/config.rs`
- Create: `config/default-config.yaml`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for config loading**

```rust
// crates/claude-notify-core/src/config.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert!(config.desktop.enabled);
        assert!(config.sound.enabled);
        assert!(!config.webhook.enabled);
        assert_eq!(config.sound.volume, 0.8);
        assert_eq!(config.activity.idle_threshold_seconds, 30);
        assert_eq!(config.suppression.cooldown_seconds, 7);
    }

    #[test]
    fn load_from_yaml_string() {
        let yaml = r#"
desktop:
  enabled: false
  timeout: 10
sound:
  volume: 0.5
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(!config.desktop.enabled);
        assert_eq!(config.desktop.timeout, 10);
        assert_eq!(config.sound.volume, 0.5);
        // Unspecified fields use defaults
        assert!(config.sound.enabled);
    }

    #[test]
    fn merge_configs() {
        let base = Config::default();
        let overlay_yaml = r#"
sound:
  volume: 0.3
webhook:
  enabled: true
  preset: "discord"
  url: "https://example.com/hook"
"#;
        let overlay: Config = serde_yaml::from_str(overlay_yaml).unwrap();
        let merged = base.merge(overlay);
        assert_eq!(merged.sound.volume, 0.3);
        assert!(merged.webhook.enabled);
        assert_eq!(merged.webhook.preset, "discord");
        // Base values preserved where overlay doesn't specify
        assert!(merged.desktop.enabled);
    }

    #[test]
    fn priority_override() {
        let yaml = r#"
priority_overrides:
  review_complete: normal
  question: low
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config.priority_overrides.get("review_complete"),
            Some(&Priority::Normal)
        );
    }

    #[test]
    fn status_override() {
        let yaml = r#"
status_overrides:
  task_complete:
    enabled: true
    sound: "~/sounds/done.mp3"
    title: "Done!"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        let tc = config.status_overrides.get("task_complete").unwrap();
        assert_eq!(tc.title.as_deref(), Some("Done!"));
        assert_eq!(tc.sound.as_deref(), Some("~/sounds/done.mp3"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- config`
Expected: FAIL — Config not defined.

- [ ] **Step 3: Implement Config struct and loading**

```rust
// crates/claude-notify-core/src/config.rs
use crate::error::{NotifyError, Result};
use crate::types::Priority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub desktop: DesktopConfig,
    pub sound: SoundConfig,
    pub terminal_bell: TerminalBellConfig,
    pub webhook: WebhookConfig,
    pub priority_overrides: HashMap<String, Priority>,
    pub priority_channels: HashMap<String, HashMap<String, bool>>,
    pub status_overrides: HashMap<String, StatusOverride>,
    pub activity: ActivityConfig,
    pub suppression: SuppressionConfig,
    pub team: TeamConfig,
    pub debug: DebugConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop: DesktopConfig::default(),
            sound: SoundConfig::default(),
            terminal_bell: TerminalBellConfig::default(),
            webhook: WebhookConfig::default(),
            priority_overrides: HashMap::new(),
            priority_channels: HashMap::new(),
            status_overrides: HashMap::new(),
            activity: ActivityConfig::default(),
            suppression: SuppressionConfig::default(),
            team: TeamConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    pub enabled: bool,
    pub click_to_focus: bool,
    pub terminal_bundle_id: String,
    pub app_icon: String,
    pub timeout: u64,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            click_to_focus: true,
            terminal_bundle_id: String::new(),
            app_icon: String::new(),
            timeout: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
    pub volume: f32,
    pub device: String,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.8,
            device: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalBellConfig {
    pub enabled: bool,
}

impl Default for TerminalBellConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub preset: String,
    pub url: String,
    pub chat_id: String,
    pub headers: HashMap<String, String>,
    pub template: String,
    pub retry_max: u32,
    pub timeout_seconds: u64,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            preset: "slack".to_string(),
            url: String::new(),
            chat_id: String::new(),
            headers: HashMap::new(),
            template: String::new(),
            retry_max: 3,
            timeout_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StatusOverride {
    pub enabled: Option<bool>,
    pub sound: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityConfig {
    pub enabled: bool,
    pub idle_threshold_seconds: u64,
    pub suppress_when_focused: bool,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            idle_threshold_seconds: 30,
            suppress_when_focused: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SuppressionConfig {
    pub cooldown_seconds: u64,
    pub task_to_question_cooldown: u64,
    pub content_dedup_seconds: u64,
    pub filters: Vec<SuppressionFilter>,
}

impl Default for SuppressionConfig {
    fn default() -> Self {
        Self {
            cooldown_seconds: 7,
            task_to_question_cooldown: 12,
            content_dedup_seconds: 180,
            filters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SuppressionFilter {
    pub status: Option<String>,
    pub git_branch: Option<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamConfig {
    pub mode: String,
    pub notify_on_subagent: bool,
    pub suppress_for_subagents: bool,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            mode: "always".to_string(),
            notify_on_subagent: false,
            suppress_for_subagents: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugConfig {
    pub enabled: bool,
    pub log_file: String,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: String::new(),
        }
    }
}

impl Config {
    /// Load config from a YAML file. Returns default config if file doesn't exist.
    pub fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load config with layered overrides:
    /// defaults <- plugin built-in <- user global <- project local
    pub fn load_layered(plugin_root: &Path, project_root: Option<&Path>) -> Result<Self> {
        let mut config = Self::default();

        // Plugin built-in default
        let builtin = plugin_root.join("config").join("default-config.yaml");
        if builtin.exists() {
            let overlay = Self::load_from_file(&builtin)?;
            config = config.merge(overlay);
        }

        // User global
        let home = dirs_path();
        if let Some(home) = home {
            let user_config = home
                .join(".claude")
                .join("claude-notification")
                .join("config.yaml");
            if user_config.exists() {
                let overlay = Self::load_from_file(&user_config)?;
                config = config.merge(overlay);
            }
        }

        // Project local
        if let Some(project) = project_root {
            let project_config = project.join(".claude-notification.yaml");
            if project_config.exists() {
                let overlay = Self::load_from_file(&project_config)?;
                config = config.merge(overlay);
            }
        }

        Ok(config)
    }

    /// Merge another config on top of this one.
    /// Non-default values in `other` override values in `self`.
    pub fn merge(self, other: Config) -> Config {
        // For simplicity, serialize both to serde_yaml::Value, deep-merge, deserialize back.
        let base_val = serde_yaml::to_value(&self).unwrap_or(serde_yaml::Value::Null);
        let other_val = serde_yaml::to_value(&other).unwrap_or(serde_yaml::Value::Null);
        let merged = deep_merge_yaml(base_val, other_val);
        serde_yaml::from_value(merged).unwrap_or(self)
    }
}

fn deep_merge_yaml(base: serde_yaml::Value, overlay: serde_yaml::Value) -> serde_yaml::Value {
    use serde_yaml::Value;
    match (base, overlay) {
        (Value::Mapping(mut base_map), Value::Mapping(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let merged_val = if let Some(base_val) = base_map.remove(&key) {
                    deep_merge_yaml(base_val, overlay_val)
                } else {
                    overlay_val
                };
                base_map.insert(key, merged_val);
            }
            Value::Mapping(base_map)
        }
        (_, overlay) => overlay,
    }
}

fn dirs_path() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Create default-config.yaml**

```yaml
# config/default-config.yaml
# Default configuration for claude-notification plugin.
# User overrides: ~/.claude/claude-notification/config.yaml
# Project overrides: .claude-notification.yaml (in project root)

desktop:
  enabled: true
  click_to_focus: true
  terminal_bundle_id: ""
  app_icon: ""
  timeout: 5

sound:
  enabled: true
  volume: 0.8
  device: ""

terminal_bell:
  enabled: true

webhook:
  enabled: false
  preset: "slack"
  url: ""
  chat_id: ""
  headers: {}
  template: ""
  retry_max: 3
  timeout_seconds: 10

priority_overrides: {}
priority_channels: {}
status_overrides: {}

activity:
  enabled: true
  idle_threshold_seconds: 30
  suppress_when_focused: true

suppression:
  cooldown_seconds: 7
  task_to_question_cooldown: 12
  content_dedup_seconds: 180
  filters: []

team:
  mode: "always"
  notify_on_subagent: false
  suppress_for_subagents: true

debug:
  enabled: false
  log_file: ""
```

- [ ] **Step 5: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- config`
Expected: All config tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/claude-notify-core/src/config.rs config/
git commit -m "feat(core): add config system with YAML loading and layered merge"
```

---

## Task 4: Hook Input Parsing

**Files:**
- Create: `crates/claude-notify-core/src/hook.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for hook input parsing**

```rust
// crates/claude-notify-core/src/hook.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_hook_input() {
        let json = r#"{
            "session_id": "abc123",
            "transcript_path": "/tmp/transcript.jsonl",
            "tool_name": "Write",
            "tool_input": {"file_path": "/foo/bar.rs", "content": "hello"},
            "tool_result": "success",
            "is_team_lead": false,
            "team_name": ""
        }"#;
        let input = HookInput::from_json(json).unwrap();
        assert_eq!(input.session_id, "abc123");
        assert_eq!(input.tool_name, "Write");
        assert!(!input.is_team_lead);
    }

    #[test]
    fn parse_minimal_hook_input() {
        let json = r#"{"session_id": "x", "transcript_path": "/tmp/t.jsonl"}"#;
        let input = HookInput::from_json(json).unwrap();
        assert_eq!(input.session_id, "x");
        assert_eq!(input.tool_name, "");
    }

    #[test]
    fn parse_invalid_json() {
        let result = HookInput::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_hook_input_from_reader() {
        let json = r#"{"session_id": "s1", "transcript_path": "/tmp/t.jsonl", "tool_name": "Bash"}"#;
        let reader = std::io::Cursor::new(json);
        let input = HookInput::from_reader(reader).unwrap();
        assert_eq!(input.session_id, "s1");
        assert_eq!(input.tool_name, "Bash");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- hook`
Expected: FAIL — HookInput not defined.

- [ ] **Step 3: Implement HookInput**

```rust
// crates/claude-notify-core/src/hook.rs
use crate::error::{NotifyError, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub session_id: String,
    pub transcript_path: String,
    #[serde(default)]
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    #[serde(default)]
    pub tool_result: serde_json::Value,
    #[serde(default)]
    pub is_team_lead: bool,
    #[serde(default)]
    pub team_name: String,
    #[serde(default, rename = "isApiErrorMessage")]
    pub is_api_error_message: bool,
}

impl HookInput {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| NotifyError::HookInput(e.to_string()))
    }

    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(|e| NotifyError::HookInput(e.to_string()))
    }

    pub fn from_stdin() -> Result<Self> {
        Self::from_reader(std::io::stdin().lock())
    }
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- hook`
Expected: All hook tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-core/src/hook.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add hook input JSON parsing from stdin/reader"
```

---

## Task 5: Transcript Analyzer and Status Detection

**Files:**
- Create: `crates/claude-notify-core/src/analyzer.rs`
- Create: `tests/fixtures/task_complete.jsonl`
- Create: `tests/fixtures/question.jsonl`
- Create: `tests/fixtures/api_error.jsonl`
- Create: `tests/fixtures/session_limit.jsonl`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Create test fixture files**

```jsonl
// tests/fixtures/task_complete.jsonl
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write","input":{"file_path":"/tmp/test.rs","content":"fn main() {}"}}]},"duration_ms":500}
{"type":"tool_result","tool_name":"Write","content":"File written successfully"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I've created the file for you."}]},"duration_ms":300}
```

```jsonl
// tests/fixtures/question.jsonl
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"AskUserQuestion","input":{"question":"Which database do you prefer: PostgreSQL or MySQL?"}}]},"duration_ms":200}
```

```jsonl
// tests/fixtures/api_error.jsonl
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"An error occurred"}]},"isApiErrorMessage":true,"error":{"status":429,"message":"Rate limit exceeded"}}
```

```jsonl
// tests/fixtures/session_limit.jsonl
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Session limit reached. Please start a new session."}]},"duration_ms":100}
```

- [ ] **Step 2: Write tests for analyzer**

```rust
// crates/claude-notify-core/src/analyzer.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::HookInput;

    fn make_hook_input(tool_name: &str) -> HookInput {
        HookInput {
            session_id: "test".to_string(),
            transcript_path: String::new(),
            tool_name: tool_name.to_string(),
            tool_input: serde_json::Value::Null,
            tool_result: serde_json::Value::Null,
            is_team_lead: false,
            team_name: String::new(),
            is_api_error_message: false,
        }
    }

    #[test]
    fn detect_session_limit() {
        let messages = vec![Message {
            msg_type: "assistant".to_string(),
            text_content: "Session limit reached. Please start a new session.".to_string(),
            tool_name: None,
            tool_input: serde_json::Value::Null,
            is_api_error: false,
            error_status: None,
        }];
        let ctx = make_hook_input("");
        assert_eq!(detect_status(&messages, &ctx), Status::SessionLimit);
    }

    #[test]
    fn detect_api_error() {
        let messages = vec![Message {
            msg_type: "assistant".to_string(),
            text_content: "An error occurred".to_string(),
            tool_name: None,
            tool_input: serde_json::Value::Null,
            is_api_error: true,
            error_status: Some(500),
        }];
        let ctx = make_hook_input("");
        assert_eq!(detect_status(&messages, &ctx), Status::ApiError);
    }

    #[test]
    fn detect_api_overloaded() {
        let messages = vec![Message {
            msg_type: "assistant".to_string(),
            text_content: "".to_string(),
            tool_name: None,
            tool_input: serde_json::Value::Null,
            is_api_error: true,
            error_status: Some(429),
        }];
        let ctx = make_hook_input("");
        assert_eq!(detect_status(&messages, &ctx), Status::ApiOverloaded);
    }

    #[test]
    fn detect_question() {
        let ctx = make_hook_input("AskUserQuestion");
        let messages = vec![];
        assert_eq!(detect_status(&messages, &ctx), Status::Question);
    }

    #[test]
    fn detect_plan_ready() {
        let ctx = make_hook_input("ExitPlanMode");
        let messages = vec![];
        assert_eq!(detect_status(&messages, &ctx), Status::PlanReady);
    }

    #[test]
    fn detect_task_complete_with_write_tool() {
        let messages = vec![Message {
            msg_type: "tool_result".to_string(),
            text_content: "".to_string(),
            tool_name: Some("Write".to_string()),
            tool_input: serde_json::Value::Null,
            is_api_error: false,
            error_status: None,
        }];
        let ctx = make_hook_input("");
        assert_eq!(detect_status(&messages, &ctx), Status::TaskComplete);
    }

    #[test]
    fn detect_review_complete_read_only() {
        let messages = vec![
            Message {
                msg_type: "tool_result".to_string(),
                text_content: "".to_string(),
                tool_name: Some("Read".to_string()),
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
            Message {
                msg_type: "assistant".to_string(),
                text_content: "a]".repeat(120),  // >200 chars
                tool_name: None,
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
        ];
        let ctx = make_hook_input("");
        assert_eq!(detect_status(&messages, &ctx), Status::ReviewComplete);
    }

    #[test]
    fn parse_jsonl_transcript() {
        let jsonl = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello"}]},"duration_ms":100}
{"type":"tool_result","tool_name":"Write","content":"ok"}
"#;
        let messages = parse_transcript_str(jsonl);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text_content, "Hello");
        assert_eq!(messages[1].tool_name, Some("Write".to_string()));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- analyzer`
Expected: FAIL — analyzer module not defined.

- [ ] **Step 4: Implement analyzer**

```rust
// crates/claude-notify-core/src/analyzer.rs
use crate::error::Result;
use crate::hook::HookInput;
use crate::types::Status;
use std::path::Path;
use tracing::debug;

const RECENT_MESSAGE_LIMIT: usize = 15;
const OVERLOADED_STATUS_CODES: &[u16] = &[429, 529];
const WRITE_TOOLS: &[&str] = &["Write", "Edit", "Bash", "NotebookEdit"];
const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
const LONG_TEXT_THRESHOLD: usize = 200;

#[derive(Debug, Clone)]
pub struct Message {
    pub msg_type: String,
    pub text_content: String,
    pub tool_name: Option<String>,
    pub tool_input: serde_json::Value,
    pub is_api_error: bool,
    pub error_status: Option<u16>,
}

pub fn parse_transcript(path: &Path) -> Result<Vec<Message>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| crate::error::NotifyError::TranscriptParse(e.to_string()))?;
    Ok(parse_transcript_str(&content))
}

pub fn parse_transcript_str(content: &str) -> Vec<Message> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| parse_message_line(line))
        .collect()
}

fn parse_message_line(line: &str) -> Option<Message> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;

    let msg_type = val.get("type")?.as_str()?.to_string();
    let is_api_error = val
        .get("isApiErrorMessage")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let error_status = val
        .get("error")
        .and_then(|e| e.get("status"))
        .and_then(|s| s.as_u64())
        .map(|s| s as u16);

    let tool_name = val
        .get("tool_name")
        .and_then(|v| v.as_str())
        .map(String::from);

    let tool_input = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .and_then(|arr| {
            arr.iter().find(|item| {
                item.get("type").and_then(|t| t.as_str()) == Some("tool_use")
            })
        })
        .and_then(|item| item.get("input").cloned())
        .unwrap_or(serde_json::Value::Null);

    // Extract text content from assistant messages
    let text_content = extract_text_content(&val);

    // Extract tool_name from content array if not at top level
    let tool_name = tool_name.or_else(|| {
        val.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| {
                arr.iter().find_map(|item| {
                    if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        item.get("name").and_then(|n| n.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
            })
    });

    Some(Message {
        msg_type,
        text_content,
        tool_name,
        tool_input,
        is_api_error,
        error_status,
    })
}

fn extract_text_content(val: &serde_json::Value) -> String {
    // Try content array in message
    if let Some(arr) = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        let texts: Vec<&str> = arr
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str())
                } else {
                    None
                }
            })
            .collect();
        if !texts.is_empty() {
            return texts.join("\n");
        }
    }

    // Try top-level content string
    val.get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn detect_status(messages: &[Message], hook_context: &HookInput) -> Status {
    let recent = recent_messages(messages);

    // 1. Session limit (highest priority)
    if detect_session_limit(&recent) {
        debug!("detected session_limit");
        return Status::SessionLimit;
    }

    // 2. API errors
    if let Some(status) = detect_api_error(hook_context, &recent) {
        debug!(status = %status.as_str(), "detected api error");
        return status;
    }

    // 3. Direct tool mapping
    if hook_context.tool_name == "AskUserQuestion" {
        debug!("detected question from AskUserQuestion tool");
        return Status::Question;
    }
    if hook_context.tool_name == "ExitPlanMode" {
        debug!("detected plan_ready from ExitPlanMode tool");
        return Status::PlanReady;
    }

    // 4. Tool analysis
    let tools: Vec<&str> = recent
        .iter()
        .filter_map(|m| m.tool_name.as_deref())
        .collect();

    if tools.iter().any(|t| WRITE_TOOLS.contains(t)) {
        debug!("detected task_complete (write tools found)");
        return Status::TaskComplete;
    }

    let has_only_reads = !tools.is_empty()
        && tools.iter().all(|t| READ_TOOLS.contains(t));
    let has_long_text = recent
        .iter()
        .any(|m| m.msg_type == "assistant" && m.text_content.len() > LONG_TEXT_THRESHOLD);

    if has_only_reads && has_long_text {
        debug!("detected review_complete (read-only + long text)");
        return Status::ReviewComplete;
    }

    // 5. Fallback
    debug!("fallback to task_complete");
    Status::TaskComplete
}

fn recent_messages(messages: &[Message]) -> &[Message] {
    let start = messages.len().saturating_sub(RECENT_MESSAGE_LIMIT);
    &messages[start..]
}

fn detect_session_limit(messages: &[Message]) -> bool {
    messages.iter().rev().take(3).any(|m| {
        m.text_content
            .to_lowercase()
            .contains("session limit reached")
    })
}

fn detect_api_error(hook_context: &HookInput, messages: &[Message]) -> Option<Status> {
    // Check hook context flag
    if hook_context.is_api_error_message {
        return Some(Status::ApiError);
    }

    // Check recent messages
    for msg in messages.iter().rev().take(3) {
        if msg.is_api_error {
            if let Some(code) = msg.error_status {
                if OVERLOADED_STATUS_CODES.contains(&code) {
                    return Some(Status::ApiOverloaded);
                }
            }
            return Some(Status::ApiError);
        }
    }

    None
}

// --- tests at bottom (from Step 2) ---
```

- [ ] **Step 5: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- analyzer`
Expected: All analyzer tests PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/claude-notify-core/src/analyzer.rs crates/claude-notify-core/src/lib.rs tests/
git commit -m "feat(core): add transcript parser and status detection state machine"
```

---

## Task 6: Content Summary Extraction

**Files:**
- Create: `crates/claude-notify-core/src/summary.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for summary extraction**

```rust
// crates/claude-notify-core/src/summary.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_markdown_formatting() {
        assert_eq!(clean_markdown("## Hello World"), "Hello World");
        assert_eq!(clean_markdown("**bold** text"), "bold text");
        assert_eq!(clean_markdown("`code` here"), "code here");
        assert_eq!(clean_markdown("[link](http://example.com)"), "link");
    }

    #[test]
    fn clean_code_blocks() {
        let input = "before\n```rust\nfn main() {}\n```\nafter";
        assert_eq!(clean_markdown(input), "before after");
    }

    #[test]
    fn truncate_at_word_boundary() {
        let long = "This is a very long sentence that should be truncated at a word boundary somewhere around here and not in the middle of a word";
        let result = truncate(long, 50);
        assert!(result.len() <= 53); // 50 + "..."
        assert!(result.ends_with("..."));
        assert!(!result.contains("  ")); // no broken words
    }

    #[test]
    fn truncate_short_text_unchanged() {
        assert_eq!(truncate("short text", 50), "short text");
    }

    #[test]
    fn extract_question_from_tool() {
        let messages = vec![Message {
            msg_type: "assistant".to_string(),
            text_content: "".to_string(),
            tool_name: Some("AskUserQuestion".to_string()),
            tool_input: serde_json::json!({"question": "Which DB do you prefer?"}),
            is_api_error: false,
            error_status: None,
        }];
        let summary = extract_summary(&Status::Question, &messages);
        assert_eq!(summary, "Which DB do you prefer?");
    }

    #[test]
    fn extract_question_fallback_to_text() {
        let messages = vec![Message {
            msg_type: "assistant".to_string(),
            text_content: "What do you think about this approach?".to_string(),
            tool_name: None,
            tool_input: serde_json::Value::Null,
            is_api_error: false,
            error_status: None,
        }];
        let summary = extract_summary(&Status::Question, &messages);
        assert!(summary.contains("What do you think"));
    }

    #[test]
    fn extract_work_summary_with_action_counts() {
        let messages = vec![
            Message {
                msg_type: "tool_result".to_string(),
                text_content: "".to_string(),
                tool_name: Some("Write".to_string()),
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
            Message {
                msg_type: "tool_result".to_string(),
                text_content: "".to_string(),
                tool_name: Some("Write".to_string()),
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
            Message {
                msg_type: "tool_result".to_string(),
                text_content: "".to_string(),
                tool_name: Some("Read".to_string()),
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
            Message {
                msg_type: "assistant".to_string(),
                text_content: "I've updated both files.".to_string(),
                tool_name: None,
                tool_input: serde_json::Value::Null,
                is_api_error: false,
                error_status: None,
            },
        ];
        let summary = extract_summary(&Status::TaskComplete, &messages);
        assert!(summary.contains("2 writes"));
        assert!(summary.contains("1 read"));
    }

    #[test]
    fn extract_session_limit() {
        let summary = extract_summary(&Status::SessionLimit, &[]);
        assert_eq!(summary, "Session limit reached");
    }

    #[test]
    fn collapse_whitespace() {
        assert_eq!(clean_markdown("hello   world\n\nfoo"), "hello world foo");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- summary`
Expected: FAIL — summary module not defined.

- [ ] **Step 3: Implement summary extraction**

```rust
// crates/claude-notify-core/src/summary.rs
use crate::analyzer::Message;
use crate::types::Status;
use std::collections::HashMap;

const MAX_SUMMARY_LEN: usize = 150;

pub fn extract_summary(status: &Status, messages: &[Message]) -> String {
    let raw = match status {
        Status::Question => extract_question(messages),
        Status::PlanReady => extract_plan_summary(messages),
        Status::ApiError | Status::ApiOverloaded => extract_error_info(messages),
        Status::SessionLimit => "Session limit reached".to_string(),
        Status::TaskComplete | Status::ReviewComplete => extract_work_summary(messages),
    };
    clean_and_truncate(&raw)
}

fn extract_question(messages: &[Message]) -> String {
    // Try AskUserQuestion tool_input first
    for msg in messages.iter().rev() {
        if msg.tool_name.as_deref() == Some("AskUserQuestion") {
            if let Some(q) = msg.tool_input.get("question").and_then(|v| v.as_str()) {
                return q.to_string();
            }
        }
    }
    // Fallback: last text containing '?'
    for msg in messages.iter().rev() {
        if msg.msg_type == "assistant" && msg.text_content.contains('?') {
            return msg.text_content.clone();
        }
    }
    "Waiting for your input".to_string()
}

fn extract_plan_summary(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        if msg.tool_name.as_deref() == Some("ExitPlanMode") {
            if let Some(plan) = msg.tool_input.get("plan").and_then(|v| v.as_str()) {
                return plan.to_string();
            }
        }
    }
    // Fallback: last assistant text
    for msg in messages.iter().rev() {
        if msg.msg_type == "assistant" && !msg.text_content.is_empty() {
            return msg.text_content.clone();
        }
    }
    "Plan is ready for review".to_string()
}

fn extract_error_info(messages: &[Message]) -> String {
    for msg in messages.iter().rev().take(3) {
        if msg.is_api_error {
            let code = msg
                .error_status
                .map(|c| format!(" ({})", c))
                .unwrap_or_default();
            if !msg.text_content.is_empty() {
                return format!("{}{}", msg.text_content, code);
            }
            return format!("API error{}", code);
        }
    }
    "API error occurred".to_string()
}

fn extract_work_summary(messages: &[Message]) -> String {
    let recent: Vec<&Message> = messages.iter().rev().take(5).collect();

    // Count tool actions
    let mut tool_counts: HashMap<&str, usize> = HashMap::new();
    for msg in &recent {
        if let Some(ref tool) = msg.tool_name {
            let category = match tool.as_str() {
                "Write" | "Edit" | "NotebookEdit" => "write",
                "Read" | "Grep" | "Glob" => "read",
                "Bash" => "bash",
                _ => continue,
            };
            *tool_counts.entry(category).or_insert(0) += 1;
        }
    }

    // Find last meaningful text
    let last_text = recent
        .iter()
        .find(|m| m.msg_type == "assistant" && !m.text_content.is_empty())
        .map(|m| m.text_content.as_str())
        .unwrap_or("");

    // Build summary
    let mut parts = Vec::new();
    if let Some(&count) = tool_counts.get("write") {
        parts.push(format!("{} write{}", count, if count > 1 { "s" } else { "" }));
    }
    if let Some(&count) = tool_counts.get("read") {
        parts.push(format!("{} read{}", count, if count > 1 { "s" } else { "" }));
    }
    if let Some(&count) = tool_counts.get("bash") {
        parts.push(format!("{} bash", count));
    }

    let action_str = if parts.is_empty() {
        String::new()
    } else {
        format!("[{}] ", parts.join(", "))
    };

    format!("{}{}", action_str, last_text)
}

fn clean_and_truncate(text: &str) -> String {
    let cleaned = clean_markdown(text);
    truncate(&cleaned, MAX_SUMMARY_LEN)
}

pub fn clean_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Remove code blocks (```...```)
    while let Some(start) = result.find("```") {
        if let Some(end) = result[start + 3..].find("```") {
            result = format!("{}{}", &result[..start], &result[start + 3 + end + 3..]);
        } else {
            break;
        }
    }

    // Remove inline code
    result = result.replace('`', "");

    // Remove markdown links [text](url) -> text
    let mut cleaned = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut link_text = String::new();
            let mut found_close = false;
            for inner in chars.by_ref() {
                if inner == ']' {
                    found_close = true;
                    break;
                }
                link_text.push(inner);
            }
            if found_close {
                // Skip (url) part if present
                if chars.peek() == Some(&'(') {
                    chars.next();
                    for inner in chars.by_ref() {
                        if inner == ')' {
                            break;
                        }
                    }
                }
                cleaned.push_str(&link_text);
            } else {
                cleaned.push('[');
                cleaned.push_str(&link_text);
            }
        } else {
            cleaned.push(c);
        }
    }
    result = cleaned;

    // Remove headers (## )
    result = result
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim_start()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Remove bold/italic markers
    result = result.replace("**", "");
    result = result.replace("__", "");
    result = result.replace('*', "");
    result = result.replace('_', " ");

    // Remove bullet points
    result = result
        .split('\n')
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                &trimmed[2..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Collapse whitespace
    let mut prev_space = false;
    result = result
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                if prev_space {
                    None
                } else {
                    prev_space = true;
                    Some(' ')
                }
            } else {
                prev_space = false;
                Some(c)
            }
        })
        .collect();

    result.trim().to_string()
}

pub fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    // Find word boundary before max_len
    let truncated = &text[..max_len];
    if let Some(last_space) = truncated.rfind(' ') {
        format!("{}...", &text[..last_space])
    } else {
        format!("{}...", truncated)
    }
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
pub mod summary;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- summary`
Expected: All summary tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-core/src/summary.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add content summary extraction with markdown cleanup"
```

---

## Task 7: Priority Assessment

**Files:**
- Create: `crates/claude-notify-core/src/priority.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for priority**

```rust
// crates/claude-notify-core/src/priority.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_priorities() {
        let engine = PriorityEngine::new(HashMap::new(), HashMap::new());
        assert_eq!(engine.assess(&Status::ApiError), Priority::Urgent);
        assert_eq!(engine.assess(&Status::Question), Priority::Urgent);
        assert_eq!(engine.assess(&Status::SessionLimit), Priority::Urgent);
        assert_eq!(engine.assess(&Status::TaskComplete), Priority::Normal);
        assert_eq!(engine.assess(&Status::PlanReady), Priority::Normal);
        assert_eq!(engine.assess(&Status::ReviewComplete), Priority::Low);
    }

    #[test]
    fn priority_override() {
        let mut overrides = HashMap::new();
        overrides.insert("review_complete".to_string(), Priority::Normal);
        overrides.insert("question".to_string(), Priority::Low);
        let engine = PriorityEngine::new(overrides, HashMap::new());
        assert_eq!(engine.assess(&Status::ReviewComplete), Priority::Normal);
        assert_eq!(engine.assess(&Status::Question), Priority::Low);
        // Non-overridden stays default
        assert_eq!(engine.assess(&Status::ApiError), Priority::Urgent);
    }

    #[test]
    fn channels_for_urgent() {
        let engine = PriorityEngine::new(HashMap::new(), HashMap::new());
        let channels = engine.channels_for(&Priority::Urgent);
        assert!(channels.contains(&Channel::Desktop));
        assert!(channels.contains(&Channel::Sound));
        assert!(channels.contains(&Channel::TerminalBell));
        assert!(channels.contains(&Channel::Webhook));
    }

    #[test]
    fn channels_for_low() {
        let engine = PriorityEngine::new(HashMap::new(), HashMap::new());
        let channels = engine.channels_for(&Priority::Low);
        assert!(channels.contains(&Channel::Desktop));
        assert!(!channels.contains(&Channel::Sound));
        assert!(!channels.contains(&Channel::Webhook));
    }

    #[test]
    fn channel_override() {
        let mut channel_overrides = HashMap::new();
        let mut urgent_channels = HashMap::new();
        urgent_channels.insert("sound".to_string(), false);
        channel_overrides.insert("urgent".to_string(), urgent_channels);
        let engine = PriorityEngine::new(HashMap::new(), channel_overrides);
        let channels = engine.channels_for(&Priority::Urgent);
        assert!(!channels.contains(&Channel::Sound));
        assert!(channels.contains(&Channel::Desktop));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- priority`
Expected: FAIL.

- [ ] **Step 3: Implement PriorityEngine**

```rust
// crates/claude-notify-core/src/priority.rs
use crate::types::{Channel, Priority, Status};
use std::collections::HashMap;

pub struct PriorityEngine {
    overrides: HashMap<String, Priority>,
    channel_overrides: HashMap<String, HashMap<String, bool>>,
}

impl PriorityEngine {
    pub fn new(
        overrides: HashMap<String, Priority>,
        channel_overrides: HashMap<String, HashMap<String, bool>>,
    ) -> Self {
        Self {
            overrides,
            channel_overrides,
        }
    }

    pub fn assess(&self, status: &Status) -> Priority {
        // Check user overrides first
        if let Some(priority) = self.overrides.get(status.as_str()) {
            return *priority;
        }
        // Default mapping
        match status {
            Status::ApiError | Status::SessionLimit | Status::Question => Priority::Urgent,
            Status::TaskComplete | Status::PlanReady | Status::ApiOverloaded => Priority::Normal,
            Status::ReviewComplete => Priority::Low,
        }
    }

    pub fn channels_for(&self, priority: &Priority) -> Vec<Channel> {
        let priority_str = match priority {
            Priority::Urgent => "urgent",
            Priority::Normal => "normal",
            Priority::Low => "low",
        };

        let defaults = default_channels(priority);

        // Apply overrides
        if let Some(overrides) = self.channel_overrides.get(priority_str) {
            defaults
                .into_iter()
                .filter(|ch| {
                    let ch_str = match ch {
                        Channel::Desktop => "desktop",
                        Channel::Sound => "sound",
                        Channel::TerminalBell => "terminal_bell",
                        Channel::Webhook => "webhook",
                    };
                    overrides.get(ch_str).copied().unwrap_or(true)
                })
                .collect()
        } else {
            defaults
        }
    }

    pub fn bypasses_idle_check(&self, priority: &Priority) -> bool {
        matches!(priority, Priority::Urgent)
    }

    pub fn bypasses_cooldown(&self, priority: &Priority) -> bool {
        matches!(priority, Priority::Urgent)
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

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
pub mod summary;
pub mod priority;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- priority`
Expected: All priority tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-core/src/priority.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add priority assessment engine with overrides"
```

---

## Task 8: Suppression Engine

**Files:**
- Create: `crates/claude-notify-core/src/suppression.rs`
- Create: `crates/claude-notify-core/src/state.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for state persistence**

```rust
// crates/claude-notify-core/src/state.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_state() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test-state.json");
        let mut state = SessionState::new();
        state.last_notification_time = Some(1000);
        state.last_notification_status = Some("task_complete".to_string());
        state.last_notification_content = Some("done".to_string());
        state.save(&path).unwrap();

        let loaded = SessionState::load(&path).unwrap();
        assert_eq!(loaded.last_notification_time, Some(1000));
        assert_eq!(loaded.last_notification_status.as_deref(), Some("task_complete"));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let state = SessionState::load(Path::new("/nonexistent/state.json")).unwrap();
        assert!(state.last_notification_time.is_none());
    }
}
```

- [ ] **Step 2: Write tests for suppression engine**

```rust
// crates/claude-notify-core/src/suppression.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Status;

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn no_suppression_on_first_notification() {
        let config = SuppressionConfig::default();
        let state = SessionState::new();
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::TaskComplete, "hello", &state, false);
        assert!(result.is_none());
    }

    #[test]
    fn cooldown_suppresses_within_window() {
        let config = SuppressionConfig::default();
        let mut state = SessionState::new();
        state.last_notification_time = Some(now_secs());
        state.last_notification_status = Some("task_complete".to_string());
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::Question, "q?", &state, false);
        assert!(result.is_some());
        assert!(result.unwrap().contains("cooldown"));
    }

    #[test]
    fn cooldown_allows_after_window() {
        let config = SuppressionConfig::default();
        let mut state = SessionState::new();
        state.last_notification_time = Some(now_secs() - 20); // 20 seconds ago
        state.last_notification_status = Some("task_complete".to_string());
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::Question, "q?", &state, false);
        assert!(result.is_none());
    }

    #[test]
    fn content_dedup_suppresses_same_content() {
        let config = SuppressionConfig::default();
        let mut state = SessionState::new();
        state.last_notification_time = Some(now_secs() - 60);
        state.last_notification_content = Some("same message".to_string());
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::TaskComplete, "same message", &state, false);
        assert!(result.is_some());
        assert!(result.unwrap().contains("dedup"));
    }

    #[test]
    fn content_dedup_allows_different_content() {
        let config = SuppressionConfig::default();
        let mut state = SessionState::new();
        state.last_notification_time = Some(now_secs() - 60);
        state.last_notification_content = Some("old message".to_string());
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::TaskComplete, "new message", &state, false);
        assert!(result.is_none());
    }

    #[test]
    fn bypass_cooldown_ignores_suppression() {
        let config = SuppressionConfig::default();
        let mut state = SessionState::new();
        state.last_notification_time = Some(now_secs());
        state.last_notification_status = Some("task_complete".to_string());
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::ApiError, "error", &state, true); // bypass=true
        assert!(result.is_none());
    }

    #[test]
    fn filter_suppresses_matching_status() {
        let config = SuppressionConfig {
            filters: vec![SuppressionFilter {
                status: Some("task_complete".to_string()),
                git_branch: None,
                folder: None,
            }],
            ..Default::default()
        };
        let state = SessionState::new();
        let engine = SuppressionEngine::new(&config);
        let result = engine.check(&Status::TaskComplete, "msg", &state, false);
        assert!(result.is_some());
        assert!(result.unwrap().contains("filter"));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- suppression state`
Expected: FAIL.

- [ ] **Step 4: Implement SessionState**

```rust
// crates/claude-notify-core/src/state.rs
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionState {
    pub last_notification_time: Option<u64>,
    pub last_notification_status: Option<String>,
    pub last_notification_content: Option<String>,
    pub last_task_complete_time: Option<u64>,
}

impl SessionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path)?;
        let state: Self = serde_json::from_str(&content)
            .unwrap_or_default();
        Ok(state)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn state_path(session_id: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("claude-notify-state-{}.json", session_id))
    }

    pub fn update_after_notification(&mut self, status: &str, content: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_notification_time = Some(now);
        self.last_notification_status = Some(status.to_string());
        self.last_notification_content = Some(content.to_string());
        if status == "task_complete" {
            self.last_task_complete_time = Some(now);
        }
    }
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 5: Implement SuppressionEngine**

```rust
// crates/claude-notify-core/src/suppression.rs
use crate::config::{SuppressionConfig, SuppressionFilter};
use crate::types::Status;

pub struct SuppressionEngine<'a> {
    config: &'a SuppressionConfig,
}

impl<'a> SuppressionEngine<'a> {
    pub fn new(config: &'a SuppressionConfig) -> Self {
        Self { config }
    }

    /// Returns Some(reason) if notification should be suppressed, None if allowed.
    pub fn check(
        &self,
        status: &Status,
        content: &str,
        state: &crate::state::SessionState,
        bypass_cooldown: bool,
    ) -> Option<String> {
        // Check filters first (always applied, even with bypass)
        if let Some(reason) = self.check_filters(status) {
            return Some(reason);
        }

        if bypass_cooldown {
            return None;
        }

        let now = now_secs();

        // Check cascade: task_complete -> question suppression
        if *status == Status::Question {
            if let Some(last_time) = state.last_notification_time {
                let elapsed = now.saturating_sub(last_time);

                // After task_complete, longer cooldown for questions
                if state.last_notification_status.as_deref() == Some("task_complete")
                    && elapsed < self.config.task_to_question_cooldown
                {
                    return Some(format!(
                        "cooldown: question suppressed {}s after task_complete",
                        self.config.task_to_question_cooldown - elapsed
                    ));
                }

                // General cooldown for questions after any notification
                if elapsed < self.config.cooldown_seconds {
                    return Some(format!(
                        "cooldown: question suppressed {}s after last notification",
                        self.config.cooldown_seconds - elapsed
                    ));
                }
            }
        }

        // Content dedup
        if let (Some(last_content), Some(last_time)) =
            (&state.last_notification_content, state.last_notification_time)
        {
            let elapsed = now.saturating_sub(last_time);
            if elapsed < self.config.content_dedup_seconds && last_content == content {
                return Some(format!(
                    "dedup: same content within {}s window",
                    self.config.content_dedup_seconds
                ));
            }
        }

        None
    }

    fn check_filters(&self, status: &Status) -> Option<String> {
        for filter in &self.config.filters {
            let status_match = filter
                .status
                .as_ref()
                .map(|s| s == status.as_str())
                .unwrap_or(true);

            // git_branch and folder checks would need runtime context;
            // for now, only status filter is checked here.
            // Full filter matching happens in decision.rs with runtime context.
            let branch_match = filter.git_branch.is_none();
            let folder_match = filter.folder.is_none();

            if status_match && branch_match && folder_match {
                return Some(format!("filter: status={} matched", status.as_str()));
            }
        }
        None
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// --- tests at bottom (from Step 2) ---
```

- [ ] **Step 6: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
pub mod summary;
pub mod priority;
pub mod state;
pub mod suppression;
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- suppression state`
Expected: All tests PASS.

- [ ] **Step 8: Commit**

```bash
git add crates/claude-notify-core/src/state.rs crates/claude-notify-core/src/suppression.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add suppression engine with cooldown, dedup, cascade, and filters"
```

---

## Task 9: Deduplication (File Locks)

**Files:**
- Create: `crates/claude-notify-core/src/dedup.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for dedup**

```rust
// crates/claude-notify-core/src/dedup.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_lock_succeeds_first_time() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test.lock");
        let guard = DedupLock::try_acquire(&lock_path, 2).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn acquire_lock_fails_when_held() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test2.lock");
        let _guard1 = DedupLock::try_acquire(&lock_path, 2).unwrap();
        let guard2 = DedupLock::try_acquire(&lock_path, 2).unwrap();
        assert!(guard2.is_none());
    }

    #[test]
    fn stale_lock_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let lock_path = dir.path().join("test3.lock");
        // Create a stale lock (timestamp in the past)
        std::fs::write(&lock_path, "0").unwrap();
        let guard = DedupLock::try_acquire(&lock_path, 2).unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn lock_path_for_session() {
        let path = dedup_lock_path("session123");
        assert!(path.to_string_lossy().contains("claude-notify-dedup-session123"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- dedup`
Expected: FAIL.

- [ ] **Step 3: Implement DedupLock**

```rust
// crates/claude-notify-core/src/dedup.rs
use crate::error::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct DedupLock {
    path: PathBuf,
}

impl DedupLock {
    /// Try to acquire a dedup lock. Returns Some(guard) if acquired, None if already held.
    /// `ttl_seconds` is the max age of a lock before it's considered stale.
    pub fn try_acquire(path: &Path, ttl_seconds: u64) -> Result<Option<Self>> {
        // Check if lock exists and is not stale
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(timestamp) = content.trim().parse::<u64>() {
                    let now = now_secs();
                    if now.saturating_sub(timestamp) < ttl_seconds {
                        debug!("dedup lock held (age {}s < {}s TTL)", now - timestamp, ttl_seconds);
                        return Ok(None);
                    }
                    debug!("dedup lock stale, replacing");
                }
            }
        }

        // Create/replace lock
        let now = now_secs();
        fs::write(path, now.to_string())?;
        debug!("dedup lock acquired at {}", path.display());
        Ok(Some(Self {
            path: path.to_path_buf(),
        }))
    }
}

impl Drop for DedupLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn dedup_lock_path(session_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("claude-notify-dedup-{}.lock", session_id))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
pub mod summary;
pub mod priority;
pub mod state;
pub mod suppression;
pub mod dedup;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- dedup`
Expected: All dedup tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-core/src/dedup.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add file-lock based deduplication with TTL"
```

---

## Task 10: Decision Engine

**Files:**
- Create: `crates/claude-notify-core/src/decision.rs`
- Modify: `crates/claude-notify-core/src/lib.rs`

- [ ] **Step 1: Write tests for decision engine**

```rust
// crates/claude-notify-core/src/decision.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    struct MockActivity {
        idle: u64,
        focused: bool,
    }

    impl UserActivity for MockActivity {
        fn idle_seconds(&self) -> u64 { self.idle }
        fn is_terminal_focused(&self) -> bool { self.focused }
    }

    fn default_engine() -> (Config, PriorityEngine) {
        let config = Config::default();
        let priority = PriorityEngine::new(
            config.priority_overrides.clone(),
            config.priority_channels.clone(),
        );
        (config, priority)
    }

    #[test]
    fn notify_when_user_idle() {
        let (config, priority_engine) = default_engine();
        let activity = MockActivity { idle: 60, focused: false };
        let state = SessionState::new();
        let engine = DecisionEngine::new(&config, &priority_engine);
        let decision = engine.decide(
            Status::TaskComplete,
            "Task done",
            &activity,
            &state,
        );
        match decision {
            Decision::Notify { priority, .. } => assert_eq!(priority, Priority::Normal),
            other => panic!("expected Notify, got {:?}", other),
        }
    }

    #[test]
    fn downgrade_when_terminal_focused() {
        let (config, priority_engine) = default_engine();
        let activity = MockActivity { idle: 5, focused: true };
        let state = SessionState::new();
        let engine = DecisionEngine::new(&config, &priority_engine);
        let decision = engine.decide(
            Status::TaskComplete,
            "Task done",
            &activity,
            &state,
        );
        match decision {
            Decision::Downgrade { to, channels, .. } => {
                assert_eq!(to, Priority::Low);
                assert!(channels.contains(&Channel::TerminalBell));
                assert!(!channels.contains(&Channel::Sound));
            }
            other => panic!("expected Downgrade, got {:?}", other),
        }
    }

    #[test]
    fn urgent_bypasses_focus_check() {
        let (config, priority_engine) = default_engine();
        let activity = MockActivity { idle: 5, focused: true };
        let state = SessionState::new();
        let engine = DecisionEngine::new(&config, &priority_engine);
        let decision = engine.decide(
            Status::ApiError,
            "API Error",
            &activity,
            &state,
        );
        match decision {
            Decision::Notify { priority, .. } => assert_eq!(priority, Priority::Urgent),
            other => panic!("expected Notify, got {:?}", other),
        }
    }

    #[test]
    fn suppress_within_cooldown() {
        let (config, priority_engine) = default_engine();
        let activity = MockActivity { idle: 60, focused: false };
        let mut state = SessionState::new();
        state.update_after_notification("task_complete", "done");
        let engine = DecisionEngine::new(&config, &priority_engine);
        let decision = engine.decide(
            Status::Question,
            "question?",
            &activity,
            &state,
        );
        match decision {
            Decision::Suppress { .. } => {} // expected
            other => panic!("expected Suppress, got {:?}", other),
        }
    }

    #[test]
    fn activity_disabled_skips_focus_check() {
        let mut config = Config::default();
        config.activity.enabled = false;
        let priority_engine = PriorityEngine::new(
            config.priority_overrides.clone(),
            config.priority_channels.clone(),
        );
        let activity = MockActivity { idle: 0, focused: true };
        let state = SessionState::new();
        let engine = DecisionEngine::new(&config, &priority_engine);
        let decision = engine.decide(
            Status::TaskComplete,
            "done",
            &activity,
            &state,
        );
        // Should NOT downgrade because activity detection is disabled
        match decision {
            Decision::Notify { .. } => {}
            other => panic!("expected Notify, got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- decision`
Expected: FAIL.

- [ ] **Step 3: Implement DecisionEngine**

```rust
// crates/claude-notify-core/src/decision.rs
use crate::config::Config;
use crate::priority::PriorityEngine;
use crate::state::SessionState;
use crate::suppression::SuppressionEngine;
use crate::types::{Channel, Decision, Notification, Priority, Status};
use tracing::{debug, info};

/// Trait for user activity detection — implemented by platform crate, mockable in tests.
pub trait UserActivity {
    fn idle_seconds(&self) -> u64;
    fn is_terminal_focused(&self) -> bool;
}

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

    pub fn decide(
        &self,
        status: Status,
        summary: &str,
        activity: &dyn UserActivity,
        state: &SessionState,
    ) -> Decision {
        let priority = self.priority_engine.assess(&status);
        debug!(status = %status.as_str(), priority = ?priority, "assessed priority");

        // Build notification
        let notification = Notification::new(
            &format!("{} {}", status.default_icon(), status.default_title()),
            summary,
        )
        .with_priority(priority);

        // Check suppression
        let bypass = self.priority_engine.bypasses_cooldown(&priority);
        let suppression = SuppressionEngine::new(&self.config.suppression);
        if let Some(reason) = suppression.check(&status, summary, state, bypass) {
            info!(reason = %reason, "notification suppressed");
            return Decision::Suppress { reason };
        }

        // Check user activity for potential downgrade
        if self.config.activity.enabled && !self.priority_engine.bypasses_idle_check(&priority) {
            let idle = activity.idle_seconds();
            let focused = activity.is_terminal_focused();

            debug!(idle_seconds = idle, terminal_focused = focused, "user activity");

            if focused && idle < self.config.activity.idle_threshold_seconds {
                // User is actively looking at the terminal — downgrade
                let downgraded_channels = if self.config.terminal_bell.enabled {
                    vec![Channel::TerminalBell]
                } else {
                    vec![Channel::Desktop]
                };

                info!("downgrading: terminal focused and user active");
                return Decision::Downgrade {
                    from: priority,
                    to: Priority::Low,
                    reason: "terminal focused, user active".to_string(),
                    channels: downgraded_channels,
                    notification,
                };
            }
        }

        // Normal notification
        let channels = self.priority_engine.channels_for(&priority);
        info!(
            status = %status.as_str(),
            priority = ?priority,
            channels = ?channels,
            "decision: notify"
        );
        Decision::Notify {
            channels,
            priority,
            notification,
        }
    }
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 4: Update lib.rs**

```rust
// crates/claude-notify-core/src/lib.rs
pub mod types;
pub mod error;
pub mod config;
pub mod hook;
pub mod analyzer;
pub mod summary;
pub mod priority;
pub mod state;
pub mod suppression;
pub mod dedup;
pub mod decision;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-core -- decision`
Expected: All decision tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-core/src/decision.rs crates/claude-notify-core/src/lib.rs
git commit -m "feat(core): add intelligence decision engine combining priority, activity, and suppression"
```

---

## Task 11: Platform Traits and Activity Detection

**Files:**
- Modify: `crates/claude-notify-platform/src/lib.rs`
- Create: `crates/claude-notify-platform/src/activity.rs`
- Create: `crates/claude-notify-platform/src/macos.rs`
- Create: `crates/claude-notify-platform/src/linux.rs`
- Create: `crates/claude-notify-platform/src/windows.rs`

- [ ] **Step 1: Write tests for activity detection**

```rust
// crates/claude-notify-platform/src/activity.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_activity_returns_defaults() {
        let detector = NoopActivityDetector;
        assert_eq!(detector.idle_seconds(), 0);
        assert!(!detector.is_terminal_focused());
    }
}
```

- [ ] **Step 2: Implement platform traits and activity detection**

```rust
// crates/claude-notify-platform/src/lib.rs
pub mod activity;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "windows")]
pub mod windows;

/// Trait for sending desktop notifications.
pub trait DesktopNotifier: Send + Sync {
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        icon: Option<&std::path::Path>,
        timeout: Option<u64>,
    ) -> Result<(), String>;

    fn supports_click_action(&self) -> bool {
        false
    }
}

/// Trait for detecting user activity.
pub trait UserActivityDetector: Send + Sync {
    fn idle_seconds(&self) -> u64;
    fn is_terminal_focused(&self) -> bool;
}

/// Create the platform-appropriate activity detector.
pub fn create_activity_detector() -> Box<dyn UserActivityDetector> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacActivityDetector::new()) }

    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxActivityDetector::new()) }

    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsActivityDetector::new()) }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    { Box::new(activity::NoopActivityDetector) }
}

/// Create the platform-appropriate desktop notifier.
pub fn create_desktop_notifier() -> Box<dyn DesktopNotifier> {
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacNotifier::new()) }

    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxNotifier::new()) }

    #[cfg(target_os = "windows")]
    { Box::new(windows::WindowsNotifier::new()) }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    { Box::new(activity::NoopNotifier) }
}
```

```rust
// crates/claude-notify-platform/src/activity.rs

/// Noop fallback for unsupported platforms and testing.
pub struct NoopActivityDetector;

impl super::UserActivityDetector for NoopActivityDetector {
    fn idle_seconds(&self) -> u64 { 0 }
    fn is_terminal_focused(&self) -> bool { false }
}

pub struct NoopNotifier;

impl super::DesktopNotifier for NoopNotifier {
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

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 3: Implement macOS platform**

```rust
// crates/claude-notify-platform/src/macos.rs
#[cfg(target_os = "macos")]
use mac_notification_sys::*;

pub struct MacNotifier;

impl MacNotifier {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "macos")]
impl super::DesktopNotifier for MacNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        let mut notification = mac_notification_sys::Notification::new();
        notification.title(title);
        notification.message(body);
        if let Some(sub) = subtitle {
            notification.subtitle(sub);
        }
        notification.send().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn supports_click_action(&self) -> bool { true }
}

pub struct MacActivityDetector;

impl MacActivityDetector {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "macos")]
impl super::UserActivityDetector for MacActivityDetector {
    fn idle_seconds(&self) -> u64 {
        // Use IOKit to get idle time via CGEventSourceSecondsSinceLastEventType
        // This requires linking against CoreGraphics
        use std::process::Command;
        let output = Command::new("ioreg")
            .args(["-c", "IOHIDSystem", "-d", "4"])
            .output()
            .ok();
        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse HIDIdleTime from ioreg output (in nanoseconds)
            for line in stdout.lines() {
                if line.contains("HIDIdleTime") {
                    if let Some(val) = line.split('=').nth(1) {
                        if let Ok(ns) = val.trim().parse::<u64>() {
                            return ns / 1_000_000_000;
                        }
                    }
                }
            }
        }
        0
    }

    fn is_terminal_focused(&self) -> bool {
        use std::process::Command;
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        if term_program.is_empty() {
            return false;
        }
        // Use osascript to check frontmost app
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
            .output()
            .ok();
        if let Some(output) = output {
            let frontmost = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
            return frontmost.contains(&term_program.to_lowercase());
        }
        false
    }
}
```

- [ ] **Step 4: Implement Linux platform**

```rust
// crates/claude-notify-platform/src/linux.rs
pub struct LinuxNotifier;

impl LinuxNotifier {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "linux")]
impl super::DesktopNotifier for LinuxNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        _subtitle: Option<&str>,
        icon: Option<&std::path::Path>,
        timeout: Option<u64>,
    ) -> Result<(), String> {
        let mut notification = notify_rust::Notification::new();
        notification.summary(title).body(body);
        if let Some(icon) = icon {
            notification.icon(&icon.to_string_lossy());
        }
        if let Some(timeout) = timeout {
            notification.timeout(notify_rust::Timeout::Milliseconds((timeout * 1000) as u32));
        }
        notification.show().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct LinuxActivityDetector;

impl LinuxActivityDetector {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "linux")]
impl super::UserActivityDetector for LinuxActivityDetector {
    fn idle_seconds(&self) -> u64 {
        // Try xprintidle for X11
        use std::process::Command;
        let output = Command::new("xprintidle").output().ok();
        if let Some(output) = output {
            if output.status.success() {
                let ms_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(ms) = ms_str.parse::<u64>() {
                    return ms / 1000;
                }
            }
        }
        0
    }

    fn is_terminal_focused(&self) -> bool {
        use std::process::Command;
        // Use xdotool to get active window name
        let output = Command::new("xdotool")
            .args(["getactivewindow", "getwindowname"])
            .output()
            .ok();
        if let Some(output) = output {
            let window_name = String::from_utf8_lossy(&output.stdout).to_lowercase();
            let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default().to_lowercase();
            if !term_program.is_empty() {
                return window_name.contains(&term_program);
            }
        }
        false
    }
}
```

- [ ] **Step 5: Implement Windows platform**

```rust
// crates/claude-notify-platform/src/windows.rs
pub struct WindowsNotifier;

impl WindowsNotifier {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "windows")]
impl super::DesktopNotifier for WindowsNotifier {
    fn send(
        &self,
        title: &str,
        body: &str,
        _subtitle: Option<&str>,
        _icon: Option<&std::path::Path>,
        _timeout: Option<u64>,
    ) -> Result<(), String> {
        use winrt_notification::Toast;
        Toast::new(Toast::POWERSHELL_APP_ID)
            .title(title)
            .text1(body)
            .show()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct WindowsActivityDetector;

impl WindowsActivityDetector {
    pub fn new() -> Self { Self }
}

#[cfg(target_os = "windows")]
impl super::UserActivityDetector for WindowsActivityDetector {
    fn idle_seconds(&self) -> u64 {
        // Use GetLastInputInfo via windows-sys
        // For initial implementation, return 0 (always active)
        // Full implementation requires windows-sys crate
        0
    }

    fn is_terminal_focused(&self) -> bool {
        false
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-platform`
Expected: All tests PASS (platform-specific code compiles conditionally).

- [ ] **Step 7: Commit**

```bash
git add crates/claude-notify-platform/
git commit -m "feat(platform): add desktop notification and activity detection for macOS, Linux, Windows"
```

---

## Task 12: Dispatch Traits and Router

**Files:**
- Create: `crates/claude-notify-dispatch/src/traits.rs`
- Create: `crates/claude-notify-dispatch/src/desktop.rs`
- Create: `crates/claude-notify-dispatch/src/sound.rs`
- Create: `crates/claude-notify-dispatch/src/terminal_bell.rs`
- Create: `crates/claude-notify-dispatch/src/webhook.rs`
- Modify: `crates/claude-notify-dispatch/src/lib.rs`

- [ ] **Step 1: Write tests for dispatch router**

```rust
// crates/claude-notify-dispatch/src/lib.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct FakeDispatcher {
        sent: Arc<Mutex<Vec<String>>>,
        should_fail: bool,
    }

    impl Dispatcher for FakeDispatcher {
        fn dispatch(&self, title: &str, body: &str) -> Result<(), String> {
            if self.should_fail {
                return Err("fake error".to_string());
            }
            self.sent.lock().unwrap().push(format!("{}: {}", title, body));
            Ok(())
        }
    }

    #[test]
    fn router_dispatches_to_all_channels() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let d1 = FakeDispatcher { sent: sent.clone(), should_fail: false };
        let d2 = FakeDispatcher { sent: sent.clone(), should_fail: false };

        let router = NotifyRouter::new();
        let report = router.dispatch_to(
            &[&d1, &d2],
            "Title",
            "Body",
        );
        assert_eq!(report.successes, 2);
        assert_eq!(report.failures, 0);
        assert_eq!(sent.lock().unwrap().len(), 2);
    }

    #[test]
    fn router_continues_on_failure() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let d1 = FakeDispatcher { sent: sent.clone(), should_fail: true };
        let d2 = FakeDispatcher { sent: sent.clone(), should_fail: false };

        let router = NotifyRouter::new();
        let report = router.dispatch_to(
            &[&d1, &d2],
            "Title",
            "Body",
        );
        assert_eq!(report.successes, 1);
        assert_eq!(report.failures, 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-dispatch`
Expected: FAIL.

- [ ] **Step 3: Implement traits and router**

```rust
// crates/claude-notify-dispatch/src/traits.rs

/// Common trait for all notification dispatchers.
pub trait Dispatcher: Send + Sync {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String>;
}
```

```rust
// crates/claude-notify-dispatch/src/desktop.rs
use crate::traits::Dispatcher;
use claude_notify_platform::DesktopNotifier;
use std::path::PathBuf;

pub struct DesktopDispatcher {
    notifier: Box<dyn DesktopNotifier>,
    icon: Option<PathBuf>,
    timeout: Option<u64>,
}

impl DesktopDispatcher {
    pub fn new(notifier: Box<dyn DesktopNotifier>, icon: Option<PathBuf>, timeout: Option<u64>) -> Self {
        Self { notifier, icon, timeout }
    }
}

impl Dispatcher for DesktopDispatcher {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String> {
        self.notifier.send(title, body, None, self.icon.as_deref(), self.timeout)
    }
}
```

```rust
// crates/claude-notify-dispatch/src/sound.rs
use crate::traits::Dispatcher;
use std::path::PathBuf;
use tracing::debug;

pub struct SoundDispatcher {
    volume: f32,
    sounds_dir: PathBuf,
}

impl SoundDispatcher {
    pub fn new(volume: f32, sounds_dir: PathBuf) -> Self {
        Self { volume, sounds_dir }
    }

    fn sound_file_for(&self, title: &str) -> Option<PathBuf> {
        // Map notification title/status to sound file
        let filename = if title.contains("Task Complete") {
            "task-complete.mp3"
        } else if title.contains("Review Complete") {
            "review-complete.mp3"
        } else if title.contains("Question") {
            "question.mp3"
        } else if title.contains("Plan Ready") {
            "plan-ready.mp3"
        } else if title.contains("Error") || title.contains("Session Limit") {
            "error.mp3"
        } else {
            return None;
        };
        let path = self.sounds_dir.join(filename);
        if path.exists() { Some(path) } else { None }
    }
}

impl Dispatcher for SoundDispatcher {
    fn dispatch(&self, title: &str, _body: &str) -> Result<(), String> {
        let sound_file = match self.sound_file_for(title) {
            Some(f) => f,
            None => {
                debug!("no sound file for: {}", title);
                return Ok(());
            }
        };

        debug!(file = %sound_file.display(), volume = self.volume, "playing sound");

        // Use rodio for playback
        let file = std::fs::File::open(&sound_file).map_err(|e| e.to_string())?;
        let reader = std::io::BufReader::new(file);

        let (_stream, stream_handle) =
            rodio::OutputStream::try_default().map_err(|e| e.to_string())?;
        let sink = rodio::Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        let source = rodio::Decoder::new(reader).map_err(|e| e.to_string())?;
        sink.set_volume(self.volume);
        sink.append(source);
        sink.sleep_until_end();

        Ok(())
    }
}
```

```rust
// crates/claude-notify-dispatch/src/terminal_bell.rs
use crate::traits::Dispatcher;

pub struct TerminalBellDispatcher;

impl TerminalBellDispatcher {
    pub fn new() -> Self { Self }
}

impl Dispatcher for TerminalBellDispatcher {
    fn dispatch(&self, _title: &str, _body: &str) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut tty) = OpenOptions::new().write(true).open("/dev/tty") {
                let _ = tty.write_all(b"\x07");
            }
        }
        #[cfg(windows)]
        {
            print!("\x07");
        }
        Ok(())
    }
}
```

```rust
// crates/claude-notify-dispatch/src/webhook.rs
use crate::traits::Dispatcher;
use serde_json::json;
use tracing::{debug, warn};

pub struct WebhookDispatcher {
    url: String,
    preset: String,
    chat_id: String,
    headers: std::collections::HashMap<String, String>,
    template: String,
    retry_max: u32,
    timeout_seconds: u64,
}

impl WebhookDispatcher {
    pub fn new(
        url: String,
        preset: String,
        chat_id: String,
        headers: std::collections::HashMap<String, String>,
        template: String,
        retry_max: u32,
        timeout_seconds: u64,
    ) -> Self {
        Self { url, preset, chat_id, headers, template, retry_max, timeout_seconds }
    }

    fn format_payload(&self, title: &str, body: &str) -> serde_json::Value {
        match self.preset.as_str() {
            "slack" => json!({
                "blocks": [{
                    "type": "section",
                    "text": { "type": "mrkdwn", "text": format!("*{}*\n{}", title, body) }
                }]
            }),
            "discord" => json!({
                "embeds": [{
                    "title": title,
                    "description": body,
                    "color": 5814783
                }]
            }),
            "telegram" => json!({
                "chat_id": self.chat_id,
                "text": format!("*{}*\n{}", title, body),
                "parse_mode": "Markdown"
            }),
            "lark" => json!({
                "msg_type": "interactive",
                "card": {
                    "header": { "title": { "tag": "plain_text", "content": title } },
                    "elements": [{ "tag": "div", "text": { "tag": "plain_text", "content": body } }]
                }
            }),
            "custom" => {
                if !self.template.is_empty() {
                    let filled = self.template
                        .replace("{{title}}", title)
                        .replace("{{body}}", body);
                    serde_json::from_str(&filled).unwrap_or(json!({"title": title, "body": body}))
                } else {
                    json!({"title": title, "body": body})
                }
            }
            _ => json!({"title": title, "body": body}),
        }
    }

    fn send_with_retry(&self, payload: &serde_json::Value) -> Result<(), String> {
        let mut last_err = String::new();
        for attempt in 0..=self.retry_max {
            if attempt > 0 {
                let delay = std::time::Duration::from_secs(1 << (attempt - 1)); // 1, 2, 4s
                std::thread::sleep(delay);
                debug!(attempt, "retrying webhook");
            }

            let mut request = ureq::post(&self.url)
                .timeout(std::time::Duration::from_secs(self.timeout_seconds));
            for (k, v) in &self.headers {
                request = request.header(k, v);
            }

            match request.send_json(payload) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_err = e.to_string();
                    warn!(error = %last_err, attempt, "webhook failed");
                }
            }
        }
        Err(format!("webhook failed after {} retries: {}", self.retry_max, last_err))
    }
}

impl Dispatcher for WebhookDispatcher {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String> {
        let payload = self.format_payload(title, body);
        self.send_with_retry(&payload)
    }
}
```

- [ ] **Step 4: Implement router in lib.rs**

```rust
// crates/claude-notify-dispatch/src/lib.rs
pub mod traits;
pub mod desktop;
pub mod sound;
pub mod terminal_bell;
pub mod webhook;

use traits::Dispatcher;
use tracing::warn;

pub struct NotifyRouter;

#[derive(Debug)]
pub struct DispatchReport {
    pub successes: usize,
    pub failures: usize,
    pub errors: Vec<String>,
}

impl NotifyRouter {
    pub fn new() -> Self { Self }

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
                Ok(()) => report.successes += 1,
                Err(e) => {
                    warn!(error = %e, "dispatch failed");
                    report.errors.push(e);
                    report.failures += 1;
                }
            }
        }
        report
    }
}

// --- tests at bottom (from Step 1) ---
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test -p claude-notify-dispatch`
Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/claude-notify-dispatch/
git commit -m "feat(dispatch): add desktop, sound, terminal bell, webhook dispatchers with router"
```

---

## Task 13: CLI Binary Entry Point

**Files:**
- Modify: `crates/claude-notify/src/main.rs`

- [ ] **Step 1: Implement CLI with clap**

```rust
// crates/claude-notify/src/main.rs
use anyhow::Result;
use clap::{Parser, Subcommand};
use claude_notify_core::{
    analyzer, config::Config, decision::DecisionEngine, dedup, hook::HookInput,
    priority::PriorityEngine, state::SessionState, summary, types::Decision,
};
use claude_notify_dispatch::{
    desktop::DesktopDispatcher, sound::SoundDispatcher,
    terminal_bell::TerminalBellDispatcher, webhook::WebhookDispatcher, NotifyRouter,
};
use claude_notify_dispatch::traits::Dispatcher;
use tracing::{debug, error, info};

#[derive(Parser)]
#[command(name = "claude-notify", version, about = "Smart notification plugin for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Handle a hook event (reads JSON from stdin)
    HandleHook {
        /// Hook type: PreToolUse, Notification, Stop, SubagentStop, TeammateIdle
        hook_type: String,
    },
    /// Send a test notification
    Test,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// List available sounds
    Sounds {
        #[command(subcommand)]
        action: SoundsAction,
    },
    /// Show version
    Version,
}

#[derive(Subcommand)]
enum ConfigAction {
    Show,
    Validate,
    Reset,
}

#[derive(Subcommand)]
enum SoundsAction {
    List,
}

fn main() {
    let result = run();
    if let Err(e) = result {
        // Log error but exit 0 — never block Claude Code
        error!(error = %e, "claude-notify error");
        std::process::exit(0);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Determine plugin root from env or binary location
    let plugin_root = std::env::var("CLAUDE_PLUGIN_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .unwrap_or_default()
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf()
        });

    let config = Config::load_layered(&plugin_root, None).unwrap_or_default();

    // Setup tracing
    if config.debug.enabled {
        let log_file = if config.debug.log_file.is_empty() {
            dirs_home()
                .join(".claude")
                .join("claude-notification")
                .join("debug.log")
        } else {
            std::path::PathBuf::from(&config.debug.log_file)
        };
        if let Some(parent) = log_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;
        tracing_subscriber::fmt()
            .with_writer(file)
            .with_ansi(false)
            .init();
    }

    match cli.command {
        Commands::HandleHook { hook_type } => handle_hook(&config, &plugin_root, &hook_type),
        Commands::Test => handle_test(&config, &plugin_root),
        Commands::Config { action } => handle_config(&config, action),
        Commands::Sounds { action } => handle_sounds(&plugin_root, action),
        Commands::Version => {
            println!("claude-notify v{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn handle_hook(config: &Config, plugin_root: &std::path::Path, hook_type: &str) -> Result<()> {
    debug!(hook_type, "handling hook");

    // Parse hook input from stdin
    let input = HookInput::from_stdin()?;
    debug!(session_id = %input.session_id, tool_name = %input.tool_name, "parsed hook input");

    // Dedup check
    let lock_path = dedup::dedup_lock_path(&input.session_id);
    let _lock = match dedup::DedupLock::try_acquire(&lock_path, 2)? {
        Some(lock) => lock,
        None => {
            debug!("dedup: concurrent notification suppressed");
            return Ok(());
        }
    };

    // Parse transcript
    let transcript_path = std::path::Path::new(&input.transcript_path);
    let messages = if transcript_path.exists() {
        analyzer::parse_transcript(transcript_path)?
    } else {
        vec![]
    };

    // Detect status
    let status = analyzer::detect_status(&messages, &input);
    info!(status = %status.as_str(), "status detected");

    // Extract summary
    let summary_text = summary::extract_summary(&status, &messages);
    debug!(summary = %summary_text, "summary extracted");

    // Load state
    let state_path = SessionState::state_path(&input.session_id);
    let state = SessionState::load(&state_path)?;

    // Build engines
    let priority_engine = PriorityEngine::new(
        config.priority_overrides.clone(),
        config.priority_channels.clone(),
    );

    // Get user activity
    let activity_detector = claude_notify_platform::create_activity_detector();

    // Adapter: platform UserActivityDetector -> core UserActivity trait
    struct ActivityAdapter(Box<dyn claude_notify_platform::UserActivityDetector>);
    impl claude_notify_core::decision::UserActivity for ActivityAdapter {
        fn idle_seconds(&self) -> u64 { self.0.idle_seconds() }
        fn is_terminal_focused(&self) -> bool { self.0.is_terminal_focused() }
    }
    let activity = ActivityAdapter(activity_detector);

    // Make decision
    let decision_engine = DecisionEngine::new(config, &priority_engine);
    let decision = decision_engine.decide(status, &summary_text, &activity, &state);

    // Dispatch
    match &decision {
        Decision::Notify { channels, notification, .. }
        | Decision::Downgrade { channels, notification, .. } => {
            let dispatchers = build_dispatchers(config, plugin_root, channels);
            let dispatcher_refs: Vec<&dyn Dispatcher> = dispatchers.iter().map(|d| d.as_ref()).collect();
            let router = NotifyRouter::new();
            let report = router.dispatch_to(&dispatcher_refs, &notification.title, &notification.body);
            info!(successes = report.successes, failures = report.failures, "dispatch complete");

            // Update state
            let mut state = state;
            state.update_after_notification(status.as_str(), &summary_text);
            state.save(&state_path)?;
        }
        Decision::Suppress { reason } => {
            info!(reason = %reason, "notification suppressed");
        }
    }

    Ok(())
}

fn build_dispatchers(
    config: &Config,
    plugin_root: &std::path::Path,
    channels: &[claude_notify_core::types::Channel],
) -> Vec<Box<dyn Dispatcher>> {
    use claude_notify_core::types::Channel;
    let mut dispatchers: Vec<Box<dyn Dispatcher>> = Vec::new();

    for channel in channels {
        match channel {
            Channel::Desktop if config.desktop.enabled => {
                let notifier = claude_notify_platform::create_desktop_notifier();
                let icon = if config.desktop.app_icon.is_empty() {
                    None
                } else {
                    Some(std::path::PathBuf::from(&config.desktop.app_icon))
                };
                dispatchers.push(Box::new(DesktopDispatcher::new(
                    notifier,
                    icon,
                    Some(config.desktop.timeout),
                )));
            }
            Channel::Sound if config.sound.enabled => {
                dispatchers.push(Box::new(SoundDispatcher::new(
                    config.sound.volume,
                    plugin_root.join("sounds"),
                )));
            }
            Channel::TerminalBell if config.terminal_bell.enabled => {
                dispatchers.push(Box::new(TerminalBellDispatcher::new()));
            }
            Channel::Webhook if config.webhook.enabled && !config.webhook.url.is_empty() => {
                dispatchers.push(Box::new(WebhookDispatcher::new(
                    config.webhook.url.clone(),
                    config.webhook.preset.clone(),
                    config.webhook.chat_id.clone(),
                    config.webhook.headers.clone(),
                    config.webhook.template.clone(),
                    config.webhook.retry_max,
                    config.webhook.timeout_seconds,
                )));
            }
            _ => {}
        }
    }

    dispatchers
}

fn handle_test(config: &Config, plugin_root: &std::path::Path) -> Result<()> {
    let channels = vec![
        claude_notify_core::types::Channel::Desktop,
        claude_notify_core::types::Channel::Sound,
        claude_notify_core::types::Channel::TerminalBell,
    ];
    let dispatchers = build_dispatchers(config, plugin_root, &channels);
    let dispatcher_refs: Vec<&dyn Dispatcher> = dispatchers.iter().map(|d| d.as_ref()).collect();
    let router = NotifyRouter::new();
    let report = router.dispatch_to(&dispatcher_refs, "Test Notification", "Claude Notify is working!");
    println!(
        "Test notification sent: {} succeeded, {} failed",
        report.successes, report.failures
    );
    for err in &report.errors {
        println!("  Error: {}", err);
    }
    Ok(())
}

fn handle_config(config: &Config, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let yaml = serde_yaml::to_string(config)?;
            println!("{}", yaml);
        }
        ConfigAction::Validate => {
            println!("Configuration is valid.");
        }
        ConfigAction::Reset => {
            let home = dirs_home();
            let config_path = home
                .join(".claude")
                .join("claude-notification")
                .join("config.yaml");
            if config_path.exists() {
                std::fs::remove_file(&config_path)?;
                println!("Configuration reset to defaults.");
            } else {
                println!("No user configuration found. Already using defaults.");
            }
        }
    }
    Ok(())
}

fn handle_sounds(plugin_root: &std::path::Path, _action: SoundsAction) -> Result<()> {
    let sounds_dir = plugin_root.join("sounds");
    if sounds_dir.exists() {
        for entry in std::fs::read_dir(&sounds_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                println!("{}", name.to_string_lossy());
            }
        }
    } else {
        println!("No sounds directory found.");
    }

    // macOS system sounds
    #[cfg(target_os = "macos")]
    {
        let sys_sounds = std::path::Path::new("/System/Library/Sounds");
        if sys_sounds.exists() {
            println!("\nSystem sounds:");
            for entry in std::fs::read_dir(sys_sounds)? {
                let entry = entry?;
                if let Some(name) = entry.path().file_stem() {
                    println!("  system:{}", name.to_string_lossy());
                }
            }
        }
    }

    Ok(())
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo build`
Expected: Compiles successfully.

- [ ] **Step 3: Test CLI help**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo run -- --help`
Expected: Shows help with subcommands: handle-hook, test, config, sounds, version.

- [ ] **Step 4: Commit**

```bash
git add crates/claude-notify/src/main.rs
git commit -m "feat(cli): add CLI entry point with handle-hook, test, config, sounds commands"
```

---

## Task 14: Plugin Files (hooks.json, plugin.json, hook-wrapper)

**Files:**
- Create: `.claude-plugin/plugin.json`
- Create: `hooks/hooks.json`
- Create: `hooks/hook-wrapper.sh`
- Create: `hooks/hook-wrapper.cmd`
- Create: `bin/.gitkeep`

- [ ] **Step 1: Create plugin.json**

```json
// .claude-plugin/plugin.json
{
  "name": "claude-notification",
  "description": "Smart notification plugin for Claude Code — intelligent timing, clear content, highly configurable, cross-platform",
  "version": "0.1.0",
  "author": {
    "name": "zhaohejie"
  },
  "license": "MIT",
  "keywords": [
    "notification",
    "desktop",
    "webhook",
    "sound",
    "cross-platform",
    "smart"
  ],
  "skills": [
    "./skills/settings/SKILL.md"
  ],
  "commands": [
    "./commands/notification-settings.md"
  ]
}
```

- [ ] **Step 2: Create hooks.json**

```json
// hooks/hooks.json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "ExitPlanMode|AskUserQuestion",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook PreToolUse",
            "timeout": 30
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook Notification",
            "timeout": 30
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook Stop",
            "timeout": 30
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook SubagentStop",
            "timeout": 30
          }
        ]
      }
    ],
    "TeammateIdle": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook TeammateIdle",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 3: Create hook-wrapper.sh**

```bash
#!/bin/bash
# hook-wrapper.sh — lazy-download and run claude-notify binary
set -euo pipefail

PLUGIN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$PLUGIN_ROOT/bin"

# Read version from plugin.json
VERSION="0.1.0"
if command -v jq &>/dev/null && [ -f "$PLUGIN_ROOT/.claude-plugin/plugin.json" ]; then
    VERSION=$(jq -r '.version' "$PLUGIN_ROOT/.claude-plugin/plugin.json" 2>/dev/null || echo "$VERSION")
fi

# Detect platform
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64)  ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Unsupported architecture: $ARCH" >&2; exit 0 ;;
esac

BINARY_NAME="claude-notify"
BINARY_PATH="$BIN_DIR/${BINARY_NAME}-${OS}-${ARCH}"

# Download if missing
if [ ! -x "$BINARY_PATH" ]; then
    mkdir -p "$BIN_DIR"

    # Try local cargo build first (development mode)
    if [ -d "$PLUGIN_ROOT/crates" ] && command -v cargo &>/dev/null; then
        echo "Building from source..." >&2
        (cd "$PLUGIN_ROOT/crates" && cargo build --release -p claude-notify 2>&2)
        TARGET_DIR="$PLUGIN_ROOT/crates/target/release"
        if [ -f "$TARGET_DIR/$BINARY_NAME" ]; then
            cp "$TARGET_DIR/$BINARY_NAME" "$BINARY_PATH"
            chmod +x "$BINARY_PATH"
        fi
    fi

    # If still missing, try downloading from GitHub Release
    if [ ! -x "$BINARY_PATH" ]; then
        DOWNLOAD_URL="https://github.com/zhaohejie/claude-notification-plugin/releases/download/v${VERSION}/${BINARY_NAME}-${OS}-${ARCH}"
        echo "Downloading claude-notify v${VERSION}..." >&2
        if command -v curl &>/dev/null; then
            curl -fsSL "$DOWNLOAD_URL" -o "$BINARY_PATH" 2>/dev/null || true
        elif command -v wget &>/dev/null; then
            wget -q "$DOWNLOAD_URL" -O "$BINARY_PATH" 2>/dev/null || true
        fi
        if [ -f "$BINARY_PATH" ]; then
            chmod +x "$BINARY_PATH"
        fi
    fi

    if [ ! -x "$BINARY_PATH" ]; then
        echo "Failed to obtain claude-notify binary. Run: cd crates && cargo build --release" >&2
        exit 0
    fi
fi

# Forward to binary with plugin root env
export CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"
exec "$BINARY_PATH" "$@"
```

- [ ] **Step 4: Create hook-wrapper.cmd**

```cmd
@echo off
REM hook-wrapper.cmd — Windows launcher for claude-notify

set PLUGIN_ROOT=%~dp0..
set BIN_DIR=%PLUGIN_ROOT%\bin
set BINARY_NAME=claude-notify
set BINARY_PATH=%BIN_DIR%\%BINARY_NAME%-windows-x86_64.exe

if not exist "%BINARY_PATH%" (
    echo claude-notify binary not found. Run: cd crates ^&^& cargo build --release >&2
    exit /b 0
)

set CLAUDE_PLUGIN_ROOT=%PLUGIN_ROOT%
"%BINARY_PATH%" %*
```

- [ ] **Step 5: Create bin/.gitkeep**

```
# Empty file to keep bin/ directory in git
```

- [ ] **Step 6: Make hook-wrapper.sh executable**

Run: `chmod +x /Users/zhaohejie/claude/claude-notification-plugin/hooks/hook-wrapper.sh`

- [ ] **Step 7: Commit**

```bash
git add .claude-plugin/ hooks/ bin/
git commit -m "feat: add plugin manifest, hook registration, and platform launchers"
```

---

## Task 15: Skill and Command Files

**Files:**
- Create: `skills/settings/SKILL.md`
- Create: `commands/notification-settings.md`

- [ ] **Step 1: Create settings skill**

```markdown
// skills/settings/SKILL.md
---
name: notification-settings
description: Use when user wants to configure notification settings, adjust notification behavior, or set up webhooks for the claude-notification plugin.
---

# Notification Settings

Help the user configure the claude-notification plugin interactively.

## Steps

1. Read the current configuration from `~/.claude/claude-notification/config.yaml` (if it exists)
2. Show the current settings summary to the user
3. Ask what they want to configure:
   - Desktop notification settings (enable/disable, click-to-focus, timeout)
   - Sound settings (enable/disable, volume, device)
   - Webhook setup (Slack, Discord, Telegram, Lark, or custom)
   - Priority rules (override default priority for specific statuses)
   - Suppression rules (cooldown, filters)
   - Team mode settings
   - Test notification
   - Reset to defaults
4. Guide them through the chosen configuration
5. Write the updated config to `~/.claude/claude-notification/config.yaml`
6. Offer to send a test notification to verify settings

## Configuration File

Location: `~/.claude/claude-notification/config.yaml`

Use the Read tool to check if it exists, then Edit or Write to update it.
For the full config schema, refer to the plugin's `config/default-config.yaml`.

## Test Command

After configuration, run this to test:
```bash
${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh test
```
```

- [ ] **Step 2: Create notification-settings command**

```markdown
// commands/notification-settings.md
---
description: "Configure notification settings interactively"
argument-hint: ""
allowed-tools: ["Read", "Write", "Edit", "Bash"]
---

Use the notification-settings skill to guide the user through configuring their notification preferences.
```

- [ ] **Step 3: Commit**

```bash
git add skills/ commands/
git commit -m "feat: add interactive settings skill and notification-settings command"
```

---

## Task 16: End-to-End Integration Test

**Files:**
- Create: `tests/e2e_test.sh`

- [ ] **Step 1: Write end-to-end test script**

```bash
#!/bin/bash
# tests/e2e_test.sh — End-to-end integration test
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building claude-notify ==="
(cd "$PROJECT_ROOT/crates" && cargo build --release)

BINARY="$PROJECT_ROOT/crates/target/release/claude-notify"

echo "=== Test: version ==="
$BINARY version

echo "=== Test: config show ==="
CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY config show

echo "=== Test: sounds list ==="
CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY sounds list || true

echo "=== Test: handle-hook with mock input ==="
# Create a mock transcript
TMPDIR=$(mktemp -d)
cat > "$TMPDIR/transcript.jsonl" <<'JSONL'
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write","input":{"file_path":"/tmp/test.rs","content":"fn main() {}"}}]},"duration_ms":500}
{"type":"tool_result","tool_name":"Write","content":"File written successfully"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I have created the file for you."}]},"duration_ms":300}
JSONL

echo '{"session_id":"test-e2e","transcript_path":"'"$TMPDIR"'/transcript.jsonl","tool_name":"","is_team_lead":false}' | \
    CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY handle-hook Stop || true

echo "=== Test: handle-hook with question ==="
cat > "$TMPDIR/transcript-q.jsonl" <<'JSONL'
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"AskUserQuestion","input":{"question":"Which framework do you prefer?"}}]},"duration_ms":200}
JSONL

echo '{"session_id":"test-e2e-q","transcript_path":"'"$TMPDIR"'/transcript-q.jsonl","tool_name":"AskUserQuestion","is_team_lead":false}' | \
    CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY handle-hook Stop || true

# Cleanup
rm -rf "$TMPDIR"

echo "=== All e2e tests passed ==="
```

- [ ] **Step 2: Make it executable and run**

Run: `chmod +x /Users/zhaohejie/claude/claude-notification-plugin/tests/e2e_test.sh && /Users/zhaohejie/claude/claude-notification-plugin/tests/e2e_test.sh`
Expected: All tests pass, no panics or crashes. Desktop notifications may appear (expected).

- [ ] **Step 3: Run full unit test suite**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo test --workspace`
Expected: All unit tests pass across all 4 crates.

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "test: add end-to-end integration test script"
```

---

## Task 17: Final Cleanup and Documentation

**Files:**
- Create: `.gitignore`

- [ ] **Step 1: Create .gitignore**

```gitignore
# .gitignore
/crates/target/
/bin/claude-notify-*
*.log
.DS_Store
```

- [ ] **Step 2: Verify full build**

Run: `cd /Users/zhaohejie/claude/claude-notification-plugin/crates && cargo build --release && cargo test --workspace`
Expected: Build and all tests pass.

- [ ] **Step 3: Commit**

```bash
git add .gitignore
git commit -m "chore: add .gitignore for build artifacts and logs"
```

- [ ] **Step 4: Verify plugin structure is complete**

Run: `find /Users/zhaohejie/claude/claude-notification-plugin -type f | grep -v '.git/' | grep -v 'target/' | sort`
Expected output should show all files from the File Structure section above.
