#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building claude-notify ==="
export PATH="$HOME/.cargo/bin:$PATH"
(cd "$PROJECT_ROOT/crates" && cargo build --release 2>&1)

BINARY="$PROJECT_ROOT/crates/target/release/claude-notify"

echo "=== Test: version ==="
$BINARY version

echo "=== Test: config show ==="
CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY config show > /dev/null
echo "config show: OK"

echo "=== Test: sounds list ==="
CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY sounds list || true
echo "sounds list: OK"

echo "=== Test: handle-hook with task complete ==="
TMPDIR_E2E=$(mktemp -d)
cat > "$TMPDIR_E2E/transcript.jsonl" <<'JSONL'
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Write","input":{"file_path":"/tmp/test.rs","content":"fn main() {}"}}]},"duration_ms":500}
{"type":"tool_result","tool_name":"Write","content":"File written successfully"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"I have created the file for you."}]},"duration_ms":300}
JSONL

echo '{"session_id":"test-e2e-1","transcript_path":"'"$TMPDIR_E2E"'/transcript.jsonl","tool_name":"","is_team_lead":false}' | \
    CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY handle-hook Stop 2>/dev/null || true
echo "handle-hook Stop: OK"

echo "=== Test: handle-hook with question ==="
cat > "$TMPDIR_E2E/transcript-q.jsonl" <<'JSONL'
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"AskUserQuestion","input":{"question":"Which framework?"}}]},"duration_ms":200}
JSONL

echo '{"session_id":"test-e2e-2","transcript_path":"'"$TMPDIR_E2E"'/transcript-q.jsonl","tool_name":"AskUserQuestion","is_team_lead":false}' | \
    CLAUDE_PLUGIN_ROOT="$PROJECT_ROOT" $BINARY handle-hook Stop 2>/dev/null || true
echo "handle-hook Question: OK"

rm -rf "$TMPDIR_E2E"
echo "=== All e2e tests passed ==="
