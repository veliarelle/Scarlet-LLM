pub mod json_store;

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn config_root(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("cannot resolve app_config_dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create config dir: {e}"))?;
    Ok(dir)
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_root(app)?.join("settings.json"))
}

pub fn proxies_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_root(app)?.join("proxies.json"))
}

pub fn proxy_secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_root(app)?.join("proxy_secrets.json"))
}

#[allow(dead_code)]
pub fn presets_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = config_root(app)?.join("presets");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create presets dir: {e}"))?;
    Ok(dir)
}

#[allow(dead_code)]
pub fn agent_presets_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = config_root(app)?.join("agent_presets");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create agent presets dir: {e}"))?;
    Ok(dir)
}

#[allow(dead_code)]
pub fn chats_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = config_root(app)?.join("chats");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create chats dir: {e}"))?;
    Ok(dir)
}
