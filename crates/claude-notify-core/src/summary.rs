use crate::analyzer::Message;
use crate::types::Status;

// ─── Public API ───────────────────────────────────────────────────────────────

/// Dispatch to the appropriate extractor based on status.
pub fn extract_summary(status: &Status, messages: &[Message]) -> String {
    match status {
        Status::Question => extract_question(messages),
        Status::PlanReady => extract_plan_summary(messages),
        Status::ApiError | Status::ApiOverloaded => extract_error_info(messages),
        Status::SessionLimit => "Session limit reached. Please start a new session.".to_string(),
        Status::TaskComplete | Status::ReviewComplete => extract_work_summary(messages),
    }
}

// ─── Per-status extractors ────────────────────────────────────────────────────

/// Extract the question from an AskUserQuestion tool_input, or fall back to
/// the last assistant text that contains a '?'.
pub fn extract_question(messages: &[Message]) -> String {
    // 1. Look for AskUserQuestion tool with a "question" field
    for msg in messages.iter().rev() {
        if msg.tool_name.as_deref() == Some("AskUserQuestion") {
            if let Some(q) = msg.tool_input.get("question").and_then(|v| v.as_str()) {
                if !q.trim().is_empty() {
                    return clean_and_truncate(q);
                }
            }
        }
    }

    // 2. Fall back to the last text message that contains a '?'
    for msg in messages.iter().rev() {
        if msg.text_content.contains('?') && !msg.text_content.is_empty() {
            return clean_and_truncate(&msg.text_content);
        }
    }

    String::new()
}

/// Extract the plan from an ExitPlanMode tool_input, or fall back to the last
/// assistant text.
pub fn extract_plan_summary(messages: &[Message]) -> String {
    // 1. Look for ExitPlanMode tool with a "plan" field
    for msg in messages.iter().rev() {
        if msg.tool_name.as_deref() == Some("ExitPlanMode") {
            if let Some(plan) = msg.tool_input.get("plan").and_then(|v| v.as_str()) {
                if !plan.trim().is_empty() {
                    return clean_and_truncate(plan);
                }
            }
        }
    }

    // 2. Fall back to the last non-empty assistant text
    for msg in messages.iter().rev() {
        if !msg.text_content.is_empty() {
            return clean_and_truncate(&msg.text_content);
        }
    }

    String::new()
}

/// Extract error information from API error messages.
pub fn extract_error_info(messages: &[Message]) -> String {
    for msg in messages.iter().rev() {
        if msg.is_api_error {
            let code = msg
                .error_status
                .map(|c| format!("[{}] ", c))
                .unwrap_or_default();
            let text = if msg.text_content.is_empty() {
                "API error occurred".to_string()
            } else {
                msg.text_content.clone()
            };
            return format!("{}{}", code, clean_and_truncate(&text));
        }
    }

    // Fall back to any message with error content
    for msg in messages.iter().rev() {
        let lower = msg.text_content.to_lowercase();
        if lower.contains("error") || lower.contains("failed") {
            return clean_and_truncate(&msg.text_content);
        }
    }

    "An error occurred".to_string()
}

/// Extract a work summary from the last assistant text.
/// Focuses on what Claude said, not tool call statistics.
pub fn extract_work_summary(messages: &[Message]) -> String {
    // Find the last meaningful assistant text
    messages
        .iter()
        .rev()
        .find(|m| !m.text_content.is_empty() && m.tool_name.is_none())
        .map(|m| clean_and_truncate(&m.text_content))
        .unwrap_or_default()
}

// ─── Text utilities ───────────────────────────────────────────────────────────

/// Clean markdown then truncate to 150 chars.
pub fn clean_and_truncate(text: &str) -> String {
    let cleaned = clean_markdown(text);
    truncate(&cleaned, 150)
}

/// Remove common markdown formatting.
pub fn clean_markdown(text: &str) -> String {
    // 1. Remove fenced code blocks (``` ... ```) — including content inside
    let mut s = remove_code_blocks(text);

    // 2. Remove inline backticks
    s = remove_inline_backticks(&s);

    // 3. Remove markdown links [text](url) -> text
    s = remove_markdown_links(&s);

    // 4. Remove headers (## Header -> Header)
    s = remove_headers(&s);

    // 5. Remove bold/italic markers (**, __, *)
    s = remove_emphasis(&s);

    // 6. Remove bullet points at line start (- item, * item)
    s = remove_bullets(&s);

    // 7. Collapse whitespace (spaces, tabs, newlines)
    s = collapse_whitespace(&s);

    s.trim().to_string()
}

fn remove_code_blocks(text: &str) -> String {
    // Remove fenced code blocks delimited by ``` (with optional language tag)
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Check for ``` at current position
        if bytes.get(i) == Some(&b'`')
            && bytes.get(i + 1) == Some(&b'`')
            && bytes.get(i + 2) == Some(&b'`')
        {
            // Skip everything until the closing ```
            i += 3;
            // Skip optional language tag on the same line
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            // Skip the newline after the opening fence
            if i < bytes.len() && bytes[i] == b'\n' {
                i += 1;
            }
            // Advance until closing ```
            while i < bytes.len() {
                if bytes.get(i) == Some(&b'`')
                    && bytes.get(i + 1) == Some(&b'`')
                    && bytes.get(i + 2) == Some(&b'`')
                {
                    i += 3;
                    // Skip trailing newline after closing fence
                    if i < bytes.len() && bytes[i] == b'\n' {
                        i += 1;
                    }
                    break;
                }
                i += 1;
            }
            // Replace the whole block with a space separator
            result.push(' ');
        } else {
            // Safety: we're iterating byte-by-byte; need to handle multi-byte UTF-8
            let ch = text[i..].chars().next().unwrap();
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}

fn remove_inline_backticks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut inside = false;
    for ch in text.chars() {
        if ch == '`' {
            inside = !inside;
        } else if !inside {
            result.push(ch);
        }
    }
    result
}

fn remove_markdown_links(text: &str) -> String {
    // Replace [text](url) with just text
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if chars[i] == '[' {
            // Try to find the end of [text]
            let mut j = i + 1;
            while j < n && chars[j] != ']' {
                j += 1;
            }
            if j < n && j + 1 < n && chars[j + 1] == '(' {
                // Found [text]( — now find the closing )
                let link_text: String = chars[i + 1..j].iter().collect();
                let mut k = j + 2;
                while k < n && chars[k] != ')' {
                    k += 1;
                }
                if k < n {
                    result.push_str(&link_text);
                    i = k + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn remove_headers(text: &str) -> String {
    let mut result = String::new();
    for line in text.lines() {
        let stripped = line.trim_start_matches('#').trim_start();
        result.push_str(stripped);
        result.push('\n');
    }
    // Remove trailing newline if the original had none
    if !text.ends_with('\n') {
        result.pop();
    }
    result
}

fn remove_emphasis(text: &str) -> String {
    // Remove **, __, and lone * (but not inside words) markers
    let mut s = text.to_string();
    // Order matters: replace double markers first
    s = s.replace("**", "");
    s = s.replace("__", "");
    // Remove lone * used as italic (not part of bullets, handled separately)
    // Only remove * when surrounded by word chars or adjacent whitespace markers
    s = s.replace('*', "");
    s
}

fn remove_bullets(text: &str) -> String {
    let mut result = String::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let stripped = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            &trimmed[2..]
        } else {
            line
        };
        result.push_str(stripped);
        result.push('\n');
    }
    if !text.ends_with('\n') {
        result.pop();
    }
    result
}

fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_space = true; // start true to trim leading whitespace
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    // Trim trailing space
    if result.ends_with(' ') {
        result.pop();
    }
    result
}

/// Truncate `text` to at most `max_len` chars, breaking at a word boundary
/// and appending "...".  Returns the original string if it fits.
pub fn truncate(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }

    // Walk back from max_len to find a word boundary (space)
    let chars: Vec<char> = text.chars().collect();

    // Reserve 3 chars for "..."
    let target = max_len.saturating_sub(3);

    // Find the last space at or before `target`
    let mut cut = target;
    while cut > 0 && chars[cut] != ' ' {
        cut -= 1;
    }
    if cut == 0 {
        cut = target; // no space found, hard cut
    }

    let truncated: String = chars[..cut].iter().collect();
    format!("{}...", truncated.trim_end())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_msg(
        tool_name: Option<&str>,
        tool_input: serde_json::Value,
        text_content: &str,
        is_api_error: bool,
        error_status: Option<u16>,
    ) -> Message {
        Message {
            msg_type: "assistant".to_string(),
            text_content: text_content.to_string(),
            tool_name: tool_name.map(|s| s.to_string()),
            tool_input,
            is_api_error,
            error_status,
        }
    }

    // ── clean_markdown tests ───────────────────────────────────────────────────

    #[test]
    fn clean_markdown_formatting() {
        let input = "## Header\n**bold** and __under__ and *italic* and `code` and [link](https://example.com)";
        let result = clean_markdown(input);
        assert!(!result.contains('#'), "headers should be removed");
        assert!(!result.contains("**"), "bold markers should be removed");
        assert!(
            !result.contains("__"),
            "underline markers should be removed"
        );
        assert!(!result.contains('`'), "inline backticks should be removed");
        assert!(
            !result.contains("https://example.com"),
            "urls should be removed"
        );
        assert!(result.contains("link"), "link text should be preserved");
        assert!(result.contains("Header"), "header text should be preserved");
        assert!(result.contains("bold"), "bold text should be preserved");
    }

    #[test]
    fn clean_code_blocks() {
        let input = "Some text\n```rust\nlet x = 1;\nprintln!(\"{}\", x);\n```\nMore text";
        let result = clean_markdown(input);
        assert!(
            !result.contains("let x"),
            "code block content should be removed"
        );
        assert!(
            result.contains("Some text"),
            "text before code block should remain"
        );
        assert!(
            result.contains("More text"),
            "text after code block should remain"
        );
    }

    #[test]
    fn collapse_whitespace() {
        let input = "Hello   world\n\nfoo\tbar";
        let result = clean_markdown(input);
        assert!(!result.contains("  "), "double spaces should be collapsed");
        assert!(!result.contains('\n'), "newlines should be collapsed");
        assert!(!result.contains('\t'), "tabs should be collapsed");
        assert!(
            result.contains("Hello world"),
            "words should remain separated by single space"
        );
    }

    // ── truncate tests ─────────────────────────────────────────────────────────

    #[test]
    fn truncate_at_word_boundary() {
        let text = "The quick brown fox jumps over the lazy dog and then some more words here";
        let result = truncate(text, 30);
        assert!(
            result.ends_with("..."),
            "truncated text should end with ..."
        );
        assert!(
            result.chars().count() <= 30,
            "result should not exceed max_len"
        );
        // Should break at word boundary, not mid-word
        let without_ellipsis = result.trim_end_matches("...");
        assert!(
            !without_ellipsis.ends_with(' '),
            "no trailing space before ..."
        );
    }

    #[test]
    fn truncate_short_text_unchanged() {
        let text = "Short text";
        let result = truncate(text, 150);
        assert_eq!(result, text, "short text should be returned unchanged");
    }

    // ── extract_question tests ─────────────────────────────────────────────────

    #[test]
    fn extract_question_from_tool() {
        let messages = vec![make_msg(
            Some("AskUserQuestion"),
            json!({"question": "Would you like me to proceed with the refactoring?"}),
            "",
            false,
            None,
        )];
        let result = extract_question(&messages);
        assert!(result.contains("proceed"), "should extract question text");
        assert!(!result.is_empty(), "result should not be empty");
    }

    #[test]
    fn extract_question_fallback_to_text() {
        let messages = vec![make_msg(
            None,
            json!(null),
            "I've analyzed the code. Should I continue?",
            false,
            None,
        )];
        let result = extract_question(&messages);
        assert!(
            result.contains("Should I continue"),
            "should use text with '?'"
        );
    }

    // ── extract_work_summary tests ─────────────────────────────────────────────

    #[test]
    fn extract_work_summary_uses_last_text() {
        let messages = vec![
            make_msg(Some("Write"), json!(null), "", false, None),
            make_msg(Some("Write"), json!(null), "", false, None),
            make_msg(
                None,
                json!(null),
                "All files updated successfully.",
                false,
                None,
            ),
        ];
        let result = extract_work_summary(&messages);
        assert!(
            result.contains("All files updated"),
            "should use last assistant text, got: {result}"
        );
        assert!(!result.contains("write"), "should not contain tool counts");
    }

    // ── extract_summary dispatch tests ────────────────────────────────────────

    #[test]
    fn extract_session_limit() {
        let result = extract_summary(&Status::SessionLimit, &[]);
        assert!(
            result.contains("Session limit"),
            "should return session limit message"
        );
    }

    // ── edge cases ─────────────────────────────────────────────────────────────

    #[test]
    fn clean_and_truncate_150_char_limit() {
        let long = "word ".repeat(50); // 250 chars
        let result = clean_and_truncate(&long);
        assert!(
            result.chars().count() <= 150,
            "should be truncated to 150 chars"
        );
        assert!(
            result.ends_with("..."),
            "truncated result should end with ..."
        );
    }
}
