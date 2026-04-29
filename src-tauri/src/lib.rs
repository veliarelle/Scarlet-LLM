mod commands;
mod providers;
mod storage;
mod types;

use commands::llm::StreamState;
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
use tauri::Manager;

#[tauri::command]
fn ping() -> String {
    "pong from Rust".to_string()
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
))]
fn force_webview_hardware_acceleration(app: &tauri::App) {
    if let Some(webview) = app.get_webview_window("main") {
        let _ = webview.with_webview(|platform_webview| {
            use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

            if let Some(settings) = platform_webview.inner().settings() {
                settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Always);
            }
        });
    }
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
)))]
fn force_webview_hardware_acceleration(_: &tauri::App) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(StreamState::default())
        .setup(|app| {
            force_webview_hardware_acceleration(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::proxies::list_proxies,
            commands::proxies::create_proxy,
            commands::proxies::update_proxy,
            commands::proxies::delete_proxy,
            commands::llm::list_models,
            commands::llm::send_completion,
            commands::llm::send_completion_cancellable,
            commands::llm::stream_completion,
            commands::llm::cancel_stream,
            commands::llm::execute_tool,
            commands::llm::generate_image,
            commands::llm::save_image,
            commands::attachments::prepare_attachments,
            commands::attachments::read_dropped_files,
            commands::chats::list_chats,
            commands::chats::load_chat,
            commands::chats::save_chat,
            commands::chats::create_chat,
            commands::chats::delete_chat,
            commands::chats::rename_chat,
            commands::chats::pin_chat,
            commands::chats::fork_chat,
            commands::presets::list_presets,
            commands::presets::load_preset,
            commands::presets::save_preset,
            commands::presets::create_preset,
            commands::presets::delete_preset,
            commands::agent_presets::list_agent_presets,
            commands::agent_presets::load_agent_preset,
            commands::agent_presets::save_agent_preset,
            commands::agent_presets::create_agent_preset,
            commands::agent_presets::delete_agent_preset,
            commands::import_export::export_preset,
            commands::import_export::import_preset,
            commands::import_export::export_agent_preset,
            commands::import_export::import_agent_preset,
            commands::import_export::export_profile,
            commands::import_export::import_profile,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
