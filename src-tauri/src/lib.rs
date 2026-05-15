use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Deserialize;
use tauri::{Manager, PhysicalPosition};

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
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create icon cache: {}", e))?;

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

    fs::read(&output_path).map_err(|e| format!("Failed to read converted app icon: {}", e))
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

#[cfg(target_os = "macos")]
fn set_macos_dock_orientation(orientation: &str) -> Result<(), String> {
    if !matches!(orientation, "left" | "bottom" | "right") {
        return Err(format!("Unsupported Dock orientation: {}", orientation));
    }

    let defaults_status = Command::new("defaults")
        .args([
            "write",
            "com.apple.dock",
            "orientation",
            "-string",
            orientation,
        ])
        .status()
        .map_err(|e| format!("Failed to update macOS Dock orientation: {}", e))?;

    if !defaults_status.success() {
        return Err(format!(
            "defaults failed while setting Dock orientation to {}",
            orientation
        ));
    }

    let killall_status = Command::new("killall")
        .arg("Dock")
        .status()
        .map_err(|e| format!("Failed to restart macOS Dock: {}", e))?;

    if !killall_status.success() {
        return Err("killall Dock failed while restarting macOS Dock".to_string());
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn set_macos_dock_orientation(_orientation: &str) -> Result<(), String> {
    Ok(())
}

fn move_system_dock_left() {
    if let Err(e) = set_macos_dock_orientation("left") {
        eprintln!("{}", e);
    }
}

fn restore_system_dock_bottom() {
    if let Err(e) = set_macos_dock_orientation("bottom") {
        eprintln!("{}", e);
    }
}

#[cfg(target_os = "macos")]
fn build_dock_restore_signal_set() -> libc::sigset_t {
    let mut signal_set = unsafe { std::mem::zeroed::<libc::sigset_t>() };

    unsafe {
        libc::sigemptyset(&mut signal_set);
        libc::sigaddset(&mut signal_set, libc::SIGINT);
        libc::sigaddset(&mut signal_set, libc::SIGTERM);
    }

    signal_set
}

#[cfg(target_os = "macos")]
fn install_dock_restore_signal_handler() {
    let signal_set = build_dock_restore_signal_set();

    unsafe {
        libc::pthread_sigmask(libc::SIG_BLOCK, &signal_set, std::ptr::null_mut());
    }

    std::thread::spawn(|| {
        let signal_set = build_dock_restore_signal_set();
        let mut signal = 0;

        let wait_result = unsafe { libc::sigwait(&signal_set, &mut signal) };
        if wait_result == 0 {
            restore_system_dock_bottom();
            std::process::exit(if signal == libc::SIGINT { 130 } else { 143 });
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn install_dock_restore_signal_handler() {}

fn position_dock_window<R: tauri::Runtime>(app: &tauri::App<R>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main dock window was not found")?;
    let monitor = window
        .current_monitor()
        .map_err(|e| format!("Failed to read current monitor: {}", e))?
        .or(window
            .primary_monitor()
            .map_err(|e| format!("Failed to read primary monitor: {}", e))?)
        .ok_or("No monitor found for dock window")?;
    let window_size = window
        .outer_size()
        .map_err(|e| format!("Failed to read dock window size: {}", e))?;

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let bottom_offset = 6;
    let x = monitor_position.x + ((monitor_size.width as i32 - window_size.width as i32) / 2);
    let y =
        monitor_position.y + monitor_size.height as i32 - window_size.height as i32 - bottom_offset;

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| format!("Failed to position dock window: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    install_dock_restore_signal_handler();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            move_system_dock_left();
            if let Err(e) = position_dock_window(app) {
                eprintln!("{}", e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![open_dock_item, get_app_icon])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
            restore_system_dock_bottom();
        }
        _ => {}
    });
}
