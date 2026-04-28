use crate::storage::{agent_presets_dir, json_store};
use crate::types::{AgentDefinition, AgentPreset, AgentPresetMeta};
use chrono::Utc;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

fn agent_preset_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(format!("invalid agent preset id {id}"));
    }
    Ok(agent_presets_dir(app)?.join(format!("{id}.json")))
}

#[tauri::command]
pub fn list_agent_presets(app: AppHandle) -> Result<Vec<AgentPresetMeta>, String> {
    let dir = agent_presets_dir(&app)?;
    let mut metas: Vec<AgentPresetMeta> = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("read_dir: {e}"))? {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match json_store::read::<AgentPreset>(&path) {
            Ok(p) if !p.id.is_empty() => metas.push(AgentPresetMeta {
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
pub fn load_agent_preset(app: AppHandle, id: String) -> Result<AgentPreset, String> {
    let path = agent_preset_path(&app, &id)?;
    if !path.exists() {
        return Err(format!("agent preset {id} not found"));
    }
    json_store::read(&path)
}

#[tauri::command]
pub fn save_agent_preset(app: AppHandle, preset: AgentPreset) -> Result<AgentPreset, String> {
    let mut p = preset;
    p.updated_at = Utc::now();
    let path = agent_preset_path(&app, &p.id)?;
    json_store::write_atomic(&path, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn create_agent_preset(
    app: AppHandle,
    name: String,
    agents: Vec<AgentDefinition>,
) -> Result<AgentPreset, String> {
    let now = Utc::now();
    let preset = AgentPreset {
        id: Uuid::new_v4().to_string(),
        name,
        agents,
        created_at: now,
        updated_at: now,
    };
    let path = agent_preset_path(&app, &preset.id)?;
    json_store::write_atomic(&path, &preset)?;
    Ok(preset)
}

#[tauri::command]
pub fn delete_agent_preset(app: AppHandle, id: String) -> Result<(), String> {
    let path = agent_preset_path(&app, &id)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove: {e}"))?;
    }
    Ok(())
}
