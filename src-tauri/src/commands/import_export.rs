use crate::storage::{json_store, presets_dir, settings_path};
use crate::types::{Preset, Settings};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct ProfileExport {
    version: u32,
    exported_at: DateTime<Utc>,
    settings: Settings,
    presets: Vec<Preset>,
}

fn preset_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("invalid preset id {id}"));
    }
    Ok(presets_dir(app)?.join(format!("{id}.json")))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && !id.contains('/') && !id.contains('\\') && !id.contains("..")
}

fn sanitize_preset(mut preset: Preset) -> Preset {
    if !valid_id(&preset.id) {
        preset.id = Uuid::new_v4().to_string();
    }
    preset
}

fn read_all_presets(app: &AppHandle) -> Result<Vec<Preset>, String> {
    let dir = presets_dir(app)?;
    let mut presets = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("read_dir: {e}"))? {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(preset) = json_store::read::<Preset>(&path) {
            if !preset.id.is_empty() {
                presets.push(preset);
            }
        }
    }
    presets.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(presets)
}

fn write_presets(app: &AppHandle, presets: Vec<Preset>) -> Result<usize, String> {
    let mut count = 0;
    for preset in presets.into_iter().map(sanitize_preset) {
        let path = preset_path(app, &preset.id)?;
        json_store::write_atomic(&path, &preset)?;
        count += 1;
    }
    Ok(count)
}

fn write_preset(app: &AppHandle, preset: Preset) -> Result<Preset, String> {
    let preset = sanitize_preset(preset);
    let path = preset_path(app, &preset.id)?;
    json_store::write_atomic(&path, &preset)?;
    Ok(preset)
}

fn file_name_safe(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').trim_matches('.');
    if trimmed.is_empty() {
        "preset".to_string()
    } else {
        trimmed.to_string()
    }
}

async fn save_json(
    app: &AppHandle,
    title: &str,
    default_name: &str,
    value: &impl Serialize,
) -> Result<bool, String> {
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name(default_name)
        .add_filter("JSON", &["json"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| format!("dialog closed: {e}"))? else {
        return Ok(false);
    };
    let dest = path.as_path().ok_or("invalid path")?.to_path_buf();
    let body = serde_json::to_vec_pretty(value).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&dest, body).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(true)
}

async fn pick_json(app: &AppHandle, title: &str) -> Result<Option<String>, String> {
    let (tx, rx) = oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .add_filter("JSON", &["json"])
        .pick_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| format!("dialog closed: {e}"))? else {
        return Ok(None);
    };
    let src = path.as_path().ok_or("invalid path")?.to_path_buf();
    std::fs::read_to_string(&src)
        .map(Some)
        .map_err(|e| format!("read {}: {e}", src.display()))
}

fn sanitize_profile_settings(mut settings: Settings) -> Settings {
    settings.active_proxy_id = None;
    settings.active_chat_id = None;
    settings
}

#[tauri::command]
pub async fn export_preset(app: AppHandle, preset_id: String) -> Result<bool, String> {
    let preset: Preset = json_store::read(&preset_path(&app, &preset_id)?)?;
    let file_name = format!("scarlet-preset-{}.json", file_name_safe(&preset.name));
    save_json(&app, "Export preset", &file_name, &preset).await
}

#[tauri::command]
pub async fn import_preset(app: AppHandle) -> Result<Option<Preset>, String> {
    let Some(text) = pick_json(&app, "Import preset").await? else {
        return Ok(None);
    };
    let preset: Preset =
        serde_json::from_str(&text).map_err(|e| format!("parse preset export: {e}"))?;
    write_preset(&app, preset).map(Some)
}

#[tauri::command]
pub async fn export_profile(app: AppHandle) -> Result<bool, String> {
    let settings: Settings = json_store::read_or_default(&settings_path(&app)?)?;
    let export = ProfileExport {
        version: 1,
        exported_at: Utc::now(),
        settings: sanitize_profile_settings(settings),
        presets: read_all_presets(&app)?,
    };
    save_json(&app, "Export profile", "scarlet-profile.json", &export).await
}

#[tauri::command]
pub async fn import_profile(app: AppHandle) -> Result<usize, String> {
    let Some(text) = pick_json(&app, "Import profile").await? else {
        return Ok(0);
    };
    let profile: ProfileExport =
        serde_json::from_str(&text).map_err(|e| format!("parse profile export: {e}"))?;
    let imported = write_presets(&app, profile.presets)?;
    json_store::write_atomic(
        &settings_path(&app)?,
        &sanitize_profile_settings(profile.settings),
    )?;
    Ok(imported)
}
