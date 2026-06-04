use crate::config::LlmConfig;

/// Lightweight LLM client. Only used when explicitly invoked via /ask command.
pub struct LlmClient {
    config: LlmConfig,
    client: reqwest::blocking::Client,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Option<Self> {
        if config.api_key.is_empty() {
            return None;
        }
        Some(Self {
            config,
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .ok()?,
        })
    }

    pub fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    /// Send a chat request. Returns the response text or an error message.
    pub fn chat(&self, user_message: &str, history: &[ChatMessage]) -> Option<String> {
        let mut messages: Vec<serde_json::Value> = vec![serde_json::json!({
            "role": "system",
            "content": SYSTEM_PROMPT
        })];

        // Last 8 history messages
        let recent = if history.len() > 8 { &history[history.len() - 8..] } else { history };
        for msg in recent {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_message
        }));

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "stream": false,
            "max_tokens": 512
        });

        let url = format!("{}/chat/completions", self.config.endpoint.trim_end_matches('/'));

        match self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
        {
            Ok(response) => {
                let status = response.status();
                match response.text() {
                    Ok(text) => {
                        // Parse flexibly — handle both OpenAI and DeepSeek formats
                        match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(json) => {
                                // Try standard path: choices[0].message.content
                                if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                                    return Some(content.to_string());
                                }
                                // Try delta path (streaming response stored as single)
                                if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                    return Some(content.to_string());
                                }
                                // Try message directly
                                if let Some(content) = json["choices"][0]["text"].as_str() {
                                    return Some(content.to_string());
                                }
                                // Error in response
                                if let Some(err) = json["error"]["message"].as_str() {
                                    eprintln!("LLM API error: {err}");
                                    return Some(format!("API error: {err}"));
                                }
                                eprintln!("LLM: unexpected response shape: {text}");
                                None
                            }
                            Err(e) => {
                                eprintln!("LLM parse error (HTTP {status}): {e}");
                                eprintln!("Body: {}", &text[..text.len().min(500)]);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("LLM read error: {e}");
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("LLM request error: {e}");
                None
            }
        }
    }
}

const SYSTEM_PROMPT: &str = r#"You are Cortana, a terminal-native AI assistant.
Rules:
- Be extremely concise. Terminal users hate walls of text.
- One to three sentences maximum unless asked for detail.
- You have personality — be witty, not robotic.
- Focus on code, project context, and technical questions.
- If you don't know something, say so directly."#;
