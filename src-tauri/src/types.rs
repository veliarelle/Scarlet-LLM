use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    /// Either a plain string or an OpenAI-style array of content parts
    /// (text / image_url / file). Providers convert to their own format.
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationMeta {
    #[serde(default)]
    pub model: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    /// Base64 payload without the data: URL prefix.
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub variations: Vec<String>,
    #[serde(default)]
    pub variation_index: u32,
    #[serde(default)]
    pub variation_meta: Vec<VariationMeta>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Chat {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub proxy_id: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMeta {
    pub id: String,
    pub title: String,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProxyKind {
    OpenaiCompat,
    AnthropicNative,
    GoogleNative,
    OpenaiResponses,
}

impl Default for ProxyKind {
    fn default() -> Self {
        Self::OpenaiCompat
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proxy {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub key: String,
    #[serde(default)]
    pub kind: ProxyKind,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamEntry {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_true")]
    pub send_effort: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            effort: default_effort(),
            send_effort: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub content: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Preset {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub prompts: Vec<Prompt>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetMeta {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,

    #[serde(default)]
    pub active_proxy_id: Option<String>,
    #[serde(default)]
    pub active_preset_id: Option<String>,
    #[serde(default)]
    pub active_model: Option<String>,
    #[serde(default)]
    pub active_chat_id: Option<String>,

    #[serde(default = "default_user_name")]
    pub user_name: String,
    #[serde(default = "default_assistant_name")]
    pub assistant_name: String,

    #[serde(default = "default_streaming")]
    pub streaming: bool,
    #[serde(default = "default_context_window")]
    pub context_window: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    #[serde(default = "default_params")]
    pub params: Vec<ParamEntry>,

    #[serde(default)]
    pub reasoning: ReasoningConfig,

    #[serde(default)]
    pub prompts: Vec<Prompt>,

    #[serde(default)]
    pub web_search: bool,
    #[serde(default)]
    pub agents: bool,
    #[serde(default)]
    pub tools: bool,

    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub custom_colors: std::collections::BTreeMap<String, String>,

    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,

    #[serde(default)]
    pub translucent_sidebar: bool,
    #[serde(default = "default_bar_blur")]
    pub sidebar_blur: u32,
    #[serde(default)]
    pub translucent_topbar: bool,
    #[serde(default = "default_bar_blur")]
    pub topbar_blur: u32,

    #[serde(default = "default_tool_defs")]
    pub tool_definitions: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            active_proxy_id: None,
            active_preset_id: None,
            active_model: None,
            active_chat_id: None,
            user_name: default_user_name(),
            assistant_name: default_assistant_name(),
            streaming: default_streaming(),
            context_window: default_context_window(),
            max_tokens: default_max_tokens(),
            params: default_params(),
            reasoning: ReasoningConfig::default(),
            prompts: Vec::new(),
            web_search: false,
            agents: false,
            tools: false,
            theme: default_theme(),
            custom_colors: std::collections::BTreeMap::new(),
            ui_scale: default_ui_scale(),
            translucent_sidebar: false,
            sidebar_blur: default_bar_blur(),
            translucent_topbar: false,
            topbar_blur: default_bar_blur(),
            tool_definitions: default_tool_defs(),
        }
    }
}

fn default_ui_scale() -> f32 {
    1.0
}
fn default_language() -> String {
    "ru".to_string()
}
fn default_bar_blur() -> u32 {
    8
}

fn default_user_name() -> String {
    "User".to_string()
}
fn default_assistant_name() -> String {
    "Scarlet".to_string()
}
fn default_streaming() -> bool {
    true
}
fn default_context_window() -> u32 {
    8192
}
fn default_max_tokens() -> u32 {
    2048
}
fn default_effort() -> String {
    "medium".to_string()
}
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_tool_defs() -> String {
    "[]".to_string()
}
fn default_params() -> Vec<ParamEntry> {
    vec![
        ParamEntry {
            id: "tp".to_string(),
            key: "temperature".to_string(),
            value: "0.7".to_string(),
            enabled: true,
        },
        ParamEntry {
            id: "pp".to_string(),
            key: "top_p".to_string(),
            value: "0.9".to_string(),
            enabled: true,
        },
    ]
}
