// claude-notify-core: configuration system

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::types::Priority;

// ─── Sub-structs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopConfig {
    /// Enable desktop (banner) notifications
    pub enabled: bool,
    /// Click notification to focus terminal window
    pub click_to_focus: bool,
    /// Terminal app bundle ID (e.g. "com.apple.Terminal")
    pub terminal_bundle_id: String,
    /// Custom app icon path (leave empty to use system default)
    pub app_icon: String,
    /// Notification display duration in seconds (0 = persistent)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundConfig {
    /// Enable sound alerts
    pub enabled: bool,
    /// Volume level (0.0 – 1.0)
    pub volume: f64,
    /// Audio output device name (empty = system default)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalBellConfig {
    /// Send terminal bell character on notification
    pub enabled: bool,
}

impl Default for TerminalBellConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookConfig {
    /// Enable webhook notifications
    pub enabled: bool,
    /// Preset template name (e.g. "slack", "discord", "teams")
    pub preset: String,
    /// Webhook URL (required when enabled)
    pub url: String,
    /// Chat/channel ID (used by some platforms)
    pub chat_id: String,
    /// Additional HTTP headers (e.g. Authorization)
    pub headers: HashMap<String, String>,
    /// Custom payload template (Handlebars / leave empty to use preset)
    pub template: String,
    /// Maximum retry attempts on failure
    pub retry_max: u32,
    /// HTTP request timeout in seconds
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

/// Per-status overrides that can change behaviour for a specific status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusOverride {
    /// Override the enabled state for this status
    pub enabled: Option<bool>,
    /// Override the sound file for this status
    pub sound: Option<String>,
    /// Override the notification title for this status
    pub title: Option<String>,
}

impl Default for StatusOverride {
    fn default() -> Self {
        Self {
            enabled: None,
            sound: None,
            title: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityConfig {
    /// Enable activity / focus detection
    pub enabled: bool,
    /// Seconds of inactivity before user is considered idle
    pub idle_threshold_seconds: u64,
    /// Suppress notifications when Claude's terminal is focused
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

/// A single filter rule for notification suppression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SuppressionFilter {
    /// Match a specific status (snake_case, e.g. "task_complete")
    pub status: Option<String>,
    /// Match on current git branch name
    pub git_branch: Option<String>,
    /// Match on project folder path (substring match)
    pub folder: Option<String>,
}

impl Default for SuppressionFilter {
    fn default() -> Self {
        Self {
            status: None,
            git_branch: None,
            folder: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SuppressionConfig {
    /// Minimum seconds between repeated identical notifications
    pub cooldown_seconds: u64,
    /// Cooldown when transitioning from a task event to a question event
    pub task_to_question_cooldown: u64,
    /// Seconds within which duplicate content is suppressed
    pub content_dedup_seconds: u64,
    /// List of filter rules; a notification is suppressed if any rule matches
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TeamConfig {
    /// Notification mode: "always", "on_error", "never"
    pub mode: String,
    /// Send notifications when a subagent event occurs
    pub notify_on_subagent: bool,
    /// Suppress all notifications that originate from a subagent
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DebugConfig {
    /// Enable debug logging
    pub enabled: bool,
    /// Path to debug log file (empty = stderr only)
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

// ─── Top-level Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub desktop: DesktopConfig,
    pub sound: SoundConfig,
    pub terminal_bell: TerminalBellConfig,
    pub webhook: WebhookConfig,
    pub activity: ActivityConfig,
    pub suppression: SuppressionConfig,
    pub team: TeamConfig,
    pub debug: DebugConfig,

    /// Override the priority for specific statuses (key = status name)
    pub priority_overrides: HashMap<String, Priority>,

    /// Per-channel overrides keyed by status name
    /// e.g. `priority_channels["question"]["desktop"] = false`
    pub priority_channels: HashMap<String, HashMap<String, bool>>,

    /// Per-status overrides (title, sound, enabled flag)
    pub status_overrides: HashMap<String, StatusOverride>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop: DesktopConfig::default(),
            sound: SoundConfig::default(),
            terminal_bell: TerminalBellConfig::default(),
            webhook: WebhookConfig::default(),
            activity: ActivityConfig::default(),
            suppression: SuppressionConfig::default(),
            team: TeamConfig::default(),
            debug: DebugConfig::default(),
            priority_overrides: HashMap::new(),
            priority_channels: HashMap::new(),
            status_overrides: HashMap::new(),
        }
    }
}

// ─── Loading ──────────────────────────────────────────────────────────────────

impl Config {
    /// Load config from a YAML file.  Returns `Default::default()` if the
    /// file does not exist; propagates any other I/O or parse error.
    pub fn load_from_file(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| {
            crate::NotifyError::Io(e)
        })?;
        let cfg: Config = serde_yaml::from_str(&text).map_err(|e| {
            crate::NotifyError::Config(format!("YAML parse error in {:?}: {e}", path))
        })?;
        Ok(cfg)
    }

    /// Layer configs: defaults < plugin built-in < user global < project local.
    ///
    /// - `plugin_root` – directory where the plugin is installed (contains
    ///   `config/default-config.yaml` relative to itself).
    /// - `project_root` – the currently active project directory.
    pub fn load_layered(
        plugin_root: impl AsRef<Path>,
        project_root: impl AsRef<Path>,
    ) -> crate::Result<Self> {
        // Start with Rust defaults
        let base_value = serde_yaml::to_value(Config::default()).map_err(|e| {
            crate::NotifyError::Config(format!("failed to serialize default config: {e}"))
        })?;

        // Layer 1: plugin built-in YAML
        let builtin_path = plugin_root.as_ref().join("config").join("default-config.yaml");
        let builtin_value = yaml_value_from_file_or_empty(&builtin_path)?;

        // Layer 2: user global (~/.claude/claude-notification/config.yaml)
        let user_global_path = home_dir()
            .map(|h| h.join(".claude").join("claude-notification").join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from(""));
        let user_value = if user_global_path.as_os_str().is_empty() {
            Value::Null
        } else {
            yaml_value_from_file_or_empty(&user_global_path)?
        };

        // Layer 3: project local
        let project_local_path = project_root.as_ref().join(".claude-notification.yaml");
        let project_value = yaml_value_from_file_or_empty(&project_local_path)?;

        // Deep-merge all layers in order
        let merged = [builtin_value, user_value, project_value]
            .into_iter()
            .fold(base_value, deep_merge_yaml);

        let cfg: Config = serde_yaml::from_value(merged).map_err(|e| {
            crate::NotifyError::Config(format!("failed to deserialize merged config: {e}"))
        })?;

        Ok(cfg)
    }

    /// Deep-merge `other` on top of `self`, returning the merged result.
    pub fn merge(self, other: Config) -> crate::Result<Config> {
        let base = serde_yaml::to_value(self).map_err(|e| {
            crate::NotifyError::Config(format!("merge: serialize base failed: {e}"))
        })?;
        let overlay = serde_yaml::to_value(other).map_err(|e| {
            crate::NotifyError::Config(format!("merge: serialize overlay failed: {e}"))
        })?;
        let merged = deep_merge_yaml(base, overlay);
        let cfg: Config = serde_yaml::from_value(merged).map_err(|e| {
            crate::NotifyError::Config(format!("merge: deserialize result failed: {e}"))
        })?;
        Ok(cfg)
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Recursively merge `overlay` onto `base`.
/// - Mapping entries in `overlay` override those in `base`.
/// - Sequences and scalars in `overlay` replace those in `base`.
pub fn deep_merge_yaml(base: Value, overlay: Value) -> Value {
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
        // Null overlay means "no override" – keep base
        (base, Value::Null) => base,
        // For everything else (scalars, sequences), overlay wins
        (_, overlay) => overlay,
    }
}

/// Return the user's home directory, checking HOME then USERPROFILE env vars.
pub fn dirs_path() -> Option<PathBuf> {
    home_dir()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn yaml_value_from_file_or_empty(path: &Path) -> crate::Result<Value> {
    if !path.exists() {
        return Ok(Value::Null);
    }
    let text = std::fs::read_to_string(path).map_err(|e| crate::NotifyError::Io(e))?;
    let value: Value = serde_yaml::from_str(&text).map_err(|e| {
        crate::NotifyError::Config(format!("YAML parse error in {:?}: {e}", path))
    })?;
    Ok(value)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = Config::default();

        // Desktop
        assert!(cfg.desktop.enabled);
        assert!(cfg.desktop.click_to_focus);
        assert_eq!(cfg.desktop.terminal_bundle_id, "");
        assert_eq!(cfg.desktop.app_icon, "");
        assert_eq!(cfg.desktop.timeout, 5);

        // Sound
        assert!(cfg.sound.enabled);
        assert!((cfg.sound.volume - 0.8).abs() < f64::EPSILON);
        assert_eq!(cfg.sound.device, "");

        // Terminal bell
        assert!(cfg.terminal_bell.enabled);

        // Webhook
        assert!(!cfg.webhook.enabled);
        assert_eq!(cfg.webhook.preset, "slack");
        assert_eq!(cfg.webhook.url, "");
        assert_eq!(cfg.webhook.retry_max, 3);
        assert_eq!(cfg.webhook.timeout_seconds, 10);

        // Activity
        assert!(cfg.activity.enabled);
        assert_eq!(cfg.activity.idle_threshold_seconds, 30);
        assert!(cfg.activity.suppress_when_focused);

        // Suppression
        assert_eq!(cfg.suppression.cooldown_seconds, 7);
        assert_eq!(cfg.suppression.task_to_question_cooldown, 12);
        assert_eq!(cfg.suppression.content_dedup_seconds, 180);
        assert!(cfg.suppression.filters.is_empty());

        // Team
        assert_eq!(cfg.team.mode, "always");
        assert!(!cfg.team.notify_on_subagent);
        assert!(cfg.team.suppress_for_subagents);

        // Debug
        assert!(!cfg.debug.enabled);
        assert_eq!(cfg.debug.log_file, "");

        // Maps
        assert!(cfg.priority_overrides.is_empty());
        assert!(cfg.priority_channels.is_empty());
        assert!(cfg.status_overrides.is_empty());
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
        let cfg: Config = serde_yaml::from_str(yaml).expect("should parse");

        // Specified values
        assert!(!cfg.desktop.enabled);
        assert_eq!(cfg.desktop.timeout, 10);
        assert!((cfg.sound.volume - 0.5).abs() < f64::EPSILON);

        // Unspecified values fall back to defaults
        assert!(cfg.desktop.click_to_focus);
        assert!(cfg.sound.enabled);
        assert!(cfg.terminal_bell.enabled);
        assert!(!cfg.webhook.enabled);
    }

    #[test]
    fn merge_configs() {
        let base = Config::default();

        let mut overlay = Config::default();
        overlay.desktop.enabled = false;
        overlay.debug.enabled = true;
        overlay.debug.log_file = "/tmp/test.log".to_string();

        let merged = base.merge(overlay).expect("merge should succeed");

        assert!(!merged.desktop.enabled);
        assert!(merged.debug.enabled);
        assert_eq!(merged.debug.log_file, "/tmp/test.log");

        // Fields not changed by overlay retain base values
        assert!(merged.sound.enabled);
        assert_eq!(merged.suppression.cooldown_seconds, 7);
    }

    #[test]
    fn priority_override() {
        let yaml = r#"
priority_overrides:
  task_complete: urgent
  question: low
"#;
        let cfg: Config = serde_yaml::from_str(yaml).expect("should parse");
        assert_eq!(cfg.priority_overrides.get("task_complete"), Some(&Priority::Urgent));
        assert_eq!(cfg.priority_overrides.get("question"), Some(&Priority::Low));
    }

    #[test]
    fn status_override() {
        let yaml = r#"
status_overrides:
  api_error:
    enabled: true
    title: "API problem!"
  task_complete:
    sound: "ding.wav"
"#;
        let cfg: Config = serde_yaml::from_str(yaml).expect("should parse");

        let api_err = cfg.status_overrides.get("api_error").expect("should exist");
        assert_eq!(api_err.enabled, Some(true));
        assert_eq!(api_err.title.as_deref(), Some("API problem!"));
        assert!(api_err.sound.is_none());

        let task = cfg.status_overrides.get("task_complete").expect("should exist");
        assert_eq!(task.sound.as_deref(), Some("ding.wav"));
        assert!(task.enabled.is_none());
        assert!(task.title.is_none());
    }

    #[test]
    fn deep_merge_yaml_scalar_override() {
        let base: Value = serde_yaml::from_str("x: 1\ny: 2").unwrap();
        let overlay: Value = serde_yaml::from_str("x: 99").unwrap();
        let merged = deep_merge_yaml(base, overlay);
        assert_eq!(merged["x"], Value::Number(99.into()));
        assert_eq!(merged["y"], Value::Number(2.into()));
    }

    #[test]
    fn deep_merge_yaml_null_overlay_keeps_base() {
        let base: Value = serde_yaml::from_str("x: 42").unwrap();
        let merged = deep_merge_yaml(base.clone(), Value::Null);
        assert_eq!(merged["x"], Value::Number(42.into()));
    }

    #[test]
    fn load_from_file_returns_default_when_missing() {
        let cfg = Config::load_from_file("/nonexistent/path/config.yaml")
            .expect("should not error for missing file");
        assert_eq!(cfg, Config::default());
    }
}
