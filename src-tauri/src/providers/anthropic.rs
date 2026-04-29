use crate::commands::attachments::extract_text_from_base64;
use crate::providers::{CompletionRequest, Provider, StreamCallback, StreamItem};
use crate::types::{CompletionResponse, Model, Role, TokenUsage, ToolCall};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct AnthropicProvider;

const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Deserialize)]
struct ModelsResp {
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessagesResp {
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
struct UsageWire {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    delta: Option<StreamDelta>,
    #[serde(default)]
    usage: Option<UsageWire>,
    #[serde(default)]
    message: Option<MessageStart>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageStart {
    #[serde(default)]
    usage: Option<UsageWire>,
}

/// Convert OpenAI-style content (string or parts array) to Anthropic format.
fn to_anthropic_content(content: &Value) -> Value {
    match content {
        Value::Array(parts) => {
            let converted: Vec<Value> = parts
                .iter()
                .filter_map(|part| {
                    let t = part.get("type")?.as_str()?;
                    match t {
                        "text" => {
                            Some(json!({ "type": "text", "text": part.get("text")?.as_str()? }))
                        }
                        "image_url" => {
                            let url = part.get("image_url")?.get("url")?.as_str()?;
                            let rest = url.strip_prefix("data:")?;
                            let (mime, b64) = rest.split_once(";base64,")?;
                            Some(json!({
                                "type": "image",
                                "source": { "type": "base64", "media_type": mime, "data": b64 }
                            }))
                        }
                        "file" => {
                            let src = part.get("source")?;
                            let mime = src.get("media_type")?.as_str()?;
                            let data = src.get("data")?.as_str()?;
                            if mime == "application/pdf" || mime == "text/plain" {
                                Some(json!({
                                    "type": "document",
                                    "source": { "type": "base64", "media_type": mime, "data": data }
                                }))
                            } else {
                                Some(file_part_as_text(
                                    part.get("name").and_then(Value::as_str).unwrap_or("attachment"),
                                    mime,
                                    data,
                                ))
                            }
                        }
                        _ => None,
                    }
                })
                .collect();
            Value::Array(converted)
        }
        other => other.clone(),
    }
}

fn file_part_as_text(name: &str, media_type: &str, data: &str) -> Value {
    let text = extract_text_from_base64(name, media_type, data)
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("[Attached file: {name}, {media_type}]\n\n{s}"))
        .unwrap_or_else(|| {
            format!(
                "[Attached file: {name}, {media_type}]\n\nThis file type was not sent as a binary document because Claude document blocks are safest with PDF/plain text."
            )
        });
    json!({ "type": "text", "text": text })
}

fn cache_control_block() -> Value {
    json!({ "type": "ephemeral" })
}

fn add_cache_control_to_content(content: Value) -> Value {
    match content {
        Value::String(text) => json!([
            {
                "type": "text",
                "text": text,
                "cache_control": cache_control_block(),
            }
        ]),
        Value::Array(mut blocks) => {
            if let Some(Value::Object(last)) = blocks.last_mut() {
                last.insert("cache_control".into(), cache_control_block());
            }
            Value::Array(blocks)
        }
        other => other,
    }
}

fn build_body(req: &CompletionRequest, stream: bool) -> Value {
    let mut system = String::new();
    let mut messages: Vec<Value> = Vec::new();

    for msg in &req.messages {
        match msg.role {
            Role::System => {
                if !system.is_empty() {
                    system.push('\n');
                }
                if let Some(s) = msg.content.as_str() {
                    system.push_str(s);
                }
            }
            Role::User => {
                let content = to_anthropic_content(&msg.content);
                messages.push(json!({ "role": "user", "content": content }));
            }
            Role::Assistant => {
                let mut blocks = Vec::new();
                if !msg.content.as_str().unwrap_or_default().is_empty() {
                    match to_anthropic_content(&msg.content) {
                        Value::Array(parts) => blocks.extend(parts),
                        other => blocks.push(other),
                    }
                }
                for call in &msg.tool_calls {
                    let input = serde_json::from_str::<Value>(&call.arguments)
                        .unwrap_or_else(|_| json!({ "raw": call.arguments }));
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": call.id,
                        "name": call.name,
                        "input": input,
                    }));
                }
                messages.push(json!({ "role": "assistant", "content": blocks }));
            }
            Role::Tool => {
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": msg.tool_call_id.as_deref().unwrap_or_default(),
                        "content": msg.content.as_str().unwrap_or_default(),
                    }]
                }));
            }
        }
    }

    let mut tools_arr = req.tools.clone();
    if req.web_search {
        let has_ws = tools_arr
            .iter()
            .any(|t| t.get("type").and_then(|v| v.as_str()) == Some("web_search_20250305"));
        if !has_ws {
            tools_arr.push(json!({ "type": "web_search_20250305", "name": "web_search" }));
        }
    }

    let mut body = serde_json::Map::new();
    body.insert("model".into(), Value::String(req.model.clone()));
    body.insert("messages".into(), Value::Array(messages));
    if !system.is_empty() {
        if req.prompt_caching {
            body.insert(
                "system".into(),
                json!([
                    {
                        "type": "text",
                        "text": system,
                        "cache_control": cache_control_block(),
                    }
                ]),
            );
        } else {
            body.insert("system".into(), Value::String(system));
        }
    } else if req.prompt_caching {
        if let Some(Value::Object(first_message)) = body
            .get_mut("messages")
            .and_then(Value::as_array_mut)
            .and_then(|messages| messages.first_mut())
        {
            if let Some(content) = first_message.remove("content") {
                first_message.insert("content".into(), add_cache_control_to_content(content));
            }
        }
    }
    let converted: Vec<Value> = tools_arr
        .into_iter()
        .map(|t| {
            if t.get("type").and_then(|v| v.as_str()) == Some("function") {
                if let Some(func) = t.get("function") {
                    return json!({
                        "name": func.get("name").cloned().unwrap_or(Value::Null),
                        "description": func.get("description").cloned().unwrap_or(Value::Null),
                        "input_schema": func.get("parameters").cloned().unwrap_or_else(|| json!({ "type": "object" })),
                    });
                }
            }
            t
        })
        .collect();
    if !converted.is_empty() {
        body.insert("tools".into(), Value::Array(converted));
    }
    if stream {
        body.insert("stream".into(), Value::Bool(true));
    }
    for (k, v) in &req.params {
        body.insert(k.clone(), v.clone());
    }

    Value::Object(body)
}

fn add_headers(
    req: reqwest::RequestBuilder,
    key: &str,
    web_search: bool,
) -> reqwest::RequestBuilder {
    let mut r = req
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION);
    if web_search {
        r = r.header("anthropic-beta", "web-search-2025-03-05");
    }
    r
}

#[async_trait]
impl Provider for AnthropicProvider {
    async fn list_models(&self, base_url: &str, key: &str) -> Result<Vec<Model>, String> {
        let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
        let resp = reqwest::Client::new()
            .get(&url)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ModelsResp =
            serde_json::from_str(&text).map_err(|e| format!("parse models: {e}; body={text}"))?;
        Ok(parsed
            .data
            .into_iter()
            .map(|m| Model {
                id: m.id,
                name: m.display_name,
            })
            .collect())
    }

    async fn complete(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
    ) -> Result<CompletionResponse, String> {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        let body = build_body(&req, false);
        let resp = add_headers(reqwest::Client::new().post(&url), key, req.web_search)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: MessagesResp =
            serde_json::from_str(&text).map_err(|e| format!("parse response: {e}; body={text}"))?;
        let content = parsed
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("");
        let tool_calls = parsed
            .content
            .into_iter()
            .filter(|b| b.kind == "tool_use")
            .filter_map(|b| {
                let id = b.id?;
                let name = b.name?;
                let arguments = b
                    .input
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "{}".to_string());
                Some(ToolCall {
                    id,
                    name,
                    arguments,
                })
            })
            .collect();
        Ok(CompletionResponse {
            content,
            usage: parsed.usage.map(|u| TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
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
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        let body = build_body(&req, true);
        let http = add_headers(reqwest::Client::new().post(&url), key, req.web_search).json(&body);

        let resp = tokio::select! {
            biased;
            _ = cancel.notified() => {
                cb(StreamItem::Done { usage: None });
                return Ok(());
            }
            r = http.send() => r.map_err(|e| format!("request failed: {e}"))?,
        };
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {text}"));
        }

        let mut input_tokens: u32 = 0;
        let mut output_tokens: u32 = 0;
        let mut stream = resp.bytes_stream().eventsource();

        loop {
            tokio::select! {
                biased;
                _ = cancel.notified() => { break; }
                ev = stream.next() => {
                    let Some(event) = ev else { break };
                    let event = event.map_err(|e| format!("stream error: {e}"))?;
                    if event.data.is_empty() { continue; }
                    let evt: StreamEvent = match serde_json::from_str(&event.data) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    match evt.kind.as_str() {
                        "message_start" => {
                            if let Some(msg) = evt.message {
                                if let Some(u) = msg.usage {
                                    input_tokens = u.input_tokens;
                                }
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = evt.delta {
                                if delta.kind == "text_delta" {
                                    if let Some(text) = delta.text {
                                        if !text.is_empty() {
                                            cb(StreamItem::Chunk(text));
                                        }
                                    }
                                }
                            }
                        }
                        "message_delta" => {
                            if let Some(u) = evt.usage {
                                output_tokens = u.output_tokens;
                            }
                        }
                        "message_stop" => { break; }
                        _ => {}
                    }
                }
            }
        }

        let usage = if input_tokens > 0 || output_tokens > 0 {
            Some(TokenUsage {
                prompt_tokens: input_tokens,
                completion_tokens: output_tokens,
                total_tokens: input_tokens + output_tokens,
            })
        } else {
            None
        };
        cb(StreamItem::Done { usage });
        Ok(())
    }
}
