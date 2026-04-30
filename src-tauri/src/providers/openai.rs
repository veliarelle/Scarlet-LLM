use crate::commands::attachments::extract_text_from_base64;
use crate::providers::{join_url, CompletionRequest, Provider, StreamCallback};
use crate::types::{CompletionResponse, Model, TokenUsage, ToolCall};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub const GROK_MODELS: &[(&str, &str)] = &[
    ("grok-imagine-image", "Grok Imagine Image"),
    ("grok-2-image", "Grok 2 Image"),
    ("grok-4.20-reasoning", "Grok 4.20 Reasoning"),
    ("grok-4", "Grok 4"),
    ("grok-3", "Grok 3"),
];

pub const GLM_MODELS: &[(&str, &str)] = &[
    ("glm-image", "GLM-Image"),
    ("cogview-4-250304", "CogView-4"),
    ("glm-5.1", "GLM-5.1"),
    ("glm-5-turbo", "GLM-5 Turbo"),
    ("glm-5", "GLM-5"),
    ("glm-4.7", "GLM-4.7"),
    ("glm-4.6", "GLM-4.6"),
    ("glm-4.5", "GLM-4.5"),
    ("glm-4.5-flash", "GLM-4.5 Flash"),
];

pub const DEEPSEEK_MODELS: &[(&str, &str)] = &[
    ("deepseek-v4-flash", "DeepSeek V4 Flash"),
    ("deepseek-v4-pro", "DeepSeek V4 Pro"),
    ("deepseek-chat", "DeepSeek Chat"),
    ("deepseek-reasoner", "DeepSeek Reasoner"),
];

pub const MISTRAL_MODELS: &[(&str, &str)] = &[
    ("mistral-large-latest", "Mistral Large"),
    ("mistral-medium-latest", "Mistral Medium"),
    ("mistral-small-latest", "Mistral Small"),
    ("codestral-latest", "Codestral"),
    ("ministral-8b-latest", "Ministral 8B"),
    ("ministral-3b-latest", "Ministral 3B"),
];

pub const MOONSHOT_MODELS: &[(&str, &str)] = &[
    ("kimi-k2.5", "Kimi K2.5"),
    ("kimi-k2-turbo-preview", "Kimi K2 Turbo"),
    ("kimi-k2-thinking", "Kimi K2 Thinking"),
    ("kimi-k2-thinking-turbo", "Kimi K2 Thinking Turbo"),
    ("moonshot-v1-8k", "Moonshot v1 8K"),
    ("moonshot-v1-32k", "Moonshot v1 32K"),
    ("moonshot-v1-128k", "Moonshot v1 128K"),
];

#[derive(Default)]
pub struct OpenAiProvider {
    fallback_models: &'static [(&'static str, &'static str)],
}

impl OpenAiProvider {
    pub fn with_fallback(fallback_models: &'static [(&'static str, &'static str)]) -> Self {
        Self { fallback_models }
    }
}

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
    content: Option<Value>,
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
    arguments: Value,
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

fn should_use_max_completion_tokens(base_url: &str) -> bool {
    base_url.contains("api.openai.com")
}

fn content_value_to_string(content: Option<Value>) -> String {
    match content {
        Some(Value::String(s)) => s,
        Some(Value::Array(parts)) => parts
            .into_iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| part.get("content").and_then(Value::as_str))
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>()
            .join(""),
        Some(other) => other.as_str().unwrap_or_default().to_string(),
        None => String::new(),
    }
}

fn arguments_to_string(arguments: Value) -> String {
    match arguments {
        Value::String(s) => s,
        other => other.to_string(),
    }
}

fn fallback_models(models: &'static [(&'static str, &'static str)]) -> Vec<Model> {
    models
        .iter()
        .map(|(id, name)| Model {
            id: (*id).to_string(),
            name: Some((*name).to_string()),
        })
        .collect()
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

fn normalize_file_part(part: &Value) -> Value {
    if let Some(file) = part.get("file") {
        let name = file
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or("attachment");
        let file_data = file.get("file_data").and_then(Value::as_str).unwrap_or("");
        let Some((media_type, data)) = parse_data_url(file_data) else {
            return part.clone();
        };
        if media_type == "application/pdf" {
            return part.clone();
        }
        return file_part_as_text(name, media_type, data);
    }
    let Some(source) = part.get("source") else {
        return part.clone();
    };
    let Some(media_type) = source.get("media_type").and_then(Value::as_str) else {
        return part.clone();
    };
    let Some(data) = source.get("data").and_then(Value::as_str) else {
        return part.clone();
    };
    let name = part
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("attachment");

    if media_type != "application/pdf" {
        return file_part_as_text(name, media_type, data);
    }

    let mut file = serde_json::Map::new();
    if !name.is_empty() {
        file.insert("filename".into(), Value::String(name.to_string()));
    }
    file.insert(
        "file_data".into(),
        Value::String(format!("data:{media_type};base64,{data}")),
    );

    json!({
        "type": "file",
        "file": Value::Object(file),
    })
}

fn file_part_as_text(name: &str, media_type: &str, data: &str) -> Value {
    let text = extract_text_from_base64(name, media_type, data)
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("[Attached file: {name}, {media_type}]\n\n{s}"))
        .unwrap_or_else(|| {
            format!(
                "[Attached file: {name}, {media_type}]\n\nThis file type cannot be sent as an inline document to OpenAI Chat Completions. Convert it to text or PDF if the model needs its contents."
            )
        });
    json!({ "type": "text", "text": text })
}

fn parse_data_url(file_data: &str) -> Option<(&str, &str)> {
    file_data
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(";base64,"))
}

fn normalize_openai_content(content: &Value) -> Value {
    match content {
        Value::Array(parts) => Value::Array(
            parts
                .iter()
                .map(|part| {
                    if part.get("type").and_then(Value::as_str) == Some("file") {
                        normalize_file_part(part)
                    } else {
                        part.clone()
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    }
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
            msg.insert("content".into(), normalize_openai_content(&m.content));
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
        let key = if k == "max_tokens" && should_use_max_completion_tokens(base_url) {
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
        let resp = match req.send().await {
            Ok(resp) => resp,
            Err(e) => {
                if !self.fallback_models.is_empty() {
                    return Ok(fallback_models(self.fallback_models));
                }
                return Err(format!("request failed: {e}"));
            }
        };
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("read body failed: {e}"))?;
        if !status.is_success() {
            if !self.fallback_models.is_empty() {
                return Ok(fallback_models(self.fallback_models));
            }
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ModelsResp = match serde_json::from_str(&text) {
            Ok(parsed) => parsed,
            Err(e) => {
                if !self.fallback_models.is_empty() {
                    return Ok(fallback_models(self.fallback_models));
                }
                return Err(format!("parse models response: {e}; body={text}"));
            }
        };
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
        let message = parsed.choices.into_iter().next().map(|c| c.message);
        let content = content_value_to_string(message.as_ref().and_then(|m| m.content.clone()));
        let tool_calls = message
            .map(|m| {
                m.tool_calls
                    .into_iter()
                    .map(|call| ToolCall {
                        id: call.id,
                        name: call.function.name,
                        arguments: arguments_to_string(call.function.arguments),
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
