use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum DockItemRequest {
    #[serde(rename = "app", rename_all = "camelCase")]
    App { app_path: String },
    #[serde(rename = "url")]
    Url { url: String },
}

#[tauri::command]
fn open_dock_item(item: DockItemRequest) -> Result<(), String> {
    match item {
        DockItemRequest::App { app_path } => open_with_macos(&app_path),
        DockItemRequest::Url { url } => open_with_macos(&url),
    }
}

fn open_with_macos(target: &str) -> Result<(), String> {
    Command::new("open")
        .arg(target)
        .spawn()
        .map_err(|e| format!("Failed to open {}: {}", target, e))?;
    Ok(())
}

#[tauri::command]
fn get_app_icon(app_path: String) -> Result<Vec<u8>, String> {
    let app_path = PathBuf::from(&app_path);
    let info_plist = app_path.join("Contents").join("Info.plist");
    let resources_dir = app_path.join("Contents").join("Resources");

    let icon_name = read_bundle_icon_name(&info_plist)?;
    let icon_path = resources_dir.join(ensure_icns_extension(&icon_name));

    if !icon_path.exists() {
        return Err(format!("App icon not found at {}", icon_path.display()));
    }

    let cache_dir = std::env::temp_dir().join("workspace-dock-icons");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create icon cache: {}", e))?;

    let output_path = cache_dir.join(format!("{}.png", safe_file_name(&app_path)));
    let output = Command::new("sips")
        .args(["-s", "format", "png", "-Z", "256"])
        .arg(&icon_path)
        .arg("--out")
        .arg(&output_path)
        .output()
        .map_err(|e| format!("Failed to convert app icon: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to convert {}: {}",
            icon_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    fs::read(&output_path)
        .map_err(|e| format!("Failed to read converted app icon: {}", e))
}

fn read_bundle_icon_name(info_plist: &Path) -> Result<String, String> {
    let output = Command::new("plutil")
        .args(["-extract", "CFBundleIconFile", "raw", "-o", "-"])
        .arg(info_plist)
        .output()
        .map_err(|e| format!("Failed to read app metadata: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to read icon metadata from {}: {}",
            info_plist.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if icon_name.is_empty() {
        return Err(format!("No app icon declared in {}", info_plist.display()));
    }

    Ok(icon_name)
}

fn ensure_icns_extension(icon_name: &str) -> String {
    if icon_name.ends_with(".icns") {
        icon_name.to_string()
    } else {
        format!("{}.icns", icon_name)
    }
}

fn safe_file_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("app")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_dock_item, get_app_icon])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
