pub mod codegen;
pub mod commands;
pub mod fs;
pub mod import_graph;
pub mod model;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::new_project,
            commands::save_project,
            commands::load_project,
            commands::generate_code,
            commands::list_middlewares,
            commands::import_graph,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
