use crate::storage::{chats_dir, json_store};
use crate::types::{Chat, ChatMeta, Message, Role};
use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

fn chat_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("invalid chat id {id}"));
    }
    Ok(chats_dir(app)?.join(format!("{id}.json")))
}

#[tauri::command]
pub fn list_chats(app: AppHandle) -> Result<Vec<ChatMeta>, String> {
    let dir = chats_dir(&app)?;
    let mut metas: Vec<ChatMeta> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read_dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match json_store::read::<Chat>(&path) {
            Ok(c) if !c.id.is_empty() => metas.push(ChatMeta {
                id: c.id,
                title: c.title,
                pinned: c.pinned,
                created_at: c.created_at,
                updated_at: c.updated_at,
            }),
            _ => continue,
        }
    }
    metas.sort_by(|a, b| {
        if a.pinned != b.pinned {
            return b.pinned.cmp(&a.pinned);
        }
        b.updated_at.cmp(&a.updated_at)
    });
    Ok(metas)
}

#[tauri::command]
pub fn load_chat(app: AppHandle, id: String) -> Result<Chat, String> {
    let path = chat_path(&app, &id)?;
    if !path.exists() {
        return Err(format!("chat {id} not found"));
    }
    json_store::read(&path)
}

#[tauri::command]
pub fn save_chat(app: AppHandle, chat: Chat) -> Result<Chat, String> {
    let mut chat = chat;
    chat.updated_at = Utc::now();
    let path = chat_path(&app, &chat.id)?;
    json_store::write_atomic(&path, &chat)?;
    Ok(chat)
}

#[derive(Debug, Deserialize)]
pub struct NewChatInput {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub proxy_id: Option<String>,
}

#[tauri::command]
pub fn create_chat(app: AppHandle, input: NewChatInput) -> Result<Chat, String> {
    let now = Utc::now();
    let chat = Chat {
        id: Uuid::new_v4().to_string(),
        title: input.title.unwrap_or_else(|| "Новый чат".to_string()),
        pinned: false,
        created_at: now,
        updated_at: now,
        model: input.model,
        proxy_id: input.proxy_id,
        summary: None,
        messages: Vec::new(),
    };
    let path = chat_path(&app, &chat.id)?;
    json_store::write_atomic(&path, &chat)?;
    Ok(chat)
}

#[tauri::command]
pub fn delete_chat(app: AppHandle, id: String) -> Result<(), String> {
    let path = chat_path(&app, &id)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn rename_chat(app: AppHandle, id: String, title: String) -> Result<ChatMeta, String> {
    let path = chat_path(&app, &id)?;
    let mut chat: Chat = json_store::read(&path)?;
    chat.title = title;
    chat.updated_at = Utc::now();
    json_store::write_atomic(&path, &chat)?;
    Ok(ChatMeta {
        id: chat.id,
        title: chat.title,
        pinned: chat.pinned,
        created_at: chat.created_at,
        updated_at: chat.updated_at,
    })
}

#[tauri::command]
pub fn pin_chat(app: AppHandle, id: String, pinned: bool) -> Result<ChatMeta, String> {
    let path = chat_path(&app, &id)?;
    let mut chat: Chat = json_store::read(&path)?;
    chat.pinned = pinned;
    chat.updated_at = Utc::now();
    json_store::write_atomic(&path, &chat)?;
    Ok(ChatMeta {
        id: chat.id,
        title: chat.title,
        pinned: chat.pinned,
        created_at: chat.created_at,
        updated_at: chat.updated_at,
    })
}

#[tauri::command]
pub fn fork_chat(app: AppHandle, id: String, until_message_id: String) -> Result<Chat, String> {
    let src: Chat = json_store::read(&chat_path(&app, &id)?)?;
    let idx = src
        .messages
        .iter()
        .position(|m| m.id == until_message_id)
        .ok_or_else(|| format!("message {until_message_id} not found"))?;
    let now = Utc::now();
    let mut new_messages: Vec<Message> = Vec::new();
    let summary_after_message_id = src.summary.as_ref().map(|s| s.after_message_id.clone());
    let mut new_summary_after_message_id: Option<String> = None;
    for m in src.messages.iter().take(idx + 1) {
        let new_id = Uuid::new_v4().to_string();
        if summary_after_message_id.as_ref() == Some(&m.id) {
            new_summary_after_message_id = Some(new_id.clone());
        }
        new_messages.push(Message {
            id: new_id,
            role: m.role.clone(),
            content: m.content.clone(),
            created_at: m.created_at,
            model: m.model.clone(),
            variations: m.variations.clone(),
            variation_index: m.variation_index,
            variation_meta: m.variation_meta.clone(),
            image_url: m.image_url.clone(),
            attachments: m.attachments.clone(),
        });
    }
    let summary = src.summary.and_then(|mut s| {
        if let Some(after_message_id) = new_summary_after_message_id {
            s.after_message_id = after_message_id;
            Some(s)
        } else {
            None
        }
    });
    let _ = Role::System; // suppress unused if branches change
    let new_chat = Chat {
        id: Uuid::new_v4().to_string(),
        title: format!("⑂ {}", src.title),
        pinned: false,
        created_at: now,
        updated_at: now,
        model: src.model,
        proxy_id: src.proxy_id,
        summary,
        messages: new_messages,
    };
    json_store::write_atomic(&chat_path(&app, &new_chat.id)?, &new_chat)?;
    Ok(new_chat)
}
