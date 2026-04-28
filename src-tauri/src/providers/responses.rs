use crate::providers::{join_url, CompletionRequest, Provider, StreamCallback, StreamItem};
use crate::types::{CompletionResponse, Model, Role, TokenUsage, ToolCall};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Notify;

pub struct ResponsesProvider;

// Params that have a different name in the Responses API
const RENAME: &[(&str, &str)] = &[
    ("max_tokens", "max_output_tokens"),
    ("max_completion_tokens", "max_output_tokens"),
];

// Params not supported by the Responses API at all
const UNSUPPORTED: &[&str] = &[
    "n",
    "logprobs",
    "top_logprobs",
    "frequency_penalty",
    "presence_penalty",
    "logit_bias",
    "best_of",
    "echo",
    "suffix",
];

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
struct ResponsesResp {
    #[serde(default)]
    output: Vec<OutputItem>,
    #[serde(default)]
    usage: Option<UsageWire>,
}

#[derive(Debug, Deserialize)]
struct OutputItem {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    content: Vec<ContentPart>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    call_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UsageWire {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct StreamEvent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    delta: Option<String>,
    #[serde(default)]
    response: Option<CompletedResponse>,
}

#[derive(Debug, Deserialize)]
struct CompletedResponse {
    #[serde(default)]
    usage: Option<UsageWire>,
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
    // Separate system messages into `instructions`, rest go into `input`
    let mut instructions = String::new();
    let mut input: Vec<Value> = Vec::new();

    for msg in &req.messages {
        match msg.role {
            Role::System => {
                if !instructions.is_empty() {
                    instructions.push('\n');
                }
                if let Some(s) = msg.content.as_str() {
                    instructions.push_str(s);
                }
            }
            Role::User => {
                input.push(json!({ "role": "user", "content": msg.content }));
            }
            Role::Assistant => {
                if !msg.content.as_str().unwrap_or_default().is_empty() {
                    input.push(json!({ "role": "assistant", "content": msg.content }));
                }
                for call in &msg.tool_calls {
                    input.push(json!({
                        "type": "function_call",
                        "call_id": call.id,
                        "name": call.name,
                        "arguments": call.arguments,
                    }));
                }
            }
            Role::Tool => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": msg.tool_call_id.as_deref().unwrap_or_default(),
                    "output": msg.content.as_str().unwrap_or_default(),
                }));
            }
        }
    }

    let mut body = serde_json::Map::new();
    body.insert("model".into(), Value::String(req.model.clone()));
    body.insert("input".into(), Value::Array(input));
    if !instructions.is_empty() {
        body.insert("instructions".into(), Value::String(instructions));
    }
    if stream {
        body.insert("stream".into(), Value::Bool(true));
    }
    if req.prompt_caching && supports_openai_prompt_cache_controls(base_url) {
        body.insert(
            "prompt_cache_key".into(),
            Value::String(prompt_cache_key(&req.model)),
        );
    }

    // Merge params with renaming and exclusions
    for (k, v) in &req.params {
        if UNSUPPORTED.contains(&k.as_str()) {
            continue;
        }
        let key = RENAME
            .iter()
            .find(|(old, _)| *old == k.as_str())
            .map(|(_, new)| *new)
            .unwrap_or(k.as_str());
        body.insert(key.to_string(), v.clone());
    }

    // Tools: Responses API supports web_search_preview natively
    let mut tools_arr = req.tools.clone();
    if req.web_search {
        let has_ws = tools_arr
            .iter()
            .any(|t| t.get("type").and_then(|v| v.as_str()) == Some("web_search_preview"));
        if !has_ws {
            tools_arr.push(json!({ "type": "web_search_preview" }));
        }
    }
    // Convert OpenAI function-calling tools to Responses API format
    // The Responses API wraps function tools as {"type":"function","name":...,"description":...,"parameters":...}
    let converted: Vec<Value> = tools_arr
        .into_iter()
        .map(|t| {
            if t.get("type").and_then(|v| v.as_str()) == Some("function") {
                if let Some(func) = t.get("function") {
                    return json!({
                        "type": "function",
                        "name": func.get("name").cloned().unwrap_or(Value::Null),
                        "description": func.get("description").cloned().unwrap_or(Value::Null),
                        "parameters": func.get("parameters").cloned().unwrap_or(Value::Null),
                    });
                }
            }
            t
        })
        .collect();
    if !converted.is_empty() {
        body.insert("tools".into(), Value::Array(converted));
    }

    body
}

fn extract_text(resp: ResponsesResp) -> String {
    resp.output
        .into_iter()
        .filter(|item| item.kind == "message")
        .flat_map(|item| item.content)
        .filter(|part| part.kind == "output_text")
        .filter_map(|part| part.text)
        .collect::<Vec<_>>()
        .join("")
}

fn extract_tool_calls(resp: &ResponsesResp) -> Vec<ToolCall> {
    resp.output
        .iter()
        .filter(|item| item.kind == "function_call")
        .filter_map(|item| {
            let name = item.name.as_ref()?.clone();
            Some(ToolCall {
                id: item
                    .call_id
                    .as_ref()
                    .or(item.id.as_ref())
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                name,
                arguments: item.arguments.clone().unwrap_or_else(|| "{}".to_string()),
            })
        })
        .collect()
}

#[async_trait]
impl Provider for ResponsesProvider {
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
        let url = join_url(base_url, "responses");
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
        let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        let parsed: ResponsesResp =
            serde_json::from_str(&text).map_err(|e| format!("parse response: {e}; body={text}"))?;
        let usage = parsed.usage.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.total_tokens,
        });
        let tool_calls = extract_tool_calls(&parsed);
        let content = extract_text(parsed);
        Ok(CompletionResponse {
            content,
            usage,
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
        let url = join_url(base_url, "responses");
        let body = build_body(&req, true, base_url);
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
                _ = cancel.notified() => { break; }
                ev = stream.next() => {
                    let Some(event) = ev else { break };
                    let event = event.map_err(|e| format!("stream error: {e}"))?;
                    if event.data.is_empty() || event.data == "[DONE]" { continue; }
                    let evt: StreamEvent = match serde_json::from_str(&event.data) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    match evt.kind.as_str() {
                        "response.output_text.delta" => {
                            if let Some(delta) = evt.delta {
                                if !delta.is_empty() {
                                    cb(StreamItem::Chunk(delta));
                                }
                            }
                        }
                        "response.completed" => {
                            if let Some(completed) = evt.response {
                                usage = completed.usage.map(|u| TokenUsage {
                                    prompt_tokens: u.input_tokens,
                                    completion_tokens: u.output_tokens,
                                    total_tokens: u.total_tokens,
                                });
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        cb(StreamItem::Done { usage });
        Ok(())
    }
}
