pub mod anthropic;
pub mod google;
pub mod openai;
pub mod openrouter;
pub mod responses;

use crate::types::{ChatMessage, CompletionResponse, Model, ProxyKind, TokenUsage};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub params: serde_json::Map<String, serde_json::Value>,
    pub tools: Vec<serde_json::Value>,
    pub web_search: bool,
    pub prompt_caching: bool,
}

pub enum StreamItem {
    Chunk(String),
    Done { usage: Option<TokenUsage> },
}

pub type StreamCallback = Box<dyn Fn(StreamItem) + Send + Sync>;

#[async_trait]
pub trait Provider: Send + Sync {
    async fn list_models(&self, base_url: &str, key: &str) -> Result<Vec<Model>, String>;
    async fn complete(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
    ) -> Result<CompletionResponse, String>;
    async fn complete_stream(
        &self,
        base_url: &str,
        key: &str,
        req: CompletionRequest,
        cb: StreamCallback,
        cancel: Arc<Notify>,
    ) -> Result<(), String>;
}

pub fn provider_for(kind: &ProxyKind) -> Box<dyn Provider> {
    match kind {
        ProxyKind::OpenaiCompat => Box::new(openai::OpenAiProvider),
        ProxyKind::OpenaiResponses => Box::new(responses::ResponsesProvider),
        ProxyKind::OpenRouter => Box::new(openrouter::OpenRouterProvider),
        ProxyKind::AnthropicNative => Box::new(anthropic::AnthropicProvider),
        ProxyKind::GoogleNative => Box::new(google::GoogleProvider),
    }
}

pub fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}
