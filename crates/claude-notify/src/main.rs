// claude-notify: CLI binary entry point

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::warn;

use claude_notify_core::{
    analyzer,
    config::Config,
    decision::{DecisionEngine, UserActivity},
    dedup,
    hook::HookInput,
    priority::PriorityEngine,
    state::SessionState,
    summary,
    types::{Channel, Decision},
};
use claude_notify_dispatch::{
    desktop::DesktopDispatcher,
    sound::SoundDispatcher,
    terminal_bell::TerminalBellDispatcher,
    traits::Dispatcher,
    webhook::{WebhookDispatcher, WebhookPreset},
    NotifyRouter,
};

// ─── CLI structure ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "claude-notify",
    version = env!("CARGO_PKG_VERSION"),
    about = "Claude Code notification plugin"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Handle a Claude hook event (reads JSON from stdin)
    HandleHook {
        /// Hook type (e.g. Stop, PostToolUse)
        hook_type: String,
    },
    /// Send a test notification
    Test,
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Sound management
    Sounds {
        #[command(subcommand)]
        action: SoundsAction,
    },
    /// Print version
    Version,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current config as YAML
    Show,
    /// Validate current configuration
    Validate,
    /// Delete user config file (reset to defaults)
    Reset,
}

#[derive(Subcommand)]
enum SoundsAction {
    /// List available sound files
    List,
}

// ─── ActivityAdapter ─────────────────────────────────────────────────────────

/// Adapts the platform's `UserActivityDetector` to the core `UserActivity` trait.
struct ActivityAdapter(Box<dyn claude_notify_platform::UserActivityDetector>);

impl UserActivity for ActivityAdapter {
    fn idle_seconds(&self) -> u64 {
        self.0.idle_seconds()
    }

    fn is_terminal_focused(&self) -> bool {
        self.0.is_terminal_focused()
    }
}

// ─── main / run ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        warn!("claude-notify error: {:#}", e);
        // Always exit 0 so we never block Claude Code
    }
}

fn run(cli: Cli) -> Result<()> {
    // Resolve plugin root
    let plugin_root = resolve_plugin_root();

    // Load layered config
    let config = Config::load_layered(&plugin_root, "").context("failed to load config")?;

    // Optional debug logging
    if config.debug.enabled {
        setup_debug_logging(&config.debug.log_file);
    }

    match cli.command {
        Commands::HandleHook { hook_type } => {
            handle_hook(&config, &plugin_root, &hook_type)?;
        }
        Commands::Test => {
            handle_test(&config, &plugin_root)?;
        }
        Commands::Config { action } => {
            handle_config(&config, action)?;
        }
        Commands::Sounds { action } => {
            handle_sounds(&plugin_root, action)?;
        }
        Commands::Version => {
            println!("claude-notify {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}

// ─── Plugin root resolution ───────────────────────────────────────────────────

fn resolve_plugin_root() -> PathBuf {
    if let Ok(root) = std::env::var("CLAUDE_PLUGIN_ROOT") {
        return PathBuf::from(root);
    }
    // Fallback: parent directory of the binary
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ─── Debug logging setup ──────────────────────────────────────────────────────

fn setup_debug_logging(log_file: &str) {
    use tracing_subscriber::{fmt, EnvFilter};

    let log_path = if log_file.is_empty() {
        std::env::var("HOME")
            .map(|h| format!("{h}/.claude/claude-notification/debug.log"))
            .unwrap_or_else(|_| "/tmp/claude-notify-debug.log".to_string())
    } else {
        log_file.to_string()
    };

    // Best-effort: create parent dirs, then open the log file
    if let Some(parent) = Path::new(&log_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter(EnvFilter::new("debug"))
            .with_writer(move || file.try_clone().expect("clone log file"))
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}

// ─── handle_hook ─────────────────────────────────────────────────────────────

fn handle_hook(config: &Config, plugin_root: &Path, _hook_type: &str) -> Result<()> {
    // 1. Parse HookInput from stdin
    let hook_input = HookInput::from_stdin().context("failed to parse hook input from stdin")?;

    // 2. DedupLock check (2s TTL)
    let lock_path = dedup::dedup_lock_path(&hook_input.session_id);
    let _lock = match dedup::DedupLock::try_acquire(&lock_path, 2).context("dedup lock error")? {
        Some(lock) => lock,
        None => {
            tracing::debug!(
                "duplicate hook event suppressed for session {}",
                hook_input.session_id
            );
            return Ok(());
        }
    };

    // 3. Parse transcript
    let transcript_path = Path::new(&hook_input.transcript_path);
    let messages = if transcript_path.exists() {
        analyzer::parse_transcript(transcript_path).unwrap_or_default()
    } else {
        Vec::new()
    };

    // 4. detect_status
    let status = analyzer::detect_status(&messages, &hook_input);

    // 5. extract_summary
    let summary = summary::extract_summary(&status, &messages);

    // 6. Load SessionState
    let state_path = SessionState::state_path(&hook_input.session_id);
    let mut state = SessionState::load(&state_path).unwrap_or_default();

    // 7. Build PriorityEngine from config
    let priority_engine = PriorityEngine::new(
        config.priority_overrides.clone(),
        config.priority_channels.clone(),
    );

    // 8. Get platform activity detector
    let detector = claude_notify_platform::create_activity_detector();

    // 9. Adapter pattern
    let activity = ActivityAdapter(detector);

    // 10. DecisionEngine.decide()
    let engine = DecisionEngine::new(config, &priority_engine);
    let decision = engine.decide(status, &summary, &activity, &state);

    // 11. Match decision
    match decision {
        Decision::Notify {
            channels,
            notification,
            ..
        } => {
            tracing::debug!("Decision: Notify via {:?}", channels);
            let dispatchers = build_dispatchers(config, plugin_root, &channels);
            let refs: Vec<&dyn Dispatcher> = dispatchers.iter().map(|d| d.as_ref()).collect();
            let router = NotifyRouter::new();
            let report = router.dispatch_to(&refs, &notification.title, &notification.body);
            if report.failures > 0 {
                warn!(
                    "dispatch had {} failures: {:?}",
                    report.failures, report.errors
                );
            }
            state.update_after_notification(status.as_str(), &summary);
            let _ = state.save(&state_path);
        }
        Decision::Downgrade {
            channels,
            notification,
            from,
            to,
            reason,
        } => {
            tracing::debug!("Decision: Downgrade {:?} → {:?} ({})", from, to, reason);
            let dispatchers = build_dispatchers(config, plugin_root, &channels);
            let refs: Vec<&dyn Dispatcher> = dispatchers.iter().map(|d| d.as_ref()).collect();
            let router = NotifyRouter::new();
            let report = router.dispatch_to(&refs, &notification.title, &notification.body);
            if report.failures > 0 {
                warn!(
                    "dispatch had {} failures: {:?}",
                    report.failures, report.errors
                );
            }
            state.update_after_notification(status.as_str(), &summary);
            let _ = state.save(&state_path);
        }
        Decision::Suppress { reason } => {
            tracing::debug!("Decision: Suppress ({})", reason);
        }
    }

    Ok(())
}

// ─── build_dispatchers ────────────────────────────────────────────────────────

fn build_dispatchers(
    config: &Config,
    plugin_root: &Path,
    channels: &[Channel],
) -> Vec<Box<dyn Dispatcher>> {
    let mut dispatchers: Vec<Box<dyn Dispatcher>> = Vec::new();

    for channel in channels {
        match channel {
            Channel::Desktop if config.desktop.enabled => {
                let notifier = claude_notify_platform::create_desktop_notifier();
                let icon = if config.desktop.app_icon.is_empty() {
                    None
                } else {
                    Some(config.desktop.app_icon.clone())
                };
                let timeout_ms = if config.desktop.timeout > 0 {
                    Some((config.desktop.timeout * 1000) as u32)
                } else {
                    None
                };
                dispatchers.push(Box::new(DesktopDispatcher::new(notifier, icon, timeout_ms)));
            }
            Channel::Sound if config.sound.enabled => {
                let sounds_dir = plugin_root.join("sounds");
                let volume = config.sound.volume as f32;
                dispatchers.push(Box::new(SoundDispatcher::new(volume, sounds_dir)));
            }
            Channel::TerminalBell if config.terminal_bell.enabled => {
                dispatchers.push(Box::new(TerminalBellDispatcher::new()));
            }
            Channel::Webhook if config.webhook.enabled && !config.webhook.url.is_empty() => {
                let preset = WebhookPreset::parse(&config.webhook.preset);
                let mut dispatcher = WebhookDispatcher::new(config.webhook.url.clone())
                    .with_preset(preset)
                    .with_retry_max(config.webhook.retry_max)
                    .with_timeout_seconds(config.webhook.timeout_seconds);
                if !config.webhook.chat_id.is_empty() {
                    dispatcher = dispatcher.with_chat_id(config.webhook.chat_id.clone());
                }
                for (k, v) in &config.webhook.headers {
                    dispatcher = dispatcher.with_header(k.clone(), v.clone());
                }
                dispatchers.push(Box::new(dispatcher));
            }
            _ => {
                // Channel disabled in config — skip
            }
        }
    }

    dispatchers
}

// ─── handle_test ─────────────────────────────────────────────────────────────

fn handle_test(config: &Config, plugin_root: &Path) -> Result<()> {
    println!("Sending test notification...");

    let test_channels = vec![Channel::Desktop, Channel::Sound, Channel::TerminalBell];
    let dispatchers = build_dispatchers(config, plugin_root, &test_channels);

    if dispatchers.is_empty() {
        println!("No dispatchers enabled. Check your config.");
        return Ok(());
    }

    let refs: Vec<&dyn Dispatcher> = dispatchers.iter().map(|d| d.as_ref()).collect();
    let router = NotifyRouter::new();
    let report = router.dispatch_to(
        &refs,
        "✅ claude-notify test",
        "If you see this, notifications are working!",
    );

    println!(
        "Test complete: {} succeeded, {} failed",
        report.successes, report.failures
    );
    for err in &report.errors {
        eprintln!("  Error: {err}");
    }

    Ok(())
}

// ─── handle_config ────────────────────────────────────────────────────────────

fn handle_config(config: &Config, action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let yaml =
                serde_yaml::to_string(config).context("failed to serialize config to YAML")?;
            print!("{yaml}");
        }
        ConfigAction::Validate => {
            println!("Configuration is valid");
        }
        ConfigAction::Reset => {
            let config_path = std::env::var("HOME")
                .map(|h| {
                    PathBuf::from(h)
                        .join(".claude")
                        .join("claude-notification")
                        .join("config.yaml")
                })
                .context("HOME env var not set")?;

            if config_path.exists() {
                std::fs::remove_file(&config_path)
                    .with_context(|| format!("failed to delete {:?}", config_path))?;
                println!("Deleted user config: {}", config_path.display());
            } else {
                println!("No user config file found at {}", config_path.display());
            }
        }
    }
    Ok(())
}

// ─── handle_sounds ────────────────────────────────────────────────────────────

fn handle_sounds(plugin_root: &Path, action: SoundsAction) -> Result<()> {
    match action {
        SoundsAction::List => {
            let sounds_dir = plugin_root.join("sounds");
            println!("Plugin sounds ({}/):", sounds_dir.display());

            if sounds_dir.exists() {
                let mut entries: Vec<_> = std::fs::read_dir(&sounds_dir)
                    .with_context(|| format!("failed to read sounds dir {:?}", sounds_dir))?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|x| x.to_str())
                            .map(|x| matches!(x, "mp3" | "wav" | "ogg" | "aiff" | "aac"))
                            .unwrap_or(false)
                    })
                    .collect();
                entries.sort_by_key(|e| e.file_name());
                if entries.is_empty() {
                    println!("  (no sound files found)");
                } else {
                    for entry in entries {
                        println!("  {}", entry.file_name().to_string_lossy());
                    }
                }
            } else {
                println!("  (sounds directory does not exist)");
            }

            // macOS system sounds
            #[cfg(target_os = "macos")]
            {
                let system_sounds = Path::new("/System/Library/Sounds");
                if system_sounds.exists() {
                    println!("\nmacOS system sounds ({}/): ", system_sounds.display());
                    if let Ok(entries) = std::fs::read_dir(system_sounds) {
                        let mut names: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path()
                                    .extension()
                                    .and_then(|x| x.to_str())
                                    .map(|x| x == "aiff")
                                    .unwrap_or(false)
                            })
                            .map(|e| e.file_name().to_string_lossy().into_owned())
                            .collect();
                        names.sort();
                        for name in names {
                            println!("  {name}");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
