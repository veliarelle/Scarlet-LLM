use crate::providers::{provider_for, CompletionRequest, StreamItem};
use crate::types::{Attachment, ChatMessage, CompletionResponse, Model, Proxy, Role, TokenUsage};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tokio::sync::Notify;

pub struct StreamState {
    pub cancels: Mutex<HashMap<String, Arc<Notify>>>,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            cancels: Mutex::new(HashMap::new()),
        }
    }
}

fn load_proxy(app: &AppHandle, id: &str) -> Result<Proxy, String> {
    crate::commands::proxies::find_private(app, id)
}

#[tauri::command]
pub async fn list_models(app: AppHandle, proxy_id: String) -> Result<Vec<Model>, String> {
    let proxy = load_proxy(&app, &proxy_id)?;
    let provider = provider_for(&proxy.kind);
    provider.list_models(&proxy.base_url, &proxy.key).await
}

#[derive(Debug, Deserialize)]
pub struct SendCompletionInput {
    pub proxy_id: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub web_search: bool,
    #[serde(default)]
    pub prompt_caching: bool,
}

#[tauri::command]
pub async fn send_completion(
    app: AppHandle,
    input: SendCompletionInput,
) -> Result<CompletionResponse, String> {
    let proxy = load_proxy(&app, &input.proxy_id)?;
    let provider = provider_for(&proxy.kind);
    let req = CompletionRequest {
        model: input.model,
        messages: input.messages,
        params: input.params,
        tools: input.tools,
        web_search: input.web_search,
        prompt_caching: input.prompt_caching,
    };
    provider.complete(&proxy.base_url, &proxy.key, req).await
}

#[tauri::command]
pub async fn send_completion_cancellable(
    app: AppHandle,
    state: State<'_, StreamState>,
    input: SendCompletionInput,
    request_id: String,
) -> Result<CompletionResponse, String> {
    let proxy = load_proxy(&app, &input.proxy_id)?;
    let provider = provider_for(&proxy.kind);
    let req = CompletionRequest {
        model: input.model,
        messages: input.messages,
        params: input.params,
        tools: input.tools,
        web_search: input.web_search,
        prompt_caching: input.prompt_caching,
    };

    let cancel = Arc::new(Notify::new());
    {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.insert(request_id.clone(), cancel.clone());
    }

    let result = tokio::select! {
        biased;
        _ = cancel.notified() => Err("generation cancelled".to_string()),
        r = provider.complete(&proxy.base_url, &proxy.key, req) => r,
    };

    {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.remove(&request_id);
    }

    result
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Chunk { content: String },
    Done { usage: Option<TokenUsage> },
    Error { message: String },
}

#[tauri::command]
pub async fn stream_completion(
    app: AppHandle,
    state: State<'_, StreamState>,
    input: SendCompletionInput,
    stream_id: String,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let proxy = load_proxy(&app, &input.proxy_id)?;
    let provider = provider_for(&proxy.kind);
    let req = CompletionRequest {
        model: input.model,
        messages: input.messages,
        params: input.params,
        tools: input.tools,
        web_search: input.web_search,
        prompt_caching: input.prompt_caching,
    };

    let cancel = Arc::new(Notify::new());
    {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.insert(stream_id.clone(), cancel.clone());
    }

    let ch = on_event.clone();
    let cb: crate::providers::StreamCallback = Box::new(move |item| {
        let event = match item {
            StreamItem::Chunk(s) => StreamEvent::Chunk { content: s },
            StreamItem::Done { usage } => StreamEvent::Done { usage },
        };
        let _ = ch.send(event);
    });

    let result = provider
        .complete_stream(&proxy.base_url, &proxy.key, req, cb, cancel.clone())
        .await;

    {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.remove(&stream_id);
    }

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = on_event.send(StreamEvent::Error { message: e.clone() });
            Err(e)
        }
    }
}

#[tauri::command]
pub fn cancel_stream(state: State<'_, StreamState>, stream_id: String) -> Result<(), String> {
    let map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
    if let Some(c) = map.get(&stream_id) {
        c.notify_waiters();
    }
    Ok(())
}

// ---------- Custom tools ----------

#[derive(Debug, Deserialize)]
pub struct ExecuteToolInput {
    pub definition: Value,
    pub arguments: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolExecutionResponse {
    pub content: String,
}

fn template_arg(args: &Value, key: &str) -> String {
    let Some(value) = args.get(key) else {
        return String::new();
    };
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn render_template_string(input: &str, args: &Value) -> String {
    let mut out = input.replace("{{json}}", &args.to_string());
    if let Some(obj) = args.as_object() {
        for key in obj.keys() {
            out = out.replace(&format!("{{{{{key}}}}}"), &template_arg(args, key));
        }
    }
    out
}

fn render_template_value(value: &Value, args: &Value) -> Value {
    match value {
        Value::String(s) => Value::String(render_template_string(s, args)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| render_template_value(item, args))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), render_template_value(v, args)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn clipped_tool_body(text: String) -> String {
    const MAX: usize = 20_000;
    if text.chars().count() <= MAX {
        text
    } else {
        let clipped: String = text.chars().take(MAX).collect();
        format!("{clipped}\n\n[tool output truncated]")
    }
}

#[tauri::command]
pub async fn execute_tool(input: ExecuteToolInput) -> Result<ToolExecutionResponse, String> {
    let executor = input
        .definition
        .get("executor")
        .and_then(Value::as_object)
        .ok_or("tool has no executor")?;
    let executor_type = executor
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("http");
    if executor_type != "http" {
        return Err(format!("unsupported executor type: {executor_type}"));
    }

    let raw_url = executor
        .get("url")
        .and_then(Value::as_str)
        .ok_or("HTTP tool executor requires url")?;
    let url = render_template_string(raw_url, &input.arguments);
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("HTTP tool url must start with http:// or https://".to_string());
    }

    let method = executor
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("GET")
        .to_uppercase();
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| format!("invalid HTTP method: {e}"))?;
    let timeout_ms = executor
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(30_000)
        .clamp(1_000, 120_000);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| format!("cannot build HTTP client: {e}"))?;

    let mut req = client.request(method.clone(), url);
    if let Some(headers) = executor.get("headers").and_then(Value::as_object) {
        for (key, value) in headers {
            if let Some(v) = value.as_str() {
                req = req.header(key, render_template_string(v, &input.arguments));
            }
        }
    }

    if let Some(body) = executor.get("body") {
        let rendered = render_template_value(body, &input.arguments);
        if let Some(text) = rendered.as_str() {
            req = req.body(text.to_string());
        } else {
            req = req.json(&rendered);
        }
    } else if method != reqwest::Method::GET && method != reqwest::Method::HEAD {
        req = req.json(&input.arguments);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("tool HTTP request failed: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("tool HTTP response read failed: {e}"))?;
    Ok(ToolExecutionResponse {
        content: clipped_tool_body(format!("HTTP {status}\n{text}")),
    })
}

// ---------- Image generation ----------

#[derive(Debug, Deserialize)]
pub struct ImageGenInput {
    pub proxy_id: String,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub image_id: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ImageGenResponse {
    pub url: String,
}

#[derive(Deserialize)]
struct ImgApiResponse {
    data: Vec<ImgDataItem>,
}

#[derive(Deserialize)]
struct ImgDataItem {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    b64_json: Option<String>,
}

fn extract_error_from_body(text: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(text).ok()?;
    let err = val.get("error")?;
    let msg = err
        .get("message")
        .and_then(|m| m.as_str())
        .or_else(|| err.as_str())
        .unwrap_or("unknown API error");
    Some(msg.to_string())
}

fn build_google_predict_image_params(
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    const ALLOWED_DIRECT: &[&str] = &[
        "sampleCount",
        "seed",
        "enhancePrompt",
        "negativePrompt",
        "aspectRatio",
        "personGeneration",
        "language",
        "safetySetting",
        "addWatermark",
        "outputOptions",
    ];
    const STRIP_TEXT_ONLY: &[&str] = &[
        "max_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "topP",
        "top_k",
        "topK",
        "frequency_penalty",
        "presence_penalty",
        "logprobs",
        "top_logprobs",
        "stream",
        "stop",
        "stream_options",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
    ];

    let mut out = serde_json::Map::new();
    out.insert("sampleCount".into(), json!(1));
    out.insert("aspectRatio".into(), json!("1:1"));
    out.insert("personGeneration".into(), json!("allow_adult"));
    out.insert("safetySetting".into(), json!("block_low_and_above"));
    out.insert(
        "outputOptions".into(),
        json!({ "mimeType": "image/jpeg", "compressionQuality": 100 }),
    );

    for (key, value) in params {
        let mapped = match key.as_str() {
            "sample_count" | "n" => Some("sampleCount"),
            "enhance_prompt" | "enhance" => Some("enhancePrompt"),
            "negative_prompt" => Some("negativePrompt"),
            "aspect_ratio" | "size" => Some("aspectRatio"),
            "person_generation" => Some("personGeneration"),
            "safety_setting" => Some("safetySetting"),
            "add_watermark" => Some("addWatermark"),
            "output_options" => Some("outputOptions"),
            k if ALLOWED_DIRECT.contains(&k) => Some(k),
            k if STRIP_TEXT_ONLY.contains(&k) => None,
            _ => None,
        };

        if let Some(mapped_key) = mapped {
            out.insert(mapped_key.to_string(), value.clone());
        }
    }

    out
}

fn build_image_content(prompt: &str, attachments: &[Attachment]) -> serde_json::Value {
    let mut parts = Vec::new();
    if !prompt.trim().is_empty() {
        parts.push(json!({ "type": "text", "text": prompt }));
    } else {
        parts.push(json!({ "type": "text", "text": "Use the attached image as reference." }));
    }

    for att in attachments {
        if att.mime_type.starts_with("image/") {
            parts.push(json!({
                "type": "image_url",
                "image_url": { "url": format!("data:{};base64,{}", att.mime_type, att.data) },
            }));
        } else if let Some(text) = att.text.as_ref().filter(|s| !s.trim().is_empty()) {
            parts.push(json!({
                "type": "text",
                "text": format!("[Attached file: {}, {}]\n\n{}", att.name, att.mime_type, text.trim()),
            }));
        } else {
            parts.push(json!({
                "type": "file",
                "source": { "type": "base64", "media_type": att.mime_type, "data": att.data },
                "name": att.name,
            }));
        }
    }

    serde_json::Value::Array(parts)
}

fn image_response_url_from_text(text: &str) -> Result<String, String> {
    let parsed: ImgApiResponse =
        serde_json::from_str(text).map_err(|e| format!("parse response: {e}; body={text}"))?;

    let item = parsed
        .data
        .into_iter()
        .next()
        .ok_or_else(|| "no image in response".to_string())?;

    if let Some(b64) = item.b64_json.filter(|s| !s.is_empty()) {
        Ok(format!("data:image/png;base64,{b64}"))
    } else if let Some(u) = item.url.filter(|s| !s.is_empty()) {
        Ok(u)
    } else {
        Err("no url or b64_json in response".to_string())
    }
}

fn image_endpoint(base_url: &str, method: &str) -> String {
    let base = base_url.trim_end_matches('/');
    if let Some(root) = base.strip_suffix("/v1beta") {
        format!("{root}/v1/images/{method}")
    } else if base.ends_with("/v1") {
        format!("{base}/images/{method}")
    } else {
        format!("{base}/v1/images/{method}")
    }
}

fn image_prompt_with_file_context(prompt: &str, attachments: &[Attachment]) -> String {
    let prompt = if prompt.trim().is_empty() {
        "Use the attached image as reference.".to_string()
    } else {
        prompt.to_string()
    };
    let file_context = attachments
        .iter()
        .filter(|att| !att.mime_type.starts_with("image/"))
        .filter_map(|att| {
            att.text
                .as_ref()
                .filter(|text| !text.trim().is_empty())
                .map(|text| {
                    format!(
                        "[Attached file: {}, {}]\n\n{}",
                        att.name,
                        att.mime_type,
                        text.trim()
                    )
                })
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    if file_context.is_empty() {
        prompt
    } else {
        format!("{prompt}\n\n{file_context}")
    }
}

async fn generate_openai_image(
    client: &reqwest::Client,
    cancel: Arc<Notify>,
    base_url: &str,
    key: &str,
    model: &str,
    prompt: &str,
    attachments: &[Attachment],
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<ImageGenResponse, String> {
    let base = base_url.trim_end_matches('/');
    let use_openai_array_field = base.contains("api.openai.com");
    let method = if attachments.is_empty() {
        "generations"
    } else {
        "edits"
    };
    let endpoint = image_endpoint(base, method);

    let prompt = if prompt.trim().is_empty() {
        "Use the attached image as reference."
    } else {
        prompt
    };

    let resp = if attachments.is_empty() {
        let body = build_openai_image_json_body(model, prompt, params);
        let mut req = client.post(&endpoint).json(&body);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }

        tokio::select! {
            biased;
            _ = cancel.notified() => return Err("generation cancelled".to_string()),
            r = req.send() => r.map_err(|e| format!("image request failed: {e}"))?,
        }
    } else {
        let mut form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("prompt", prompt.to_string());

        const IMG_STRIP: &[&str] = &[
            "max_tokens",
            "max_completion_tokens",
            "temperature",
            "top_p",
            "frequency_penalty",
            "presence_penalty",
            "logprobs",
            "top_logprobs",
            "stream",
            "stop",
            "stream_options",
        ];
        for (k, v) in params {
            if IMG_STRIP.contains(&k.as_str()) {
                continue;
            }
            if model == "gpt-image-2" && k == "input_fidelity" {
                continue;
            }
            form = form.text(
                k.clone(),
                v.as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| v.to_string()),
            );
        }

        let image_field = if use_openai_array_field {
            "image[]"
        } else {
            "image"
        };
        for att in attachments
            .iter()
            .filter(|a| a.mime_type.starts_with("image/"))
        {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&att.data)
                .map_err(|e| format!("attachment base64 decode: {e}"))?;
            let part = reqwest::multipart::Part::bytes(bytes)
                .file_name(att.name.clone())
                .mime_str(&att.mime_type)
                .map_err(|e| format!("attachment mime: {e}"))?;
            form = form.part(image_field, part);
        }

        let mut req = client.post(&endpoint).multipart(form);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }

        tokio::select! {
            biased;
            _ = cancel.notified() => return Err("generation cancelled".to_string()),
            r = req.send() => r.map_err(|e| format!("image edit request failed: {e}"))?,
        }
    };

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read image response body: {e}"))?;

    if !status.is_success() {
        if let Some(msg) = extract_error_from_body(&text) {
            return Err(format!("API error: {msg}"));
        }
        return Err(format!("HTTP {status}: {text}"));
    }
    if let Some(msg) = extract_error_from_body(&text) {
        return Err(format!("API error: {msg}"));
    }

    Ok(ImageGenResponse {
        url: image_response_url_from_text(&text)?,
    })
}

async fn generate_xai_image(
    client: &reqwest::Client,
    cancel: Arc<Notify>,
    base_url: &str,
    key: &str,
    model: &str,
    prompt: &str,
    attachments: &[Attachment],
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<ImageGenResponse, String> {
    const XAI_IMG_STRIP: &[&str] = &[
        "max_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "frequency_penalty",
        "presence_penalty",
        "logprobs",
        "top_logprobs",
        "stream",
        "stop",
        "stream_options",
        "quality",
        "size",
        "style",
    ];

    let image = attachments.iter().find(|att| att.mime_type.starts_with("image/"));
    let method = if image.is_some() { "edits" } else { "generations" };
    let endpoint = image_endpoint(base_url, method);
    let prompt = image_prompt_with_file_context(prompt, attachments);
    let mut body: serde_json::Map<String, serde_json::Value> = params
        .iter()
        .filter(|(k, _)| !XAI_IMG_STRIP.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    body.insert("model".into(), json!(model));
    body.insert("prompt".into(), json!(prompt));

    if let Some(att) = image {
        body.insert(
            "image".into(),
            json!({
                "type": "image_url",
                "url": format!("data:{};base64,{}", att.mime_type, att.data),
            }),
        );
    }

    let mut req = client.post(&endpoint).json(&body);
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }

    let resp = tokio::select! {
        biased;
        _ = cancel.notified() => return Err("generation cancelled".to_string()),
        r = req.send() => r.map_err(|e| format!("xAI image request failed: {e}"))?,
    };
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read xAI image response body: {e}"))?;

    if !status.is_success() {
        if let Some(msg) = extract_error_from_body(&text) {
            return Err(format!("API error: {msg}"));
        }
        return Err(format!("HTTP {status}: {text}"));
    }
    if let Some(msg) = extract_error_from_body(&text) {
        return Err(format!("API error: {msg}"));
    }

    Ok(ImageGenResponse {
        url: image_response_url_from_text(&text)?,
    })
}

async fn generate_glm_image(
    client: &reqwest::Client,
    cancel: Arc<Notify>,
    base_url: &str,
    key: &str,
    model: &str,
    prompt: &str,
    attachments: &[Attachment],
    params: &serde_json::Map<String, serde_json::Value>,
) -> Result<ImageGenResponse, String> {
    if attachments.iter().any(|att| att.mime_type.starts_with("image/")) {
        return Err(
            "Image references/edits are not supported by the GLM image API. Use GLM vision models in chat mode for image understanding."
                .to_string(),
        );
    }

    const GLM_IMG_STRIP: &[&str] = &[
        "max_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "frequency_penalty",
        "presence_penalty",
        "logprobs",
        "top_logprobs",
        "stream",
        "stop",
        "stream_options",
        "style",
        "background",
        "response_format",
    ];

    let endpoint = format!("{}/images/generations", base_url.trim_end_matches('/'));
    let prompt = image_prompt_with_file_context(prompt, attachments);
    let mut body: serde_json::Map<String, serde_json::Value> = params
        .iter()
        .filter(|(k, _)| !GLM_IMG_STRIP.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    body.insert("model".into(), json!(model));
    body.insert("prompt".into(), json!(prompt));

    let mut req = client.post(&endpoint).json(&body);
    if !key.is_empty() {
        req = req.bearer_auth(key);
    }

    let resp = tokio::select! {
        biased;
        _ = cancel.notified() => return Err("generation cancelled".to_string()),
        r = req.send() => r.map_err(|e| format!("GLM image request failed: {e}"))?,
    };
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("read GLM image response body: {e}"))?;

    if !status.is_success() {
        if let Some(msg) = extract_error_from_body(&text) {
            return Err(format!("API error: {msg}"));
        }
        return Err(format!("HTTP {status}: {text}"));
    }
    if let Some(msg) = extract_error_from_body(&text) {
        return Err(format!("API error: {msg}"));
    }

    Ok(ImageGenResponse {
        url: image_response_url_from_text(&text)?,
    })
}

fn build_openai_image_json_body(
    model: &str,
    prompt: &str,
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    const IMG_STRIP: &[&str] = &[
        "max_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "frequency_penalty",
        "presence_penalty",
        "logprobs",
        "top_logprobs",
        "stream",
        "stop",
        "stream_options",
    ];
    let mut body: serde_json::Map<String, serde_json::Value> = params
        .iter()
        .filter(|(k, _)| !IMG_STRIP.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    body.insert("model".into(), json!(model));
    body.insert("prompt".into(), json!(prompt));
    body
}

fn build_openrouter_image_params(
    model: &str,
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    const STRIP: &[&str] = &[
        "stream",
        "stream_options",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "max_tokens",
        "max_completion_tokens",
    ];
    let mut body: serde_json::Map<String, serde_json::Value> = params
        .iter()
        .filter(|(k, _)| !STRIP.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let model_lc = model.to_lowercase();
    let modalities = if model_lc.contains("gemini") || model_lc.starts_with("google/") {
        json!(["image", "text"])
    } else {
        json!(["image"])
    };
    body.insert("modalities".into(), modalities);
    body
}

async fn generate_openrouter_image(
    cancel: Arc<Notify>,
    base_url: &str,
    key: &str,
    model: String,
    prompt: &str,
    attachments: &[Attachment],
    params: serde_json::Map<String, serde_json::Value>,
) -> Result<ImageGenResponse, String> {
    let prompt = if prompt.trim().is_empty() {
        "Use the attached image as reference."
    } else {
        prompt
    };
    let provider = provider_for(&crate::types::ProxyKind::OpenRouter);
    let task = provider.complete(
        base_url,
        key,
        CompletionRequest {
            params: build_openrouter_image_params(&model, &params),
            model,
            messages: vec![ChatMessage {
                role: Role::User,
                content: build_image_content(prompt, attachments),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }],
            tools: Vec::new(),
            web_search: false,
            prompt_caching: false,
        },
    );
    let resp = tokio::select! {
        biased;
        _ = cancel.notified() => return Err("generation cancelled".to_string()),
        r = task => r?,
    };
    let url = resp
        .image_url
        .ok_or_else(|| format!("no image in OpenRouter response; body={}", resp.content))?;
    Ok(ImageGenResponse { url })
}

#[tauri::command]
pub async fn generate_image(
    app: AppHandle,
    state: State<'_, StreamState>,
    input: ImageGenInput,
) -> Result<ImageGenResponse, String> {
    use crate::types::ProxyKind;

    let proxy = load_proxy(&app, &input.proxy_id)?;
    // Image generation can take 60-180s; use a generous timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default();
    let cancel = Arc::new(Notify::new());
    if let Some(id) = input.image_id.as_ref() {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.insert(id.clone(), cancel.clone());
    }

    let result = match proxy.kind {
        ProxyKind::GoogleNative => {
            let base = proxy.base_url.trim_end_matches('/');
            let model_lc = input.model.to_lowercase();
            if model_lc.contains("gemini") && model_lc.contains("image") {
                let provider = provider_for(&proxy.kind);
                let task = provider.complete(
                    &proxy.base_url,
                    &proxy.key,
                    CompletionRequest {
                        model: input.model,
                        messages: vec![ChatMessage {
                            role: Role::User,
                            content: build_image_content(&input.prompt, &input.attachments),
                            name: None,
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                        }],
                        params: input.params,
                        tools: Vec::new(),
                        web_search: false,
                        prompt_caching: false,
                    },
                );
                let resp = tokio::select! {
                    biased;
                    _ = cancel.notified() => return Err("generation cancelled".to_string()),
                    r = task => r?,
                };
                let url = resp
                    .image_url
                    .ok_or_else(|| format!("no image in response; body={}", resp.content))?;
                Ok(ImageGenResponse { url })
            } else {
                let endpoint = format!("{}/v1beta/models/{}:predict", base, input.model);
                let body = json!({
                    "instances": [{ "prompt": input.prompt }],
                    "parameters": build_google_predict_image_params(&input.params),
                });

                let mut req = client.post(&endpoint).json(&body);
                if !proxy.key.is_empty() {
                    req = req.query(&[("key", &proxy.key)]);
                }

                let resp = tokio::select! {
                    biased;
                    _ = cancel.notified() => return Err("generation cancelled".to_string()),
                    r = req.send() => r.map_err(|e| format!("request failed: {e}"))?,
                };
                let status = resp.status();
                let text = resp.text().await.map_err(|e| format!("read body: {e}"))?;
                if !status.is_success() {
                    if let Some(msg) = extract_error_from_body(&text) {
                        return Err(format!("API error: {msg}"));
                    }
                    return Err(format!("HTTP {status}: {text}"));
                }
                if let Some(msg) = extract_error_from_body(&text) {
                    return Err(format!("API error: {msg}"));
                }

                let val: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("parse response: {e}"))?;

                let gemini_b64 = val
                    .get("candidates")
                    .and_then(|a| a.get(0))
                    .and_then(|c| c.get("content"))
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.as_array())
                    .and_then(|parts| {
                        parts.iter().find_map(|part| {
                            part.get("inlineData")
                                .or_else(|| part.get("inline_data"))
                                .and_then(|d| d.get("data"))
                                .and_then(|d| d.as_str())
                        })
                    });

                let imagen_predict_b64 = val
                    .get("predictions")
                    .and_then(|a| a.get(0))
                    .and_then(|p| p.get("bytesBase64Encoded").or_else(|| p.get("imageBytes")))
                    .and_then(|b| b.as_str());

                let imagen_legacy_b64 = val
                    .get("generatedImages")
                    .and_then(|a| a.get(0))
                    .and_then(|item| item.get("image"))
                    .and_then(|img| img.get("imageBytes"))
                    .and_then(|b| b.as_str());

                let Some(b64) = gemini_b64.or(imagen_predict_b64).or(imagen_legacy_b64) else {
                    return Err(format!("no image in response; body={text}"));
                };

                Ok(ImageGenResponse {
                    url: format!("data:image/png;base64,{b64}"),
                })
            }
        }

        ProxyKind::AnthropicNative => {
            Err("Image generation is not supported for Anthropic".to_string())
        }

        ProxyKind::TextCompletions => {
            Err("Image generation is not supported for Text completions".to_string())
        }

        ProxyKind::OpenRouter => {
            generate_openrouter_image(
                cancel.clone(),
                &proxy.base_url,
                &proxy.key,
                input.model,
                &input.prompt,
                &input.attachments,
                input.params,
            )
            .await
        }

        ProxyKind::Grok => {
            generate_xai_image(
                &client,
                cancel.clone(),
                &proxy.base_url,
                &proxy.key,
                &input.model,
                &input.prompt,
                &input.attachments,
                &input.params,
            )
            .await
        }

        ProxyKind::Glm => {
            generate_glm_image(
                &client,
                cancel.clone(),
                &proxy.base_url,
                &proxy.key,
                &input.model,
                &input.prompt,
                &input.attachments,
                &input.params,
            )
            .await
        }

        ProxyKind::Deepseek => {
            Err("Image generation is not supported by the official DeepSeek API".to_string())
        }

        ProxyKind::Mistral => Err(
            "Mistral image generation uses the Mistral Agents/Conversations image_generation tool and is not supported by Scarlet image mode yet"
                .to_string(),
        ),

        ProxyKind::Moonshot => Err(
            "Image generation is not supported by the official Moonshot/Kimi API; use Kimi vision models in chat mode for image understanding"
                .to_string(),
        ),

        _ => {
            generate_openai_image(
                &client,
                cancel.clone(),
                &proxy.base_url,
                &proxy.key,
                &input.model,
                &input.prompt,
                &input.attachments,
                &input.params,
            )
            .await
        }
    };

    if let Some(id) = input.image_id.as_ref() {
        let mut map = state.cancels.lock().map_err(|e| format!("lock: {e}"))?;
        map.remove(id);
    }

    result
}

// ---------- Save image ----------

#[tauri::command]
pub async fn save_image(
    app: AppHandle,
    data_url: String,
    default_name: String,
    title: Option<String>,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;

    let rest = data_url.strip_prefix("data:").ok_or("invalid data URL")?;
    let (mime_part, b64) = rest.split_once(',').ok_or("invalid data URL format")?;
    let mime = mime_part.strip_suffix(";base64").unwrap_or(mime_part);
    let ext = match mime {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 decode: {e}"))?;

    let stem = default_name
        .trim_end_matches(&format!(".{ext}"))
        .to_string();

    let path = app
        .dialog()
        .file()
        .set_title(title.unwrap_or_else(|| "Сохранить изображение".to_string()))
        .set_file_name(format!("{stem}.{ext}"))
        .add_filter("Image", &[ext])
        .blocking_save_file();

    match path {
        None => Ok(false),
        Some(fp) => {
            let dest = fp.as_path().ok_or("invalid path")?.to_path_buf();
            tokio::fs::write(&dest, &bytes)
                .await
                .map_err(|e| format!("write file: {e}"))?;
            Ok(true)
        }
    }
}
