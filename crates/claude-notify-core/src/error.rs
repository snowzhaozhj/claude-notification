use thiserror::Error;

#[derive(Debug, Error)]
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

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub type Result<T> = std::result::Result<T, NotifyError>;
