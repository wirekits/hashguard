// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .setup(|app| {
            let backdrop = commands::apply_backdrop(app.handle());
            app.manage(commands::Backdrop(backdrop));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::backdrop,
            commands::quick_paths,
            commands::start_scan,
            commands::cancel_scan,
            commands::list_baselines,
            commands::create_baseline,
            commands::verify_baseline,
            commands::delete_baseline,
            commands::reveal_in_explorer,
            commands::save_text,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start hashguard panel");
}
