# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

All Rust code lives under `crates/` (Cargo workspace). Run commands from there:

```bash
export PATH="$HOME/.cargo/bin:$PATH"  # cargo may not be in PATH
cd crates
cargo build                    # Dev build
cargo build --release          # Release build (binary: target/release/claude-notify)
cargo test --workspace         # All 68+ unit tests
cargo test -p claude-notify-core -- decision  # Tests for a specific module
cargo clippy --workspace       # Lint
cargo fmt --all -- --check     # Format check
```

E2E test (builds release, exercises CLI commands with mock transcripts):
```bash
bash tests/e2e_test.sh
```

CLI usage (after build):
```bash
claude-notify handle-hook <type>  # Hook entry, reads JSON from stdin
claude-notify test                # Send test notification
claude-notify config show         # Dump current config as YAML
claude-notify sounds list         # List available sound files
```

## Architecture

Four crates, one binary:

- **claude-notify-core** — All business logic. Types, config, transcript analysis, priority/suppression/decision engines, session state. No platform dependencies. 64+ tests.
- **claude-notify-platform** — Trait abstractions (`DesktopNotifier`, `UserActivityDetector`) with per-OS implementations. macOS uses Swift app bundle (`swift-notifier/ClaudeNotifier.app`) with osascript fallback, Linux uses `notify-rust`, Windows uses `winrt-notification`.
- **claude-notify-dispatch** — `Dispatcher` trait and implementations (desktop, sound via `rodio`, terminal bell, webhook). `NotifyRouter` fans out to all channels, failures don't block others.
- **claude-notify** — Thin CLI binary (`clap` derive). Wires the other three crates together. Contains `ActivityAdapter` to bridge platform and core traits.

## Core Data Flow

```
Hook trigger (stdin JSON)
  → DedupLock (file lock, 2s TTL)
  → Analyzer: parse transcript JSONL → detect Status (7 types, priority-ordered)
  → Summary: extract meaningful text per status type
  → DecisionEngine:
      → PriorityEngine: Status → Priority (Urgent/Normal/Low), with user overrides
      → SuppressionEngine: cooldown, cascade, content dedup, filters
      → Activity check: if terminal focused + user active → downgrade to bell only
  → Decision: Notify / Suppress / Downgrade
  → NotifyRouter → Dispatchers (Desktop / Sound / TerminalBell / Webhook)
  → SessionState: persist to temp file
```

## Config Layering

Loaded in order (later overrides earlier), deep-merged via `serde_yaml::Value`:

1. Rust defaults → 2. `config/default-config.yaml` → 3. `~/.claude/claude-notification/config.yaml` → 4. `.claude-notification.yaml` (project-level)

## Plugin Structure

- `.claude-plugin/plugin.json` — Plugin manifest
- `.claude-plugin/marketplace.json` — Marketplace distribution
- `hooks/hooks.json` — Registers 5 hook events (PreToolUse, Notification, Stop, SubagentStop, TeammateIdle)
- `hooks/hook-wrapper.sh` — Binary launcher: finds/builds/downloads binary, then `exec` forwards args+stdin
- `sounds/*.wav` — Notification audio files (WAV format, played by `rodio`)

## Key Conventions

- Errors in notification pipeline never block Claude Code — `main()` catches all errors and exits 0.
- macOS notifications use native Swift .app bundle (`swift-notifier/`) for custom icon and click-to-focus. Falls back to osascript if Swift app unavailable. osascript shows Script Editor icon — avoid using it as primary.
- `suppress_when_focused` defaults to false — terminal focus detection is unreliable from CLI subprocesses (terminal is always "frontmost" when running hook commands).
- `UserActivity` trait is defined in core's `decision.rs`; `UserActivityDetector` is in platform's `lib.rs`. The binary bridges them via `ActivityAdapter`.
- Status detection is priority-ordered: SessionLimit > ApiError > tool-based > fallback to TaskComplete.
- Urgent priority bypasses both cooldown and idle check.

## Development Gotchas

- Plugin cache (`~/.claude/plugins/cache/`) is a snapshot from install time. After local changes: rebuild binary, copy to cache, copy Swift app to cache. Changes to hooks.json/config also need manual sync.
- `hooks.json` must use nested object format: `{hooks: {EventName: [{matcher, hooks: [...]}]}}`. Flat array format silently fails to load.
- CI on Linux needs `libasound2-dev` for rodio/alsa-sys.
- Swift notifier requires ad-hoc code signing (`codesign --force --deep --sign -`) and lsregister to show custom icon. First launch needs notification permission grant in System Settings → Notifications.
- `winrt-notification::Error` doesn't impl `Display` — use `format!("{e:?}")` on Windows.
