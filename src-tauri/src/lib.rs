mod commands;
mod providers;
mod storage;
mod types;

use commands::llm::StreamState;

#[tauri::command]
fn ping() -> String {
    "pong from Rust".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(StreamState::default())
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
            commands::llm::stream_completion,
            commands::llm::cancel_stream,
            commands::llm::generate_image,
            commands::llm::save_image,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
