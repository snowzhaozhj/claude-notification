# Claude Code Smart Notification Plugin - Design Spec

## Overview

A Rust-based Claude Code plugin that delivers intelligent, cross-platform notifications. It replaces the existing `claude-notifications-go` plugin with smarter notification timing, clearer content extraction, stronger configurability, and a modern Rust architecture following community best practices.

## Goals

- **Intelligent timing**: Decide whether/how to notify based on priority, user activity, and suppression rules
- **Clear content**: Extract meaningful summaries from transcripts instead of naive truncation
- **Highly configurable**: YAML config with per-status overrides, interactive setup command, project-level config
- **Cross-platform**: macOS, Linux, Windows — native notification APIs on each platform
- **Reliable**: Never interrupt Claude Code workflow; fail silently with logging

## Tech Stack

- **Language**: Rust
- **Distribution**: Pre-compiled binaries per platform, lazy-downloaded on first hook trigger
- **Architecture**: Single binary, Cargo workspace with 4 crates

---

## Architecture

### Plugin Directory Structure

```
claude-notification-plugin/
├── .claude-plugin/
│   └── plugin.json
├── hooks/
│   ├── hooks.json
│   ├── hook-wrapper.sh          # Unix launcher (lazy binary download)
│   └── hook-wrapper.cmd         # Windows launcher
├── skills/
│   └── settings/SKILL.md        # Interactive config skill
├── commands/
│   └── notification-settings.md # /notification-settings slash command
├── config/
│   └── default-config.yaml      # Default config template
├── sounds/
│   ├── task-complete.mp3
│   ├── review-complete.mp3
│   ├── question.mp3
│   ├── plan-ready.mp3
│   └── error.mp3
├── bin/                          # Pre-compiled binaries
└── crates/                       # Cargo workspace
    ├── Cargo.toml
    ├── claude-notify/            # Binary crate (thin entry point)
    ├── claude-notify-core/       # Core logic library
    ├── claude-notify-dispatch/   # Notification dispatch
    └── claude-notify-platform/   # Platform abstractions
```

### Cargo Workspace — 4 Crates

**`claude-notify`** (binary crate)
- Thin CLI entry point using `clap` derive
- Parses subcommands, calls into `claude-notify-core`
- Top-level error handling: catches all errors, logs them, exits 0

**`claude-notify-core`** (library crate)
- `lib.rs` — public API entry point
- `hook.rs` — Hook stdin JSON parsing + deduplication (file locks)
- `analyzer.rs` — Transcript JSONL parsing + status detection state machine
- `summary.rs` — Content summary extraction and cleanup
- `priority.rs` — Priority assessment (urgent/normal/low)
- `suppression.rs` — Cooldown rules, content dedup, cascade suppression, filters
- `decision.rs` — Intelligence engine combining analyzer + priority + suppression + activity
- `config.rs` — YAML config loading with layered overrides
- `state.rs` — Session state persistence (temp files)
- `error.rs` — `thiserror` error types
- `types.rs` — Shared types (Status, Priority, Event, etc.)

**`claude-notify-dispatch`** (library crate)
- `traits.rs` — `trait Dispatcher { fn dispatch(&self, event: &NotifyEvent) -> Result<()> }`
- `desktop.rs` — Desktop notifications (delegates to platform crate)
- `sound.rs` — Audio playback via `rodio`
- `terminal_bell.rs` — Terminal bell character
- `webhook.rs` — Webhook with formatters for Slack, Discord, Telegram, Lark, Custom

**`claude-notify-platform`** (library crate)
- `lib.rs` — `trait DesktopNotifier` + `trait UserActivityDetector`
- `macos.rs` — macOS implementation (UNUserNotificationCenter, CGEventSource idle detection)
- `linux.rs` — Linux implementation (D-Bus libnotify, XScreenSaver/ext-idle-notify)
- `windows.rs` — Windows implementation (Toast API, GetLastInputInfo)
- `activity.rs` — Cross-platform user idle detection

### Why 4 Crates?

- **Independent compilation**: Changing dispatch logic doesn't recompile analyzer
- **Dependency isolation**: Platform system deps don't pollute core
- **Testability**: Core unit tests need zero system APIs — mock Dispatcher and UserActivityDetector
- **Clear boundaries**: Each crate has a single responsibility

### Core Data Flow

```
Hook trigger (stdin JSON)
  → dedup check (file lock, 2s TTL)
  → analyzer: parse transcript → detect Status
  → intelligence engine:
      → priority: assess urgency (urgent/normal/low)
      → user_activity: detect idle time, terminal focus
      → suppression: apply cooldown, dedup, cascade, filters
  → Decision: Notify / Suppress / Downgrade
  → dispatcher: route to channels (desktop / sound / bell / webhook)
  → state: persist to session state file
```

---

## Intelligence Engine

### Notification Statuses (7 types)

| Status | Detection Method |
|--------|-----------------|
| **SessionLimit** | Text "Session limit reached" in recent messages (highest priority) |
| **ApiError** | `isApiErrorMessage=true` + error field in transcript |
| **ApiOverloaded** | Rate limit / server error variants (429, 500, 529) |
| **Question** | `AskUserQuestion` tool or Notification hook with permission dialog |
| **PlanReady** | `ExitPlanMode` tool fired |
| **TaskComplete** | Recent tools include Write/Edit/Bash |
| **ReviewComplete** | Only Read/Grep/Glob tools + long text response (>200 chars) |

Detection is priority-ordered: SessionLimit > ApiError > tool-based > fallback.

### Priority Assessment

Three levels with default mapping (user-overridable):

| Status | Default Priority |
|--------|-----------------|
| ApiError, SessionLimit, Question | **urgent** |
| TaskComplete, PlanReady, ApiOverloaded | **normal** |
| ReviewComplete | **low** |

Priority determines channel behavior:

| Behavior | urgent | normal | low |
|----------|--------|--------|-----|
| Desktop notification | yes | yes | yes |
| Sound | yes | yes | no |
| Terminal bell | yes | no | no |
| Webhook | yes | yes | no |
| Bypass idle check | yes | no | no |
| Bypass cooldown | yes | no | no |

### User Activity Detection

Queried on each hook invocation (no daemon needed):

| Platform | Idle Detection | Focus Detection |
|----------|---------------|-----------------|
| macOS | `CGEventSourceSecondsSinceLastEventType` | `NSWorkspace.frontmostApplication` |
| Linux (X11) | `XScreenSaverQueryInfo` | `_NET_ACTIVE_WINDOW` |
| Linux (Wayland) | `ext-idle-notify-v1` | compositor-specific |
| Windows | `GetLastInputInfo` | `GetForegroundWindow` |

Decision logic:
- User idle > threshold (default 30s) → normal notification (desktop + sound)
- Terminal focused AND is current session's terminal → downgrade (terminal bell only)
- Terminal not focused → normal notification

### Suppression Rules

```rust
pub enum SuppressionRule {
    Cooldown { status: Status, seconds: u64 },
    ContentDedup { window_seconds: u64 },
    Cascade { after: Status, suppress: Status, seconds: u64 },
    Filter { status: Option<Status>, git_branch: Option<String>, folder: Option<String> },
}
```

Defaults:
- After `task_complete`, suppress `question` for 12 seconds
- After any notification, suppress `question` for 7 seconds
- Same content dedup: 180 second window
- File lock: 2 second TTL to prevent concurrent duplicates

### Decision Output

```rust
pub enum Decision {
    Notify { channels: Vec<Channel>, priority: Priority, notification: Notification },
    Suppress { reason: String },
    Downgrade { from: Priority, to: Priority, reason: String, channels: Vec<Channel>, notification: Notification },
}
```

---

## Notification Dispatch

### Desktop Notifications

```rust
pub struct Notification {
    pub title: String,
    pub body: String,              // <=150 chars
    pub subtitle: Option<String>,  // project/branch name
    pub icon: Option<PathBuf>,
    pub priority: Priority,
    pub click_action: Option<ClickAction>,
    pub thread_id: Option<String>, // group by session
    pub timeout: Option<u64>,      // auto-dismiss seconds
}
```

| Platform | Library | Fallback |
|----------|---------|----------|
| macOS | `mac-notification-sys` | `notify-rust` |
| Linux | `notify-rust` (D-Bus) | — |
| Windows | `winrt-notification` | — |

Click-to-focus: auto-detect terminal from `TERM_PROGRAM` env var, support tmux/zellij/WezTerm/kitty pane switching.

### Webhook

Unified trait + per-platform formatters:

- **Slack**: Block Kit format with thread grouping
- **Discord**: Embed with color and icon
- **Telegram**: Markdown format with chat_id
- **Lark/Feishu**: Rich text card
- **Custom**: User-defined JSON template with `{{title}}` `{{body}}` `{{status}}` variables

Reliability: max 3 retries, exponential backoff (1s → 2s → 4s), 10s timeout per request. Failures are logged but never block desktop notifications.

### Sound

- `rodio` crate for cross-platform audio (pure Rust)
- Supports MP3, WAV, OGG
- 5 built-in sounds (one per main status)
- Custom sound file paths supported
- macOS system sounds (`/System/Library/Sounds/`) supported
- Configurable volume (0.0 - 1.0) and output device

### Terminal Bell

Write `\x07` to `/dev/tty` (Unix) or `conout$` (Windows).

---

## Content Summary Extraction

Per-status extraction strategy:

| Status | Strategy |
|--------|----------|
| Question | Extract from `AskUserQuestion` tool_input; fallback to last text with `?` |
| PlanReady | Extract key points from `ExitPlanMode` tool_input |
| ApiError/Overloaded | Extract error code and message from transcript error field |
| SessionLimit | Fixed string "Session limit reached" |
| TaskComplete/ReviewComplete | Action counts (N writes, N reads) + last meaningful text from last 5 messages |

Cleanup pipeline:
1. Strip Markdown formatting (`#`, `**`, `` ` ``, `[]()`, code blocks)
2. Collapse consecutive whitespace
3. Truncate to 150 chars at word boundary, append `...`

---

## Configuration

### Config File Location

```
Defaults (hardcoded)
  ← config/default-config.yaml (plugin built-in)
    ← ~/.claude/claude-notification/config.yaml (user global)
      ← .claude-notification.yaml (project-level, optional)
```

### Config Schema (YAML)

```yaml
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
  preset: "slack"            # slack / discord / telegram / lark / custom
  url: ""
  chat_id: ""
  headers: {}
  template: ""
  retry_max: 3
  timeout_seconds: 10

priority_overrides: {}       # status_name: priority_level
priority_channels: {}        # priority_level: { channel: bool }

status_overrides: {}
  # task_complete:
  #   enabled: true
  #   sound: "~/my-sounds/done.mp3"
  #   title: "Done"

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
  mode: "always"             # always / wait-all / never
  notify_on_subagent: false
  suppress_for_subagents: true

debug:
  enabled: false
  log_file: ""
```

### Interactive Setup

`/notification-settings` command triggers a skill-based dialog guiding users through configuration (desktop, sound, webhook, priority rules, team mode, test notification, reset).

### CLI Subcommands

```
claude-notify handle-hook <hook_type>   # Hook entry point (stdin JSON)
claude-notify test                       # Send test notification
claude-notify config show                # Show current config
claude-notify config validate            # Validate config file
claude-notify config reset               # Reset to defaults
claude-notify sounds list                # List available sounds
claude-notify version                    # Version info
```

---

## Hook Registration

```json
{
  "hooks": {
    "PreToolUse": [{ "matcher": "ExitPlanMode|AskUserQuestion", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook PreToolUse", "timeout": 30 }] }],
    "Notification": [{ "matcher": "*", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook Notification", "timeout": 30 }] }],
    "Stop": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook Stop", "timeout": 30 }] }],
    "SubagentStop": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook SubagentStop", "timeout": 30 }] }],
    "TeammateIdle": [{ "matcher": "", "hooks": [{ "type": "command", "command": "${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh handle-hook TeammateIdle", "timeout": 30 }] }]
  }
}
```

---

## Distribution

### Pre-compiled Binaries

| Target | OS | Arch | Artifact |
|--------|-----|------|----------|
| macOS Intel | darwin | x86_64 | claude-notify-darwin-x86_64 |
| macOS Apple Silicon | darwin | aarch64 | claude-notify-darwin-aarch64 |
| Linux x64 | linux | x86_64 | claude-notify-linux-x86_64 |
| Linux ARM64 | linux | aarch64 | claude-notify-linux-aarch64 |
| Windows x64 | windows | x86_64 | claude-notify-windows-x86_64.exe |

Built with `cross-rs/cross` in GitHub Actions. Tag push triggers build + GitHub Release.

### Installation Flow

1. User installs plugin in Claude Code
2. First hook trigger → `hook-wrapper.sh` detects missing binary
3. Downloads correct platform binary from GitHub Release
4. Subsequent triggers use cached binary
5. Version mismatch triggers re-download

---

## Testing Strategy

### Unit Tests (core crate)

| Module | Focus |
|--------|-------|
| `analyzer.rs` | Various transcript inputs → correct Status |
| `summary.rs` | Markdown cleanup, truncation, per-status extraction |
| `priority.rs` | Default priorities + user overrides |
| `suppression.rs` | Cooldown, dedup, cascade, filter combinations |
| `decision.rs` | End-to-end: transcript + config + activity → correct Decision |
| `config.rs` | YAML loading, layered overrides, missing field defaults, invalid value errors |
| `hook.rs` | stdin JSON parsing, malformed input handling |

Trait-based mocking for external dependencies (MockNotifier, MockActivityDetector).

### Integration Tests (dispatch + platform crate)

- Webhook: `mockito` crate to simulate HTTP servers, verify request format
- Desktop: Platform-specific CI only, `#[ignore]` for optional skip
- Sound: Verify file loading and format parsing, no actual playback

### End-to-End Tests

- Construct full hook input JSON → run `claude-notify handle-hook Stop` → verify output/side effects
- CI uses fixture files to simulate transcripts

---

## Error Handling

**Principle: Notification errors must NEVER disrupt the user's Claude Code workflow.**

```rust
#[derive(thiserror::Error, Debug)]
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
}
```

| Layer | Strategy |
|-------|----------|
| Hook input parse failure | Log, exit 0 |
| Transcript read failure | Log, exit 0 |
| Config file corrupted | Fall back to defaults, log warning |
| Desktop notification failure | Continue with other channels |
| Sound playback failure | Silently ignore |
| Webhook failure | Retry then log, never block |
| File lock acquisition failure | Skip dedup, allow possible duplicate (better than missing notification) |

### Logging

Uses `tracing` crate. Disabled by default. Enabled via `config.debug.enabled = true`. Output to `~/.claude/claude-notification/debug.log`.

---

## Key Dependencies

| Purpose | Crate | Reason |
|---------|-------|--------|
| CLI parsing | `clap` (derive) | Community standard |
| Serialization | `serde` + `serde_yaml` + `serde_json` | De facto standard |
| Error handling | `thiserror` (lib) + `anyhow` (bin) | Best practice combo |
| HTTP client | `ureq` (sync) | Lightweight, no async runtime needed |
| Audio | `rodio` | Pure Rust, cross-platform |
| macOS notifications | `mac-notification-sys` | Native Obj-C bridge |
| Linux notifications | `notify-rust` | D-Bus libnotify wrapper |
| Windows notifications | `winrt-notification` | Windows Toast API |
| File locking | `fd-lock` | Cross-platform file locks |
| Logging | `tracing` | Modern structured logging |
| Temp files | `tempfile` | Safe temp file management |
