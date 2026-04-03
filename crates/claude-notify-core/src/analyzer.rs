use std::path::Path;

use serde_json::Value;

use crate::error::{NotifyError, Result};
use crate::hook::HookInput;
use crate::types::Status;

// ─── Message ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Message {
    pub msg_type: String,
    pub text_content: String,
    pub tool_name: Option<String>,
    pub tool_input: Value,
    pub is_api_error: bool,
    pub error_status: Option<u16>,
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

/// Read a JSONL file from disk and parse each line into a `Message`.
pub fn parse_transcript(path: &Path) -> Result<Vec<Message>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| NotifyError::TranscriptParse(format!("cannot read {:?}: {}", path, e)))?;
    Ok(parse_transcript_str(&content))
}

/// Parse a JSONL string (one JSON object per line) into a list of `Message`s.
pub fn parse_transcript_str(content: &str) -> Vec<Message> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(parse_message_line)
        .collect()
}

/// Parse a single JSONL line into a `Message`, returning `None` on failure.
pub fn parse_message_line(line: &str) -> Option<Message> {
    let v: Value = serde_json::from_str(line.trim()).ok()?;

    let msg_type = v
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    // Extract text content —————————————————————————————————————————————————
    let text_content = extract_text_content(&v);

    // Extract tool info ———————————————————————————————————————————————————
    let (tool_name, tool_input) = extract_tool_info(&v);

    // API error fields ————————————————————————————————————————————————————
    let is_api_error = v
        .get("isApiErrorMessage")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    let error_status = v
        .get("error")
        .and_then(|e| e.get("status"))
        .and_then(|s| s.as_u64())
        .map(|s| s as u16);

    Some(Message {
        msg_type,
        text_content,
        tool_name,
        tool_input,
        is_api_error,
        error_status,
    })
}

/// Extract concatenated text from message.content array (type="text" items)
/// or fall back to top-level "content" string.
fn extract_text_content(v: &Value) -> String {
    // Try message.content array first
    if let Some(content_arr) = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        let texts: Vec<&str> = content_arr
            .iter()
            .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect();
        if !texts.is_empty() {
            return texts.join(" ");
        }
    }

    // Try top-level "content" as string
    if let Some(s) = v.get("content").and_then(|c| c.as_str()) {
        return s.to_string();
    }

    // Try top-level "content" as array of objects with text
    if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
        let texts: Vec<&str> = arr
            .iter()
            .filter(|item| item.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
            .collect();
        if !texts.is_empty() {
            return texts.join(" ");
        }
    }

    String::new()
}

/// Extract tool_name and tool_input from top-level fields or message.content array.
fn extract_tool_info(v: &Value) -> (Option<String>, Value) {
    // Check message.content array for tool_use items
    if let Some(content_arr) = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        for item in content_arr {
            if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                let name = item
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
                let input = item.get("input").cloned().unwrap_or(Value::Null);
                if name.is_some() {
                    return (name, input);
                }
            }
        }
    }

    // Fall back to top-level tool_name / tool_input
    let name = v
        .get("tool_name")
        .and_then(|n| n.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let input = v.get("tool_input").cloned().unwrap_or(Value::Null);

    (name, input)
}

// ─── Detection ───────────────────────────────────────────────────────────────

/// Return the last N messages (up to 15).
pub fn recent_messages(messages: &[Message]) -> &[Message] {
    let n = messages.len();
    let start = n.saturating_sub(15);
    &messages[start..]
}

/// Detect the `Status` from the transcript messages + hook context.
/// Priority order:
///   1. SessionLimit
///   2. ApiError / ApiOverloaded
///   3. Question
///   4. PlanReady
///   5. TaskComplete (recent write/edit/bash tools)
///   6. ReviewComplete (only read tools + long text)
///   7. Fallback: TaskComplete
pub fn detect_status(messages: &[Message], hook_context: &HookInput) -> Status {
    let recent = recent_messages(messages);

    // 1. Session limit —————————————————————————————————————————————————————
    let last3_start = if recent.len() > 3 {
        recent.len() - 3
    } else {
        0
    };
    let last3 = &recent[last3_start..];
    if last3.iter().any(|m| {
        m.text_content
            .to_lowercase()
            .contains("session limit reached")
    }) {
        return Status::SessionLimit;
    }

    // 2. API errors ————————————————————————————————————————————————————————
    if hook_context.is_api_error_message {
        return Status::ApiError;
    }
    for msg in recent.iter() {
        if msg.is_api_error {
            let code = msg.error_status.unwrap_or(0);
            return if code == 429 || code == 529 {
                Status::ApiOverloaded
            } else {
                Status::ApiError
            };
        }
    }

    // 3. Question ——————————————————————————————————————————————————————————
    if hook_context.tool_name == "AskUserQuestion" {
        return Status::Question;
    }

    // 4. Plan ready ————————————————————————————————————————————————————————
    if hook_context.tool_name == "ExitPlanMode" {
        return Status::PlanReady;
    }

    // 5. Task complete (write/edit/bash tools in recent messages) —————————
    const WRITE_TOOLS: &[&str] = &["Write", "Edit", "Bash", "NotebookEdit"];
    let has_write_tool = recent.iter().any(|m| {
        m.tool_name
            .as_deref()
            .map(|n| WRITE_TOOLS.contains(&n))
            .unwrap_or(false)
    });
    if has_write_tool {
        return Status::TaskComplete;
    }

    // 6. Review complete (only read tools + long text) ————————————————————
    const READ_TOOLS: &[&str] = &["Read", "Grep", "Glob"];
    let has_any_tool = recent.iter().any(|m| m.tool_name.is_some());
    let only_read_tools = has_any_tool
        && recent.iter().all(|m| match &m.tool_name {
            None => true,
            Some(n) => READ_TOOLS.contains(&n.as_str()),
        });
    let long_text = recent.iter().any(|m| m.text_content.len() > 200);
    if only_read_tools && long_text {
        return Status::ReviewComplete;
    }

    // 7. Fallback ——————————————————————————————————————————————————————————
    Status::TaskComplete
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hook_input(tool_name: &str) -> HookInput {
        HookInput {
            session_id: "test-session".to_string(),
            transcript_path: "/tmp/transcript.jsonl".to_string(),
            tool_name: tool_name.to_string(),
            tool_input: Value::Null,
            tool_result: Value::Null,
            is_team_lead: false,
            team_name: String::new(),
            is_api_error_message: false,
        }
    }

    fn make_message(
        msg_type: &str,
        text_content: &str,
        tool_name: Option<&str>,
        is_api_error: bool,
        error_status: Option<u16>,
    ) -> Message {
        Message {
            msg_type: msg_type.to_string(),
            text_content: text_content.to_string(),
            tool_name: tool_name.map(|s| s.to_string()),
            tool_input: Value::Null,
            is_api_error,
            error_status,
        }
    }

    #[test]
    fn detect_session_limit() {
        let messages = vec![make_message(
            "assistant",
            "Session limit reached, please start a new session.",
            None,
            false,
            None,
        )];
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::SessionLimit);
    }

    #[test]
    fn detect_api_error() {
        let messages = vec![make_message(
            "assistant",
            "API call failed.",
            None,
            true,
            Some(500),
        )];
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::ApiError);
    }

    #[test]
    fn detect_api_overloaded() {
        let messages = vec![make_message(
            "assistant",
            "Too many requests.",
            None,
            true,
            Some(429),
        )];
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::ApiOverloaded);
    }

    #[test]
    fn detect_question() {
        let messages = vec![make_message(
            "assistant",
            "I need more info.",
            Some("AskUserQuestion"),
            false,
            None,
        )];
        let hook = make_hook_input("AskUserQuestion");
        assert_eq!(detect_status(&messages, &hook), Status::Question);
    }

    #[test]
    fn detect_plan_ready() {
        let messages = vec![make_message(
            "assistant",
            "Here is my plan.",
            None,
            false,
            None,
        )];
        let hook = make_hook_input("ExitPlanMode");
        assert_eq!(detect_status(&messages, &hook), Status::PlanReady);
    }

    #[test]
    fn detect_task_complete_with_write_tool() {
        let messages = vec![make_message(
            "assistant",
            "Writing the file.",
            Some("Write"),
            false,
            None,
        )];
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::TaskComplete);
    }

    #[test]
    fn detect_review_complete_read_only() {
        let long_text = "a".repeat(201);
        let messages = vec![
            make_message("assistant", &long_text, Some("Read"), false, None),
            make_message("assistant", "Also grepped.", Some("Grep"), false, None),
        ];
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::ReviewComplete);
    }

    #[test]
    fn parse_jsonl_transcript() {
        let jsonl = r#"
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello there."}]},"session_id":"s1"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Write","input":{"file_path":"/tmp/a.txt","content":"hi"}}]},"session_id":"s1"}
{"type":"tool_result","tool_use_id":"t1","content":[{"type":"text","text":"Written."}],"session_id":"s1"}
"#;
        let messages = parse_transcript_str(jsonl);
        assert_eq!(messages.len(), 3);

        // First message: assistant text
        assert_eq!(messages[0].msg_type, "assistant");
        assert_eq!(messages[0].text_content, "Hello there.");
        assert!(messages[0].tool_name.is_none());

        // Second message: tool_use Write
        assert_eq!(messages[1].msg_type, "assistant");
        assert_eq!(messages[1].tool_name.as_deref(), Some("Write"));

        // Third message: tool_result
        assert_eq!(messages[2].msg_type, "tool_result");
    }

    #[test]
    fn parse_api_error_line() {
        let line = r#"{"type":"assistant","isApiErrorMessage":true,"error":{"status":529},"message":{"role":"assistant","content":[{"type":"text","text":"Overloaded"}]}}"#;
        let msg = parse_message_line(line).expect("should parse");
        assert!(msg.is_api_error);
        assert_eq!(msg.error_status, Some(529));
        assert_eq!(msg.text_content, "Overloaded");
    }

    #[test]
    fn parse_session_limit_fixture() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/session_limit.jsonl"
        ));
        let messages = parse_transcript(path).expect("should parse fixture");
        assert!(!messages.is_empty());
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::SessionLimit);
    }

    #[test]
    fn parse_task_complete_fixture() {
        let path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/task_complete.jsonl"
        ));
        let messages = parse_transcript(path).expect("should parse fixture");
        assert!(!messages.is_empty());
        let hook = make_hook_input("");
        assert_eq!(detect_status(&messages, &hook), Status::TaskComplete);
    }
}
