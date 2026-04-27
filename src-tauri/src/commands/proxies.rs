use crate::storage::{json_store, proxies_path};
use crate::types::{Proxy, ProxyKind};
use chrono::Utc;
use serde::Deserialize;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ProxyInput {
    pub name: String,
    pub base_url: String,
    pub key: String,
    pub kind: ProxyKind,
}

fn load(app: &AppHandle) -> Result<Vec<Proxy>, String> {
    json_store::read_or_default(&proxies_path(app)?)
}

fn store(app: &AppHandle, list: &[Proxy]) -> Result<(), String> {
    json_store::write_atomic(&proxies_path(app)?, &list)
}

#[tauri::command]
pub fn list_proxies(app: AppHandle) -> Result<Vec<Proxy>, String> {
    load(&app)
}

#[tauri::command]
pub fn create_proxy(app: AppHandle, input: ProxyInput) -> Result<Proxy, String> {
    let mut list = load(&app)?;
    let proxy = Proxy {
        id: Uuid::new_v4().to_string(),
        name: input.name,
        base_url: input.base_url,
        key: input.key,
        kind: input.kind,
        created_at: Utc::now(),
    };
    list.push(proxy.clone());
    store(&app, &list)?;
    Ok(proxy)
}

#[tauri::command]
pub fn update_proxy(app: AppHandle, id: String, input: ProxyInput) -> Result<Proxy, String> {
    let mut list = load(&app)?;
    let proxy = list
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("proxy {id} not found"))?;
    proxy.name = input.name;
    proxy.base_url = input.base_url;
    proxy.key = input.key;
    proxy.kind = input.kind;
    let updated = proxy.clone();
    store(&app, &list)?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_proxy(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load(&app)?;
    let before = list.len();
    list.retain(|p| p.id != id);
    if list.len() == before {
        return Err(format!("proxy {id} not found"));
    }
    store(&app, &list)
}
