use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, PhysicalPosition};

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::MainThreadOnly;
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSPanel, NSScreen, NSView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum DockItemRequest {
    #[serde(rename = "app", rename_all = "camelCase")]
    App { app_path: String },
    #[serde(rename = "url")]
    Url { url: String },
}

#[derive(Clone, Serialize)]
struct DockMouseMovePayload {
    x: f64,
    y: f64,
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

#[tauri::command]
fn get_app_display_name(app_path: String) -> Result<String, String> {
    let app_path = PathBuf::from(&app_path);
    let info_plist = app_path.join("Contents").join("Info.plist");

    Ok(read_app_display_name(&app_path, &info_plist))
}

fn read_app_display_name(app_path: &Path, info_plist: &Path) -> String {
    read_spotlight_display_name(app_path)
        .or_else(|| read_bundle_string(info_plist, "CFBundleDisplayName"))
        .or_else(|| read_bundle_string(info_plist, "CFBundleName"))
        .unwrap_or_else(|| fallback_app_name(app_path))
}

fn read_spotlight_display_name(app_path: &Path) -> Option<String> {
    let output = Command::new("mdls")
        .args(["-name", "kMDItemDisplayName", "-raw"])
        .arg(app_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    normalize_metadata_value(&String::from_utf8_lossy(&output.stdout))
}

fn read_bundle_icon_name(info_plist: &Path) -> Result<String, String> {
    let icon_name = read_bundle_string(info_plist, "CFBundleIconFile").ok_or_else(|| {
        format!(
            "Failed to read icon metadata from {}",
            info_plist.display()
        )
    })?;

    if icon_name.is_empty() {
        return Err(format!("No app icon declared in {}", info_plist.display()));
    }

    Ok(icon_name)
}

fn read_bundle_string(info_plist: &Path, key: &str) -> Option<String> {
    let output = Command::new("plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(info_plist)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    normalize_metadata_value(&String::from_utf8_lossy(&output.stdout))
}

fn normalize_metadata_value(value: &str) -> Option<String> {
    let normalized = value.trim();

    if normalized.is_empty() || normalized == "(null)" {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn ensure_icns_extension(icon_name: &str) -> String {
    if icon_name.ends_with(".icns") {
        icon_name.to_string()
    } else {
        format!("{}.icns", icon_name)
    }
}

fn fallback_app_name(path: &Path) -> String {
    safe_file_name(path).replace('-', " ")
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

#[cfg(target_os = "macos")]
fn configure_dock_window_interaction<R: tauri::Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), String> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    app.set_dock_visibility(false);

    let window = app
        .get_webview_window("main")
        .ok_or("Main dock window was not found")?;
    let ns_window_ptr = window
        .ns_window()
        .map_err(|e| format!("Failed to access native dock window: {}", e))?;
    let ns_window = unsafe { &*(ns_window_ptr.cast::<NSWindow>()) };

    let style_mask = ns_window.styleMask();
    ns_window.setStyleMask(style_mask | NSWindowStyleMask::NonactivatingPanel);
    ns_window.setAcceptsMouseMovedEvents(true);
    ns_window.setHidesOnDeactivate(false);
    ns_window.setCanHide(false);
    unsafe {
        ns_window.setReleasedWhenClosed(false);
    }
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenNone,
    );
    ns_window.setLevel(20);
    ns_window.orderFrontRegardless();

    Ok(())
}

#[cfg(target_os = "macos")]
fn install_native_dock_panel<R: tauri::Runtime>(app: &mut tauri::App<R>) -> Result<(), String> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    app.set_dock_visibility(false);

    let mtm = MainThreadMarker::new().ok_or("Native dock panel must be created on main thread")?;
    let window = app
        .get_webview_window("main")
        .ok_or("Main dock window was not found")?;
    let ns_window_ptr = window
        .ns_window()
        .map_err(|e| format!("Failed to access native dock window: {}", e))?;
    let ns_view_ptr = window
        .ns_view()
        .map_err(|e| format!("Failed to access native dock webview: {}", e))?;

    let ns_window = unsafe { &*(ns_window_ptr.cast::<NSWindow>()) };
    let ns_view = unsafe { &*(ns_view_ptr.cast::<NSView>()) };
    let frame = dock_panel_frame(mtm, ns_window.frame());
    let content_frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(frame.size.width, frame.size.height),
    );

    let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
        NSPanel::alloc(mtm),
        frame,
        NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
        NSBackingStoreType::Buffered,
        false,
    );

    configure_native_dock_panel(&panel);

    ns_view.removeFromSuperview();
    ns_view.setFrame(content_frame);
    panel.setContentView(Some(ns_view));
    ns_window.orderOut(None);
    panel.setFrame_display(frame, true);
    panel.orderFrontRegardless();
    install_inactive_mouse_tracking(app.handle().clone(), frame);

    let _panel = Retained::into_raw(panel);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_native_dock_panel<R: tauri::Runtime>(_app: &mut tauri::App<R>) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_native_dock_panel(panel: &NSPanel) {
    panel.setFloatingPanel(true);
    panel.setBecomesKeyOnlyIfNeeded(true);
    panel.setWorksWhenModal(true);
    panel.setAcceptsMouseMovedEvents(true);
    panel.setHidesOnDeactivate(false);
    panel.setCanHide(false);
    panel.setCollectionBehavior(
        NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenNone,
    );
    panel.setLevel(20);
    panel.setHasShadow(false);
    panel.setOpaque(false);
    panel.setBackgroundColor(Some(&NSColor::clearColor()));
    unsafe {
        panel.setReleasedWhenClosed(false);
    }
}

#[cfg(target_os = "macos")]
fn dock_panel_frame(mtm: MainThreadMarker, fallback_frame: NSRect) -> NSRect {
    let screen = NSScreen::mainScreen(mtm);
    let screen_frame = screen
        .as_ref()
        .map(|screen| screen.frame())
        .unwrap_or(fallback_frame);
    let width = fallback_frame.size.width;
    let height = fallback_frame.size.height;
    let bottom_offset = 6.0;
    let x = screen_frame.origin.x + ((screen_frame.size.width - width) / 2.0);
    let y = screen_frame.origin.y + bottom_offset;

    NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
}

#[cfg(target_os = "macos")]
fn install_inactive_mouse_tracking<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    panel_frame: NSRect,
) {
    let block = RcBlock::new(move |_event: std::ptr::NonNull<NSEvent>| {
        let mouse = NSEvent::mouseLocation();
        let relative_x = mouse.x - panel_frame.origin.x;
        let relative_y = panel_frame.size.height - (mouse.y - panel_frame.origin.y);

        if relative_x >= 0.0
            && relative_x <= panel_frame.size.width
            && relative_y >= 0.0
            && relative_y <= panel_frame.size.height
        {
            let _ = app_handle.emit(
                "native-dock-mouse-move",
                DockMouseMovePayload {
                    x: relative_x,
                    y: relative_y,
                },
            );
        } else {
            let _ = app_handle.emit("native-dock-mouse-leave", ());
        }
    });

    if let Some(monitor) =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(NSEventMask::MouseMoved, &block)
    {
        let _monitor: *mut AnyObject = Retained::into_raw(monitor);
        let _block = RcBlock::into_raw(block);
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_dock_window_interaction<R: tauri::Runtime>(
    _app: &mut tauri::App<R>,
) -> Result<(), String> {
    Ok(())
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
            if let Err(panel_error) = install_native_dock_panel(app) {
                eprintln!("{}", panel_error);
                if let Err(window_error) = configure_dock_window_interaction(app) {
                    eprintln!("{}", window_error);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_display_name,
            get_app_icon,
            open_dock_item,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit => {
            restore_system_dock_bottom();
        }
        _ => {}
    });
}
