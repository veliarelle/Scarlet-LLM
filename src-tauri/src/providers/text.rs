use crate::commands::attachments::extract_text_from_base64;
use crate::providers::{join_url, CompletionRequest, Provider, StreamCallback, StreamItem};
use crate::types::{CompletionResponse, Model, Role, TokenUsage};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;

pub struct TextCompletionsProvider;

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
struct CompletionResp {
    #[serde(default)]
    choices: Vec<CompletionChoice>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<CompletionChoice>,
    #[serde(default)]
    usage: Option<UsageWire>,
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

fn usage_from_wire(usage: Option<UsageWire>) -> Option<TokenUsage> {
    usage.map(|u| TokenUsage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    })
}

fn file_data_text(name: &str, file_data: &str) -> String {
    let (mime, data) = file_data
        .strip_prefix("data:")
        .and_then(|rest| rest.split_once(";base64,"))
        .unwrap_or(("application/octet-stream", file_data));
    extract_text_from_base64(name, mime, data)
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("[Attached file: {name}, {mime}]\n{s}"))
        .unwrap_or_else(|| format!("[Attached file: {name}, {mime}]"))
}

fn file_source_text(part: &Value) -> String {
    if let Some(file) = part.get("file") {
        let name = file
            .get("filename")
            .and_then(Value::as_str)
            .unwrap_or("attachment");
        let data = file.get("file_data").and_then(Value::as_str).unwrap_or("");
        return file_data_text(name, data);
    }

    let name = part
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("attachment");
    let Some(source) = part.get("source") else {
        return format!("[Attached file: {name}]");
    };
    let mime = source
        .get("media_type")
        .and_then(Value::as_str)
        .unwrap_or("application/octet-stream");
    let data = source.get("data").and_then(Value::as_str).unwrap_or("");
    extract_text_from_base64(name, mime, data)
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("[Attached file: {name}, {mime}]\n{s}"))
        .unwrap_or_else(|| format!("[Attached file: {name}, {mime}]"))
}

fn content_to_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .map(|part| match part.get("type").and_then(Value::as_str) {
                Some("text" | "input_text") => part
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                Some("image_url" | "input_image") => "[Attached image]".to_string(),
                Some("file" | "input_file") => file_source_text(part),
                _ => part
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| part.to_string()),
            })
            .filter(|s| !s.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n"),
        other => other.as_str().unwrap_or_default().to_string(),
    }
}

fn prompt_from_messages(req: &CompletionRequest) -> String {
    let mut out = String::new();
    for msg in &req.messages {
        let label = match msg.role {
            Role::System => "System",
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::Tool => "Tool",
        };
        let text = content_to_text(&msg.content);
        if text.trim().is_empty() && msg.tool_calls.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(label);
        if let Some(name) = msg.name.as_ref().filter(|s| !s.trim().is_empty()) {
            out.push_str(" (");
            out.push_str(name);
            out.push(')');
        }
        out.push_str(":\n");
        out.push_str(text.trim());
        if !msg.tool_calls.is_empty() {
            for call in &msg.tool_calls {
                out.push_str("\n[Tool call: ");
                out.push_str(&call.name);
                out.push_str(" ");
                out.push_str(&call.arguments);
                out.push(']');
            }
        }
    }
    if !out.is_empty() {
        out.push_str("\n\nAssistant:\n");
    }
    out
}

fn build_body(req: &CompletionRequest, stream: bool) -> serde_json::Map<String, Value> {
    const STRIP: &[&str] = &[
        "messages",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "response_format",
        "stream_options",
        "max_completion_tokens",
    ];
    let mut body = serde_json::Map::new();
    body.insert("model".into(), Value::String(req.model.clone()));
    body.insert("prompt".into(), Value::String(prompt_from_messages(req)));
    body.insert("stream".into(), Value::Bool(stream));

    for (key, value) in &req.params {
        if STRIP.contains(&key.as_str()) {
            continue;
        }
        let mapped = if key == "max_output_tokens" {
            "max_tokens"
        } else {
            key.as_str()
        };
        body.insert(mapped.to_string(), value.clone());
    }
    body
}

#[async_trait]
impl Provider for TextCompletionsProvider {
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
        let url = join_url(base_url, "completions");
        let body = build_body(&req, false);
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
        let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: CompletionResp = serde_json::from_str(&text)
            .map_err(|e| format!("parse completion: {e}; body={text}"))?;
        Ok(CompletionResponse {
            content: parsed
                .choices
                .into_iter()
                .next()
                .map(|c| c.text)
                .unwrap_or_default(),
            usage: usage_from_wire(parsed.usage),
            image_url: None,
            tool_calls: Vec::new(),
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
        let url = join_url(base_url, "completions");
        let body = build_body(&req, true);
        let client = reqwest::Client::new();
        let mut http = client.post(&url).json(&body);
        if !key.is_empty() {
            http = http.bearer_auth(key);
        }

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

        let mut usage: Option<TokenUsage> = None;
        let mut stream = resp.bytes_stream().eventsource();
        loop {
            tokio::select! {
                biased;
                _ = cancel.notified() => break,
                ev = stream.next() => {
                    let Some(event) = ev else { break };
                    let event = event.map_err(|e| format!("stream error: {e}"))?;
                    if event.data == "[DONE]" { break; }
                    if event.data.is_empty() { continue; }
                    let chunk: StreamChunk = match serde_json::from_str(&event.data) {
                        Ok(chunk) => chunk,
                        Err(_) => continue,
                    };
                    if let Some(u) = chunk.usage {
                        usage = usage_from_wire(Some(u));
                    }
                    for choice in chunk.choices {
                        if !choice.text.is_empty() {
                            cb(StreamItem::Chunk(choice.text));
                        }
                    }
                }
            }
        }
        cb(StreamItem::Done { usage });
        Ok(())
    }
}
