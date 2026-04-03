use crate::traits::Dispatcher;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookPreset {
    Slack,
    Discord,
    Telegram,
    Lark,
    Custom,
}

impl WebhookPreset {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "slack" => Self::Slack,
            "discord" => Self::Discord,
            "telegram" => Self::Telegram,
            "lark" => Self::Lark,
            _ => Self::Custom,
        }
    }
}

pub struct WebhookDispatcher {
    pub url: String,
    pub preset: WebhookPreset,
    pub chat_id: Option<String>,
    pub headers: HashMap<String, String>,
    pub template: Option<String>,
    pub retry_max: u32,
    pub timeout_seconds: u64,
}

impl WebhookDispatcher {
    pub fn new(url: String) -> Self {
        Self {
            url,
            preset: WebhookPreset::Custom,
            chat_id: None,
            headers: HashMap::new(),
            template: None,
            retry_max: 3,
            timeout_seconds: 10,
        }
    }

    pub fn with_preset(mut self, preset: WebhookPreset) -> Self {
        self.preset = preset;
        self
    }

    pub fn with_chat_id(mut self, chat_id: String) -> Self {
        self.chat_id = Some(chat_id);
        self
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn with_retry_max(mut self, retry_max: u32) -> Self {
        self.retry_max = retry_max;
        self
    }

    pub fn with_timeout_seconds(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    pub fn format_payload(&self, title: &str, body: &str) -> Value {
        match &self.preset {
            WebhookPreset::Slack => json!({
                "text": format!("*{}*\n{}", title, body)
            }),
            WebhookPreset::Discord => json!({
                "embeds": [{
                    "title": title,
                    "description": body
                }]
            }),
            WebhookPreset::Telegram => json!({
                "chat_id": self.chat_id.as_deref().unwrap_or(""),
                "text": format!("<b>{}</b>\n{}", title, body),
                "parse_mode": "HTML"
            }),
            WebhookPreset::Lark => json!({
                "msg_type": "text",
                "content": {
                    "text": format!("{}\n{}", title, body)
                }
            }),
            WebhookPreset::Custom => {
                if let Some(tmpl) = &self.template {
                    let rendered = tmpl.replace("{{title}}", title).replace("{{body}}", body);
                    serde_json::from_str(&rendered).unwrap_or_else(|_| {
                        json!({
                            "title": title,
                            "body": body
                        })
                    })
                } else {
                    json!({
                        "title": title,
                        "body": body
                    })
                }
            }
        }
    }

    pub fn send_with_retry(&self, payload: &Value) -> Result<(), String> {
        let payload_str = payload.to_string();
        let mut attempt = 0u32;

        loop {
            let result = self.send_once(&payload_str);
            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    attempt += 1;
                    if attempt > self.retry_max {
                        return Err(format!(
                            "Webhook failed after {} retries: {}",
                            self.retry_max, e
                        ));
                    }
                    let backoff = 1u64 << (attempt - 1); // 1s, 2s, 4s
                    tracing::warn!(
                        "Webhook attempt {} failed: {}. Retrying in {}s",
                        attempt,
                        e,
                        backoff
                    );
                    std::thread::sleep(Duration::from_secs(backoff));
                }
            }
        }
    }

    fn send_once(&self, payload_str: &str) -> Result<(), String> {
        let timeout = Duration::from_secs(self.timeout_seconds);
        let agent = ureq::AgentBuilder::new().timeout(timeout).build();

        let mut req = agent.post(&self.url);
        for (k, v) in &self.headers {
            req = req.set(k, v);
        }

        req.send_string(payload_str)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

impl Dispatcher for WebhookDispatcher {
    fn dispatch(&self, title: &str, body: &str) -> Result<(), String> {
        let payload = self.format_payload(title, body);
        self.send_with_retry(&payload)
    }
}
