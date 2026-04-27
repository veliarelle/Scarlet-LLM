use crate::storage::{json_store, settings_path};
use crate::types::Settings;
use tauri::AppHandle;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    json_store::read_or_default(&path)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    json_store::write_atomic(&path, &settings)?;
    Ok(settings)
}
