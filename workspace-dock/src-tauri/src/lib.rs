use std::process::Command;

#[tauri::command]
fn open_app(app_path: String) -> Result<(), String> {
    Command::new("open")
        .arg(&app_path)
        .spawn()
        .map_err(|e| format!("Failed to open {}: {}", app_path, e))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
