# Claude Notify

Smart notification plugin for [Claude Code](https://claude.ai/code). Get desktop notifications when Claude needs your attention — task complete, questions, errors, and more.

## Features

- **Intelligent timing** — priority-based decisions (urgent/normal/low), cooldown dedup, cascade suppression
- **Clear content** — extracts meaningful summaries from transcripts, not raw tool counts
- **Cross-platform** — macOS (native Swift notifications), Linux (D-Bus), Windows (Toast API)
- **Multiple channels** — desktop notification, sound, terminal bell, webhook (Slack/Discord/Telegram/Lark)
- **Highly configurable** — YAML config with per-status overrides, project-level config, interactive setup

## Install

```bash
# Add marketplace
claude plugin marketplace add snowzhaozhj/claude-notification

# Install
claude plugin install claude-notification@claude-notification
```

Restart Claude Code (or run `/reload-plugins`). The first hook trigger will auto-compile the binary (requires Rust toolchain) or download it from GitHub Releases.

### macOS Note

On first notification, macOS will ask for notification permission. Go to **System Settings → Notifications → Claude Notify** and enable it.

## Usage

Once installed, notifications fire automatically on:

| Event | Priority | Trigger |
|-------|----------|---------|
| Task Complete | Normal | Claude finishes writing/editing code |
| Review Complete | Low | Claude finishes reading/analyzing code |
| Question | Urgent | Claude asks you a question |
| Plan Ready | Normal | Claude exits plan mode |
| API Error | Urgent | API returns an error |
| Session Limit | Urgent | Session token limit reached |

## Configuration

Interactive setup:
```
/claude-notification:notification-settings
```

Or edit `~/.claude/claude-notification/config.yaml` directly:

```yaml
# Disable sound
sound:
  enabled: false

# Webhook (e.g. Slack)
webhook:
  enabled: true
  preset: "slack"
  url: "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"

# Override priority for specific statuses
priority_overrides:
  review_complete: normal

# Suppress notifications on specific branches
suppression:
  filters:
    - status: task_complete
      git_branch: "wip"
```

Project-level overrides: add `.claude-notification.yaml` to your project root.

Full config reference: [`config/default-config.yaml`](config/default-config.yaml)

## Architecture

Rust workspace with 4 crates:

```
crates/
├── claude-notify/            # CLI binary (clap)
├── claude-notify-core/       # Business logic, decision engine
├── claude-notify-dispatch/   # Notification channels (desktop, sound, webhook)
└── claude-notify-platform/   # OS abstractions (macOS, Linux, Windows)
```

macOS uses a native Swift app bundle (`swift-notifier/ClaudeNotifier.app`) for custom icon and click-to-focus, with osascript fallback.

## Development

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd crates

cargo test --workspace        # Run tests
cargo build --release         # Build binary
cargo clippy --workspace      # Lint
cargo fmt --all -- --check    # Format check
```

Build Swift notifier (macOS):
```bash
bash swift-notifier/build.sh
```

## License

MIT
