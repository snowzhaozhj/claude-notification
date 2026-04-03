use std::io::Read;

use serde::{Deserialize, Serialize};

use crate::error::{NotifyError, Result};

/// Represents the JSON input passed to a Claude hook via stdin.
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
    /// Parse a `HookInput` from a JSON string.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| NotifyError::HookInput(e.to_string()))
    }

    /// Parse a `HookInput` from any `Read` implementor.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(|e| NotifyError::HookInput(e.to_string()))
    }

    /// Parse a `HookInput` from stdin.
    pub fn from_stdin() -> Result<Self> {
        Self::from_reader(std::io::stdin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_JSON: &str = r#"{
        "session_id": "abc-123",
        "transcript_path": "/tmp/transcript.json",
        "tool_name": "Bash",
        "tool_input": {"command": "ls"},
        "tool_result": {"output": "file.txt"},
        "is_team_lead": true,
        "team_name": "core",
        "isApiErrorMessage": true
    }"#;

    const MINIMAL_JSON: &str = r#"{
        "session_id": "minimal-session",
        "transcript_path": "/tmp/minimal.json"
    }"#;

    #[test]
    fn parse_valid_hook_input() {
        let input = HookInput::from_json(FULL_JSON).expect("should parse full JSON");

        assert_eq!(input.session_id, "abc-123");
        assert_eq!(input.transcript_path, "/tmp/transcript.json");
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.tool_input, serde_json::json!({"command": "ls"}));
        assert_eq!(input.tool_result, serde_json::json!({"output": "file.txt"}));
        assert!(input.is_team_lead);
        assert_eq!(input.team_name, "core");
        assert!(input.is_api_error_message);
    }

    #[test]
    fn parse_minimal_hook_input() {
        let input = HookInput::from_json(MINIMAL_JSON).expect("should parse minimal JSON");

        assert_eq!(input.session_id, "minimal-session");
        assert_eq!(input.transcript_path, "/tmp/minimal.json");
        assert_eq!(input.tool_name, "");
        assert_eq!(input.tool_input, serde_json::Value::Null);
        assert_eq!(input.tool_result, serde_json::Value::Null);
        assert!(!input.is_team_lead);
        assert_eq!(input.team_name, "");
        assert!(!input.is_api_error_message);
    }

    #[test]
    fn parse_invalid_json() {
        let result = HookInput::from_json("{ not valid json }");
        assert!(result.is_err());
        match result.unwrap_err() {
            NotifyError::HookInput(_) => {}
            other => panic!("expected HookInput error, got: {:?}", other),
        }
    }

    #[test]
    fn parse_hook_input_from_reader() {
        let cursor = std::io::Cursor::new(FULL_JSON.as_bytes());
        let input = HookInput::from_reader(cursor).expect("should parse from reader");

        assert_eq!(input.session_id, "abc-123");
        assert_eq!(input.transcript_path, "/tmp/transcript.json");
        assert_eq!(input.tool_name, "Bash");
        assert!(input.is_team_lead);
        assert!(input.is_api_error_message);
    }
}
