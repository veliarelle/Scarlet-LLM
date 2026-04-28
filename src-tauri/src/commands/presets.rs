use crate::storage::{json_store, presets_dir};
use crate::types::{Preset, PresetMeta, PresetUtilities, Prompt};
use chrono::Utc;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

fn preset_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("invalid preset id {id}"));
    }
    Ok(presets_dir(app)?.join(format!("{id}.json")))
}

#[tauri::command]
pub fn list_presets(app: AppHandle) -> Result<Vec<PresetMeta>, String> {
    let dir = presets_dir(&app)?;
    let mut metas: Vec<PresetMeta> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read_dir: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match json_store::read::<Preset>(&path) {
            Ok(p) if !p.id.is_empty() => metas.push(PresetMeta {
                id: p.id,
                name: p.name,
                created_at: p.created_at,
                updated_at: p.updated_at,
            }),
            _ => continue,
        }
    }
    metas.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(metas)
}

#[tauri::command]
pub fn load_preset(app: AppHandle, id: String) -> Result<Preset, String> {
    let path = preset_path(&app, &id)?;
    if !path.exists() {
        return Err(format!("preset {id} not found"));
    }
    json_store::read(&path)
}

#[tauri::command]
pub fn save_preset(app: AppHandle, preset: Preset) -> Result<Preset, String> {
    let mut p = preset;
    p.updated_at = Utc::now();
    let path = preset_path(&app, &p.id)?;
    json_store::write_atomic(&path, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn create_preset(
    app: AppHandle,
    name: String,
    prompts: Vec<Prompt>,
    utilities: Option<PresetUtilities>,
) -> Result<Preset, String> {
    let now = Utc::now();
    let preset = Preset {
        id: Uuid::new_v4().to_string(),
        name,
        prompts,
        utilities: utilities.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };
    let path = preset_path(&app, &preset.id)?;
    json_store::write_atomic(&path, &preset)?;
    Ok(preset)
}

#[tauri::command]
pub fn delete_preset(app: AppHandle, id: String) -> Result<(), String> {
    let path = preset_path(&app, &id)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove: {e}"))?;
    }
    Ok(())
}
