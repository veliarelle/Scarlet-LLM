use crate::providers::{provider_for, CompletionRequest, StreamItem};
use crate::storage::{json_store, proxies_path};
use crate::types::{Attachment, ChatMessage, CompletionResponse, Model, Proxy, Role, TokenUsage};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    let proxies: Vec<Proxy> = json_store::read_or_default(&proxies_path(app)?)?;
    proxies
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("proxy {id} not found"))
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
    };
    provider.complete(&proxy.base_url, &proxy.key, req).await
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
    let parsed: ImgApiResponse = serde_json::from_str(text)
        .map_err(|e| format!("parse response: {e}; body={text}"))?;

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
    let method = if attachments.is_empty() { "generations" } else { "edits" };
    let endpoint = if let Some(root) = base.strip_suffix("/v1beta") {
        format!("{root}/v1/images/{method}")
    } else if base.ends_with("/v1") {
        format!("{base}/images/{method}")
    } else {
        format!("{base}/v1/images/{method}")
    };

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
            "max_tokens", "max_completion_tokens", "temperature", "top_p",
            "frequency_penalty", "presence_penalty", "logprobs", "top_logprobs",
            "stream", "stop", "stream_options",
        ];
        for (k, v) in params {
            if IMG_STRIP.contains(&k.as_str()) {
                continue;
            }
            form = form.text(k.clone(), v.as_str().map(ToString::to_string).unwrap_or_else(|| v.to_string()));
        }

        for att in attachments.iter().filter(|a| a.mime_type.starts_with("image/")) {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&att.data)
                .map_err(|e| format!("attachment base64 decode: {e}"))?;
            let part = reqwest::multipart::Part::bytes(bytes)
                .file_name(att.name.clone())
                .mime_str(&att.mime_type)
                .map_err(|e| format!("attachment mime: {e}"))?;
            form = form.part("image", part);
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

    Ok(ImageGenResponse { url: image_response_url_from_text(&text)? })
}

fn build_openai_image_json_body(
    model: &str,
    prompt: &str,
    params: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    const IMG_STRIP: &[&str] = &[
        "max_tokens", "max_completion_tokens", "temperature", "top_p",
        "frequency_penalty", "presence_penalty", "logprobs", "top_logprobs",
        "stream", "stop", "stream_options",
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
                let task = provider
                    .complete(
                        &proxy.base_url,
                        &proxy.key,
                        CompletionRequest {
                            model: input.model,
                            messages: vec![ChatMessage {
                                role: Role::User,
                                content: build_image_content(&input.prompt, &input.attachments),
                            }],
                            params: input.params,
                            tools: Vec::new(),
                            web_search: false,
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
        .set_title("Сохранить изображение")
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
