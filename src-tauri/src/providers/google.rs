use crate::providers::{CompletionRequest, Provider, StreamCallback, StreamItem};
use crate::types::{CompletionResponse, Model, Role, TokenUsage};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct GoogleProvider;

#[derive(Debug, Deserialize)]
struct ModelsResp {
    #[serde(default)]
    models: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    name: String,
    #[serde(rename = "displayName", default)]
    display_name: Option<String>,
    #[serde(rename = "supportedGenerationMethods", default)]
    supported_methods: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GenerateResp {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata", default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    #[serde(default)]
    text: Option<String>,
    #[serde(rename = "inlineData", alias = "inline_data", default)]
    inline_data: Option<InlineData>,
}

#[derive(Debug, Deserialize)]
struct InlineData {
    #[serde(rename = "mimeType", alias = "mime_type", default)]
    mime_type: Option<String>,
    data: String,
}

#[derive(Debug, Default, Deserialize)]
struct UsageWire {
    #[serde(rename = "promptTokenCount", default)]
    prompt_tokens: u32,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_tokens: u32,
    #[serde(rename = "totalTokenCount", default)]
    total_tokens: u32,
}

fn model_endpoint(base_url: &str, model: &str, method: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/v1beta/models/{model}:{method}")
}

/// Convert OpenAI-style content (string or parts array) to Google `parts` array.
fn to_google_parts(content: &Value) -> Vec<Value> {
    match content {
        Value::String(s) => vec![json!({ "text": s })],
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| {
                let t = part.get("type")?.as_str()?;
                match t {
                    "text" => Some(json!({ "text": part.get("text")?.as_str()? })),
                    "image_url" => {
                        let url = part.get("image_url")?.get("url")?.as_str()?;
                        let rest = url.strip_prefix("data:")?;
                        let (mime, b64) = rest.split_once(";base64,")?;
                        Some(json!({ "inlineData": { "mimeType": mime, "data": b64 } }))
                    }
                    "file" => {
                        let src = part.get("source")?;
                        let mime = src.get("media_type")?.as_str()?;
                        let data = src.get("data")?.as_str()?;
                        Some(json!({ "inlineData": { "mimeType": mime, "data": data } }))
                    }
                    _ => None,
                }
            })
            .collect(),
        _ => vec![],
    }
}

fn build_body(req: &CompletionRequest) -> Value {
    let mut system_parts: Vec<Value> = Vec::new();
    let mut contents: Vec<Value> = Vec::new();

    for msg in &req.messages {
        match msg.role {
            Role::System => {
                if let Some(s) = msg.content.as_str() {
                    system_parts.push(json!({ "text": s }));
                }
            }
            Role::User => {
                let parts = to_google_parts(&msg.content);
                contents.push(json!({ "role": "user", "parts": parts }));
            }
            Role::Assistant => {
                let parts = to_google_parts(&msg.content);
                contents.push(json!({ "role": "model", "parts": parts }));
            }
        }
    }

    let mut gen_config = serde_json::Map::new();
    // Map standard param names to Google's camelCase
    if let Some(v) = req.params.get("max_tokens") {
        gen_config.insert("maxOutputTokens".into(), v.clone());
    }
    if let Some(v) = req.params.get("temperature") {
        gen_config.insert("temperature".into(), v.clone());
    }
    if let Some(v) = req.params.get("top_p") {
        gen_config.insert("topP".into(), v.clone());
    }
    if let Some(v) = req.params.get("top_k") {
        gen_config.insert("topK".into(), v.clone());
    }

    let mut body = serde_json::Map::new();
    body.insert("contents".into(), Value::Array(contents));
    if !system_parts.is_empty() {
        body.insert("systemInstruction".into(), json!({ "parts": system_parts }));
    }
    if !gen_config.is_empty() {
        body.insert("generationConfig".into(), Value::Object(gen_config));
    }

    Value::Object(body)
}

fn extract_text(resp: &GenerateResp) -> String {
    resp.candidates
        .iter()
        .filter_map(|c| c.content.as_ref())
        .flat_map(|c| c.parts.iter())
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("")
}

fn extract_image_url(resp: &GenerateResp) -> Option<String> {
    resp.candidates
        .iter()
        .filter_map(|c| c.content.as_ref())
        .flat_map(|c| c.parts.iter())
        .filter_map(|p| p.inline_data.as_ref())
        .find_map(|d| {
            let mime = d.mime_type.as_deref().unwrap_or("image/png");
            if mime.starts_with("image/") && !d.data.is_empty() {
                Some(format!("data:{mime};base64,{}", d.data))
            } else {
                None
            }
        })
}

fn parse_usage(usage: Option<UsageWire>) -> Option<TokenUsage> {
    usage.map(|u| TokenUsage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.candidates_tokens,
        total_tokens: u.total_tokens,
    })
}

#[async_trait]
impl Provider for GoogleProvider {
    async fn list_models(&self, base_url: &str, key: &str) -> Result<Vec<Model>, String> {
        let url = format!(
            "{}/v1beta/models?key={}",
            base_url.trim_end_matches('/'),
            key
        );
        let resp = reqwest::Client::new()
            .get(&url)
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
            .models
            .into_iter()
            .filter(|m| m.supported_methods.contains(&"generateContent".to_string()))
            .map(|m| {
                let id = m
                    .name
                    .strip_prefix("models/")
                    .unwrap_or(&m.name)
                    .to_string();
                Model {
                    id,
                    name: m.display_name,
                }
            })
            .collect())
    }

    async fn complete(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
    ) -> Result<CompletionResponse, String> {
        let url = format!(
            "{}?key={}",
            model_endpoint(base_url, &req.model, "generateContent"),
            key
        );
        let body = build_body(&req);
        let resp = reqwest::Client::new()
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: GenerateResp =
            serde_json::from_str(&text).map_err(|e| format!("parse response: {e}; body={text}"))?;
        Ok(CompletionResponse {
            content: extract_text(&parsed),
            image_url: extract_image_url(&parsed),
            usage: parse_usage(parsed.usage),
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
        // Google SSE streaming with alt=sse — each event data is a full GenerateContentResponse
        let url = format!(
            "{}?key={}&alt=sse",
            model_endpoint(base_url, &req.model, "streamGenerateContent"),
            key
        );
        let fallback_req = req.clone();
        let body = build_body(&req);
        let http = reqwest::Client::new().post(&url).json(&body);

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

        let mut last_usage: Option<TokenUsage> = None;
        let mut stream = resp.bytes_stream().eventsource();

        loop {
            tokio::select! {
                biased;
                _ = cancel.notified() => { break; }
                ev = stream.next() => {
                    let Some(event) = ev else { break };
                    let event = match event {
                        Ok(event) => event,
                        Err(_) => {
                            let fallback = self
                                .complete(base_url, key, fallback_req)
                                .await
                                .map_err(|e| format!("stream fallback failed: {e}"))?;
                            if !fallback.content.is_empty() {
                                cb(StreamItem::Chunk(fallback.content));
                            }
                            cb(StreamItem::Done { usage: fallback.usage });
                            return Ok(());
                        }
                    };
                    if event.data.is_empty() { continue; }
                    let chunk: GenerateResp = match serde_json::from_str(&event.data) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let text = extract_text(&chunk);
                    if !text.is_empty() {
                        cb(StreamItem::Chunk(text));
                    }
                    if let Some(u) = parse_usage(chunk.usage) {
                        last_usage = Some(u);
                    }
                }
            }
        }

        cb(StreamItem::Done { usage: last_usage });
        Ok(())
    }
}
