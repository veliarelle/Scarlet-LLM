use crate::providers::{join_url, CompletionRequest, Provider, StreamCallback};
use crate::types::{CompletionResponse, Model, TokenUsage};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct OpenRouterProvider;

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
    images: Vec<ImageItem>,
}

#[derive(Debug, Deserialize)]
struct ImageItem {
    #[serde(default)]
    image_url: Option<ImageUrl>,
    #[serde(default, rename = "imageUrl")]
    image_url_camel: Option<ImageUrl>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageUrl {
    url: String,
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

fn build_body(req: &CompletionRequest, stream: bool) -> serde_json::Map<String, Value> {
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                crate::types::Role::System => "system",
                crate::types::Role::User => "user",
                crate::types::Role::Assistant => "assistant",
            };
            json!({ "role": role, "content": m.content })
        })
        .collect();

    let mut body = serde_json::Map::new();
    body.insert("model".into(), Value::String(req.model.clone()));
    body.insert("messages".into(), Value::Array(messages));
    if stream {
        body.insert("stream".into(), Value::Bool(true));
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }
    for (k, v) in &req.params {
        body.insert(k.clone(), v.clone());
    }

    let tools_arr = req.tools.clone();
    if !tools_arr.is_empty() {
        body.insert("tools".into(), Value::Array(tools_arr));
    }

    body
}

fn usage_from_wire(usage: Option<UsageWire>) -> Option<TokenUsage> {
    usage.map(|u| TokenUsage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    })
}

fn extract_image_url(message: &ChatMessageWire) -> Option<String> {
    message.images.iter().find_map(|img| {
        img.image_url
            .as_ref()
            .or(img.image_url_camel.as_ref())
            .map(|u| u.url.clone())
            .or_else(|| img.url.clone())
            .filter(|u| !u.is_empty())
    })
}

#[async_trait]
impl Provider for OpenRouterProvider {
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
        let text = resp
            .text()
            .await
            .map_err(|e| format!("read body failed: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ChatResp = serde_json::from_str(&text)
            .map_err(|e| format!("parse completion response: {e}; body={text}"))?;
        let Some(choice) = parsed.choices.into_iter().next() else {
            return Ok(CompletionResponse {
                content: String::new(),
                usage: usage_from_wire(parsed.usage),
                image_url: None,
            });
        };
        let image_url = extract_image_url(&choice.message);
        let content = choice.message.content.unwrap_or_default();
        Ok(CompletionResponse {
            content,
            usage: usage_from_wire(parsed.usage),
            image_url,
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
        let body = build_body(&req, true);
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
