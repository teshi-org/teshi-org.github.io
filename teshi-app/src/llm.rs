//! LLM integration for the WASM build.
//!
//! Uses `reqwest` to make async HTTP requests to an OpenAI-compatible API.
//! Non-streaming mode only (streaming requires SSE parsing which adds
//! complexity; can be added later via `reqwest`'s streaming support).

use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;

/// Configuration for the LLM client.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            max_tokens: 1024,
            temperature: 0.7,
        }
    }
}

/// A chat message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A callback invoked when a chunk of the LLM response is received.
pub type ChunkCallback = Box<dyn FnMut(String)>;
/// A callback invoked when the LLM response is complete.
pub type DoneCallback = Box<dyn FnMut(String, Option<String>)>;
/// A callback invoked on error.
pub type ErrorCallback = Box<dyn FnMut(String)>;

/// Send a chat completion request to the LLM API.
///
/// The request is dispatched asynchronously; results arrive via the callbacks.
/// Only non-streaming requests are supported in this WASM build.
pub fn send_chat_request(
    config: LlmConfig,
    system: Option<String>,
    messages: Vec<ChatMessage>,
    on_chunk: ChunkCallback,
    on_done: DoneCallback,
    on_error: ErrorCallback,
) {
    spawn_local(async move {
        let client = reqwest::Client::new();

        let mut request_messages: Vec<serde_json::Value> = Vec::new();

        if let Some(sys) = system {
            request_messages.push(serde_json::json!({
                "role": "system",
                "content": sys
            }));
        }

        for msg in messages {
            request_messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        let body = serde_json::json!({
            "model": config.model,
            "messages": request_messages,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "stream": false
        });

        let mut on_chunk = on_chunk;
        let mut on_done = on_done;
        let mut on_error = on_error;

        match client
            .post(format!("{}/chat/completions", config.base_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
                            let reasoning = json["choices"][0]["message"]["reasoning_content"]
                                .as_str()
                                .map(|s| s.to_string());
                            // Simulate streaming by delivering the full response as a single chunk
                            on_chunk(content.to_string());
                            on_done(content.to_string(), reasoning);
                        } else {
                            let msg = json["error"]["message"]
                                .as_str()
                                .unwrap_or("Unknown API error")
                                .to_string();
                            on_error(msg);
                        }
                    }
                    Err(e) => {
                        on_error(format!("Failed to parse response: {}", e));
                    }
                }
            }
            Err(e) => {
                on_error(format!("Request failed: {}", e));
            }
        }
    });
}
