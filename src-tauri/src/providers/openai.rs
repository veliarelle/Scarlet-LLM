use crate::providers::{join_url, CompletionRequest, Provider, StreamCallback};
use crate::types::{CompletionResponse, Model, TokenUsage, ToolCall};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct OpenAiProvider;

#[derive(Debug, Deserialize)]
struct ModelsResp {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResp {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessageWire,
}

#[derive(Debug, Deserialize)]
struct ChatMessageWire {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCallWire>,
}

#[derive(Debug, Deserialize)]
struct ToolCallWire {
    id: String,
    function: ToolCallFunctionWire,
}

#[derive(Debug, Deserialize)]
struct ToolCallFunctionWire {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct UsageWire {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
}

#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
}

fn supports_openai_prompt_cache_controls(base_url: &str) -> bool {
    base_url.contains("api.openai.com")
}

fn prompt_cache_key(model: &str) -> String {
    let model = model
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("scarlet-llm-{model}")
}

fn build_body(
    req: &CompletionRequest,
    stream: bool,
    base_url: &str,
) -> serde_json::Map<String, Value> {
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                crate::types::Role::System => "system",
                crate::types::Role::User => "user",
                crate::types::Role::Assistant => "assistant",
                crate::types::Role::Tool => "tool",
            };
            let mut msg = serde_json::Map::new();
            msg.insert("role".into(), Value::String(role.to_string()));
            msg.insert("content".into(), m.content.clone());
            if let Some(name) = m.name.as_ref() {
                msg.insert("name".into(), Value::String(name.clone()));
            }
            if let Some(tool_call_id) = m.tool_call_id.as_ref() {
                msg.insert("tool_call_id".into(), Value::String(tool_call_id.clone()));
            }
            if !m.tool_calls.is_empty() {
                let calls = m
                    .tool_calls
                    .iter()
                    .map(|call| {
                        json!({
                            "id": call.id,
                            "type": "function",
                            "function": {
                                "name": call.name,
                                "arguments": call.arguments,
                            }
                        })
                    })
                    .collect();
                msg.insert("tool_calls".into(), Value::Array(calls));
            }
            Value::Object(msg)
        })
        .collect();

    let mut body = serde_json::Map::new();
    body.insert("model".into(), Value::String(req.model.clone()));
    body.insert("messages".into(), Value::Array(messages));
    if stream {
        body.insert("stream".into(), Value::Bool(true));
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }
    if req.prompt_caching && supports_openai_prompt_cache_controls(base_url) {
        body.insert(
            "prompt_cache_key".into(),
            Value::String(prompt_cache_key(&req.model)),
        );
    }
    for (k, v) in &req.params {
        // Official OpenAI API (gpt-4.1, o1, o3, etc.) uses max_completion_tokens
        let key = if k == "max_tokens" {
            "max_completion_tokens"
        } else {
            k.as_str()
        };
        body.insert(key.to_string(), v.clone());
    }

    // Build tools array (web_search_preview is Responses API only, not chat completions)
    let tools_arr = req.tools.clone();
    if !tools_arr.is_empty() {
        body.insert("tools".into(), Value::Array(tools_arr));
    }

    body
}

#[async_trait]
impl Provider for OpenAiProvider {
    async fn list_models(&self, base_url: &str, key: &str) -> Result<Vec<Model>, String> {
        let url = join_url(base_url, "models");
        let client = reqwest::Client::new();
        let mut req = client.get(&url);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("read body failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ModelsResp = serde_json::from_str(&text)
            .map_err(|e| format!("parse models response: {e}; body={text}"))?;
        Ok(parsed
            .data
            .into_iter()
            .map(|m| Model {
                id: m.id,
                name: m.name,
            })
            .collect())
    }

    async fn complete(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
    ) -> Result<CompletionResponse, String> {
        let url = join_url(base_url, "chat/completions");
        let body = build_body(&req, false, base_url);
        let client = reqwest::Client::new();
        let mut http = client.post(&url).json(&body);
        if !key.is_empty() {
            http = http.bearer_auth(key);
        }
        let resp = http
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("read body failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ChatResp = serde_json::from_str(&text)
            .map_err(|e| format!("parse completion response: {e}; body={text}"))?;
        let message = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message);
        let content = message
            .as_ref()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();
        let tool_calls = message
            .map(|m| {
                m.tool_calls
                    .into_iter()
                    .map(|call| ToolCall {
                        id: call.id,
                        name: call.function.name,
                        arguments: call.function.arguments,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(CompletionResponse {
            content,
            usage: parsed.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            image_url: None,
            tool_calls,
        })
    }

    async fn complete_stream(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
        cb: StreamCallback,
        cancel: Arc<Notify>,
    ) -> Result<(), String> {
        let url = join_url(base_url, "chat/completions");
        let body = build_body(&req, true, base_url);
        let client = reqwest::Client::new();
        let mut http = client.post(&url).json(&body);
        if !key.is_empty() {
            http = http.bearer_auth(key);
        }

        let resp = tokio::select! {
            biased;
            _ = cancel.notified() => {
                cb(crate::providers::StreamItem::Done { usage: None });
                return Ok(());
            }
            r = http.send() => r.map_err(|e| format!("request failed: {e}"))?,
        };
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {text}"));
        }

        let mut usage: Option<TokenUsage> = None;
        let mut stream = resp.bytes_stream().eventsource();
        loop {
            tokio::select! {
                biased;
                _ = cancel.notified() => {
                    break;
                }
                ev = stream.next() => {
                    let Some(event) = ev else { break };
                    let event = event.map_err(|e| format!("stream error: {e}"))?;
                    if event.data == "[DONE]" { break; }
                    if event.data.is_empty() { continue; }
                    let chunk: StreamChunk = match serde_json::from_str(&event.data) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if let Some(u) = chunk.usage {
                        usage = Some(TokenUsage {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                        });
                    }
                    for choice in chunk.choices {
                        if let Some(delta) = choice.delta.content {
                            if !delta.is_empty() {
                                cb(crate::providers::StreamItem::Chunk(delta));
                            }
                        }
                    }
                }
            }
        }
        cb(crate::providers::StreamItem::Done { usage });
        Ok(())
    }
}
