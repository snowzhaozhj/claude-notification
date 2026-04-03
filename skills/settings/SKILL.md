---
name: notification-settings
description: Use when user wants to configure notification settings, adjust notification behavior, or set up webhooks for the claude-notification plugin.
---

# Notification Settings

Help the user configure the claude-notification plugin interactively.

## Steps
1. Read current config from ~/.claude/claude-notification/config.yaml
2. Show current settings summary
3. Ask what to configure (desktop, sound, webhook, priority, suppression, team, test, reset)
4. Guide through chosen configuration
5. Write updated config
6. Offer test notification

## Test Command
After configuration, run: `${CLAUDE_PLUGIN_ROOT}/hooks/hook-wrapper.sh test`
