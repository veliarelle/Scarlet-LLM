use crate::storage::{json_store, proxies_path, proxy_secrets_path};
use crate::types::{Proxy, ProxyKind, ProxySecret, PublicProxy};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ProxyInput {
    pub name: String,
    pub base_url: String,
    pub key: String,
    pub kind: ProxyKind,
}

fn load_secrets(app: &AppHandle) -> Result<Vec<ProxySecret>, String> {
    json_store::read_or_default(&proxy_secrets_path(app)?)
}

fn store_secrets(app: &AppHandle, secrets: &[ProxySecret]) -> Result<(), String> {
    let path = proxy_secrets_path(app)?;
    json_store::write_atomic(&path, &secrets)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, permissions)
            .map_err(|e| format!("chmod {}: {e}", path.display()))?;
    }
    Ok(())
}

pub(crate) fn load_private(app: &AppHandle) -> Result<Vec<Proxy>, String> {
    let mut proxies: Vec<Proxy> = json_store::read_or_default(&proxies_path(app)?)?;
    let mut secret_map: HashMap<String, String> = load_secrets(app)?
        .into_iter()
        .map(|s| (s.proxy_id, s.key))
        .collect();
    let mut migrated = false;

    for proxy in &mut proxies {
        if !proxy.key.is_empty() {
            secret_map.insert(proxy.id.clone(), proxy.key.clone());
            proxy.key.clear();
            migrated = true;
        }
        if let Some(secret) = secret_map.get(&proxy.id) {
            proxy.key = secret.clone();
        }
    }

    if migrated {
        store_private(app, &proxies)?;
    }

    Ok(proxies)
}

fn store_private(app: &AppHandle, list: &[Proxy]) -> Result<(), String> {
    let public_list: Vec<Proxy> = list
        .iter()
        .cloned()
        .map(|mut proxy| {
            proxy.key.clear();
            proxy
        })
        .collect();
    let secrets: Vec<ProxySecret> = list
        .iter()
        .filter(|p| !p.key.is_empty())
        .map(|p| ProxySecret {
            proxy_id: p.id.clone(),
            key: p.key.clone(),
        })
        .collect();

    json_store::write_atomic(&proxies_path(app)?, &public_list)?;
    store_secrets(app, &secrets)
}

pub(crate) fn find_private(app: &AppHandle, id: &str) -> Result<Proxy, String> {
    load_private(app)?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("proxy {id} not found"))
}

#[tauri::command]
pub fn list_proxies(app: AppHandle) -> Result<Vec<PublicProxy>, String> {
    Ok(load_private(&app)?
        .into_iter()
        .map(PublicProxy::from)
        .collect())
}

#[tauri::command]
pub fn create_proxy(app: AppHandle, input: ProxyInput) -> Result<PublicProxy, String> {
    let mut list = load_private(&app)?;
    let proxy = Proxy {
        id: Uuid::new_v4().to_string(),
        name: input.name,
        base_url: input.base_url,
        key: input.key,
        kind: input.kind,
        created_at: Utc::now(),
    };
    list.push(proxy.clone());
    store_private(&app, &list)?;
    Ok(PublicProxy::from(proxy))
}

#[tauri::command]
pub fn update_proxy(app: AppHandle, id: String, input: ProxyInput) -> Result<PublicProxy, String> {
    let mut list = load_private(&app)?;
    let proxy = list
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or_else(|| format!("proxy {id} not found"))?;
    proxy.name = input.name;
    proxy.base_url = input.base_url;
    if !input.key.is_empty() {
        proxy.key = input.key;
    }
    proxy.kind = input.kind;
    let updated = proxy.clone();
    store_private(&app, &list)?;
    Ok(PublicProxy::from(updated))
}

#[tauri::command]
pub fn delete_proxy(app: AppHandle, id: String) -> Result<(), String> {
    let mut list = load_private(&app)?;
    let before = list.len();
    list.retain(|p| p.id != id);
    if list.len() == before {
        return Err(format!("proxy {id} not found"));
    }
    store_private(&app, &list)
}
