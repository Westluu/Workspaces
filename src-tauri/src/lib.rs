#[cfg(target_os = "macos")]
use std::ptr::NonNull;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Instant,
};

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
    NSWindowCollectionBehavior, NSWindowStyleMask, NSWorkspace,
    NSWorkspaceActiveSpaceDidChangeNotification, NSWorkspaceDidActivateApplicationNotification,
};
#[cfg(target_os = "macos")]
use objc2_foundation::{MainThreadMarker, NSNotification, NSPoint, NSRect, NSSize};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum DockItemRequest {
    #[serde(rename = "app", rename_all = "camelCase")]
    App { app_path: String },
    #[serde(rename = "url")]
    Url { url: String },
    #[serde(rename = "folder", rename_all = "camelCase")]
    Folder { folder_path: String },
}

#[derive(Clone, Serialize)]
struct DockMouseMovePayload {
    x: f64,
    y: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockVisibilityPayload {
    hidden: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstalledApp {
    name: String,
    path: String,
    bundle_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cached_icon: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedInstalledApp {
    name: String,
    path: String,
    bundle_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppCatalogDiskCache {
    version: u32,
    apps: Vec<CachedInstalledApp>,
}

const APP_CATALOG_DISK_CACHE_VERSION: u32 = 1;
const MIN_CACHED_ICON_BYTES: usize = 2048;

static INSTALLED_APPS_CACHE: Mutex<Vec<InstalledApp>> = Mutex::new(Vec::new());
static APP_CATALOG_REFRESH_IN_FLIGHT: Mutex<bool> = Mutex::new(false);

// Raw pointer to the NSPanel created by install_native_dock_panel. We intentionally
// leak the Retained<NSPanel> to keep the panel alive for the lifetime of the app, and
// store the raw pointer here so that Tauri commands can resize/query it without going
// through the dead original Tauri window.
//
// Safety: We only dereference this pointer on the main thread (enforced by
// MainThreadMarker), and the pointed-to NSPanel is never freed (we leaked it).
// The raw pointer itself is just a usize under the hood, so wrapping it in a
// Send newtype is safe given these invariants.
#[cfg(target_os = "macos")]
struct SendablePtr(NonNull<AnyObject>);

#[cfg(target_os = "macos")]
unsafe impl Send for SendablePtr {}

#[cfg(target_os = "macos")]
static DOCK_PANEL_PTR: Mutex<Option<SendablePtr>> = Mutex::new(None);

#[cfg(target_os = "macos")]
const FINDER_BUNDLE_ID: &str = "com.apple.finder";
#[cfg(target_os = "macos")]
const WORKSPACE_DOCK_BUNDLE_ID: &str = "com.wesleyluu.workspace-dock";

const DEFAULT_DOCK_ITEMS_JSON: &str = r#"[
  {"type":"app","label":"VS Code","appPath":"/Applications/Visual Studio Code.app"},
  {"type":"app","label":"Chrome","appPath":"/Applications/Google Chrome.app"},
  {"type":"url","label":"localhost:3000","url":"http://localhost:3000"}
]"#;

#[tauri::command]
fn open_dock_item(item: DockItemRequest) -> Result<(), String> {
    match item {
        DockItemRequest::App { app_path } => open_with_macos(&app_path),
        DockItemRequest::Url { url } => open_with_macos(&url),
        DockItemRequest::Folder { folder_path } => open_with_macos(&folder_path),
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
    let started_at = Instant::now();
    let result = get_app_icon_bytes(&app_path);
    if let Ok(icon_bytes) = &result {
        update_cached_icon_for_app(&app_path, icon_bytes);
    }
    let status = if result.is_ok() { "ok" } else { "error" };
    eprintln!(
        "[app-catalog] get_app_icon path=\"{}\" status={} duration_ms={}",
        app_path,
        status,
        started_at.elapsed().as_millis()
    );

    result
}

fn get_app_icon_bytes(app_path: &str) -> Result<Vec<u8>, String> {
    let app_path = PathBuf::from(app_path);

    if let Some(cached_icon) = read_cached_app_icon(&app_path) {
        return Ok(cached_icon);
    }

    let output_path = app_icon_cache_path(&app_path)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create icon cache: {}", e))?;
    }

    // NSWorkspace knows about asset-catalog/system icons. Some Apple app bundles
    // still declare tiny placeholder .icns files that convert to transparent PNGs.
    match get_app_icon_via_nsworkspace(&app_path, &output_path) {
        Ok(icon_bytes) => Ok(icon_bytes),
        Err(workspace_error) => {
            convert_bundle_icon_to_png(&app_path, &output_path).map_err(|bundle_icon_error| {
                format!(
                    "NSWorkspace icon failed: {}; bundle icon fallback failed: {}",
                    workspace_error, bundle_icon_error
                )
            })
        }
    }
}

fn convert_bundle_icon_to_png(app_path: &Path, output_path: &Path) -> Result<Vec<u8>, String> {
    let info_plist = app_path.join("Contents").join("Info.plist");
    let resources_dir = app_path.join("Contents").join("Resources");

    let icon_name = read_bundle_icon_name(&info_plist)?;
    let icon_path = resources_dir.join(ensure_icns_extension(&icon_name));

    if !icon_path.exists() {
        return Err(format!("App icon not found at {}", icon_path.display()));
    }

    let output = Command::new("sips")
        .args(["-s", "format", "png", "-Z", "256"])
        .arg(&icon_path)
        .arg("--out")
        .arg(output_path)
        .output()
        .map_err(|e| format!("Failed to convert app icon: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to convert {}: {}",
            icon_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let icon_bytes =
        fs::read(output_path).map_err(|e| format!("Failed to read converted app icon: {}", e))?;
    validate_icon_bytes(&icon_bytes)
        .map_err(|e| format!("Converted app icon is not usable: {}", e))?;

    Ok(icon_bytes)
}

fn get_app_icon_via_nsworkspace(app_path: &Path, output_path: &Path) -> Result<Vec<u8>, String> {
    let app_path_json = serde_json::to_string(&app_path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to encode app path: {}", e))?;
    let output_path_json = serde_json::to_string(&output_path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to encode output path: {}", e))?;

    let script = format!(
        r#"
ObjC.import("AppKit");
ObjC.import("Foundation");

var appPath = {};
var outputPath = {};
var icon = $.NSWorkspace.sharedWorkspace.iconForFile(appPath);
icon.setSize({{ width: 256, height: 256 }});
var tiffData = icon.TIFFRepresentation;
var bitmapRep = $.NSBitmapImageRep.imageRepWithData(tiffData);
var pngData = bitmapRep.representationUsingTypeProperties(4, $({{}}));
if (!pngData || !pngData.writeToFileAtomically(outputPath, true)) {{
  throw new Error("Failed to write PNG icon");
}}
"#,
        app_path_json, output_path_json
    );

    let output = Command::new("/usr/bin/osascript")
        .args(["-l", "JavaScript", "-e", &script])
        .output()
        .map_err(|e| format!("Failed to run NSWorkspace icon fallback: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let icon_bytes =
        fs::read(output_path).map_err(|e| format!("Failed to read NSWorkspace app icon: {}", e))?;
    validate_icon_bytes(&icon_bytes)
        .map_err(|e| format!("NSWorkspace app icon is not usable: {}", e))?;

    Ok(icon_bytes)
}

fn app_icon_cache_path(app_path: &Path) -> Result<PathBuf, String> {
    let cache_dir = std::env::temp_dir().join("workspace-dock-icons");
    Ok(cache_dir.join(format!("{}.png", app_icon_cache_key(app_path))))
}

fn legacy_app_icon_cache_path(app_path: &Path) -> Result<PathBuf, String> {
    let cache_dir = std::env::temp_dir().join("workspace-dock-icons");
    Ok(cache_dir.join(format!("{}.png", safe_file_name(app_path))))
}

fn app_icon_cache_key(app_path: &Path) -> String {
    let normalized_path = app_path.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(normalized_path.as_bytes());
    let digest = hasher.finalize();
    format!("v2-{}", general_purpose::URL_SAFE_NO_PAD.encode(digest))
}

fn read_cached_app_icon(app_path: &Path) -> Option<Vec<u8>> {
    let cache_path = app_icon_cache_path(app_path).ok()?;
    let icon_bytes = fs::read(&cache_path)
        .ok()
        .or_else(|| migrate_legacy_cached_app_icon(app_path, &cache_path))?;

    if validate_icon_bytes(&icon_bytes).is_err() {
        None
    } else {
        Some(icon_bytes)
    }
}

fn migrate_legacy_cached_app_icon(app_path: &Path, cache_path: &Path) -> Option<Vec<u8>> {
    let legacy_cache_path = legacy_app_icon_cache_path(app_path).ok()?;
    if legacy_cache_path == cache_path {
        return None;
    }

    let icon_bytes = fs::read(&legacy_cache_path).ok()?;
    if validate_icon_bytes(&icon_bytes).is_err() {
        return None;
    }

    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(cache_path, &icon_bytes);

    Some(icon_bytes)
}

fn validate_icon_bytes(icon_bytes: &[u8]) -> Result<(), String> {
    if icon_bytes.len() < MIN_CACHED_ICON_BYTES {
        return Err(format!(
            "icon data is too small ({} bytes)",
            icon_bytes.len()
        ));
    }

    Ok(())
}

fn update_cached_icon_for_app(app_path: &str, icon_bytes: &[u8]) {
    let Ok(mut cache) = INSTALLED_APPS_CACHE.lock() else {
        return;
    };

    if let Some(app) = cache.iter_mut().find(|app| app.path == app_path) {
        app.cached_icon = Some(png_bytes_to_data_url(icon_bytes));
    }
}

fn png_bytes_to_data_url(icon_bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(icon_bytes)
    )
}

#[tauri::command]
fn get_app_display_name(app_path: String) -> Result<String, String> {
    let app_path = PathBuf::from(&app_path);
    let info_plist = app_path.join("Contents").join("Info.plist");

    Ok(read_app_display_name(&app_path, &info_plist))
}

#[tauri::command]
fn get_dock_visibility() -> DockVisibilityPayload {
    current_dock_visibility()
}

#[tauri::command]
fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let started_at = Instant::now();
    let result = list_installed_apps_cached();
    let status = if result.is_ok() { "ok" } else { "error" };
    let count = result.as_ref().map(|apps| apps.len()).unwrap_or(0);
    eprintln!(
        "[app-catalog] list_installed_apps status={} count={} duration_ms={}",
        status,
        count,
        started_at.elapsed().as_millis()
    );

    result
}

fn list_installed_apps_cached() -> Result<Vec<InstalledApp>, String> {
    {
        let cache = INSTALLED_APPS_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock app cache: {}", e))?;
        if !cache.is_empty() {
            return Ok(cache.clone());
        }
    }

    if let Some(apps) = load_installed_apps_disk_cache() {
        eprintln!(
            "[app-catalog] app metadata loaded from disk count={}",
            apps.len()
        );
        let stored_apps = store_installed_apps_memory_cache(apps)?;
        spawn_app_catalog_background_refresh();
        return Ok(stored_apps);
    }

    let apps = discover_installed_apps();
    store_installed_apps_cache(apps)
}

#[tauri::command]
fn refresh_installed_apps() -> Result<Vec<InstalledApp>, String> {
    let started_at = Instant::now();
    let result = refresh_installed_apps_cache();
    let status = if result.is_ok() { "ok" } else { "error" };
    let count = result.as_ref().map(|apps| apps.len()).unwrap_or(0);
    eprintln!(
        "[app-catalog] refresh_installed_apps status={} count={} duration_ms={}",
        status,
        count,
        started_at.elapsed().as_millis()
    );

    result
}

fn refresh_installed_apps_cache() -> Result<Vec<InstalledApp>, String> {
    {
        let mut cache = INSTALLED_APPS_CACHE
            .lock()
            .map_err(|e| format!("Failed to lock app cache: {}", e))?;
        cache.clear();
    }

    let apps = discover_installed_apps();
    store_installed_apps_cache(apps)
}

fn spawn_app_catalog_background_refresh() {
    {
        let Ok(mut in_flight) = APP_CATALOG_REFRESH_IN_FLIGHT.lock() else {
            return;
        };
        if *in_flight {
            return;
        }
        *in_flight = true;
    }

    std::thread::spawn(|| {
        let started_at = Instant::now();
        let apps = discover_installed_apps();
        let count = apps.len();
        let status = match store_installed_apps_cache(apps) {
            Ok(_) => "ok",
            Err(e) => {
                eprintln!("[app-catalog] background refresh failed error=\"{}\"", e);
                "error"
            }
        };

        eprintln!(
            "[app-catalog] background refresh status={} count={} duration_ms={}",
            status,
            count,
            started_at.elapsed().as_millis()
        );

        if let Ok(mut in_flight) = APP_CATALOG_REFRESH_IN_FLIGHT.lock() {
            *in_flight = false;
        }
    });
}

fn discover_installed_apps() -> Vec<InstalledApp> {
    let started_at = Instant::now();
    let (source, paths) = match discover_app_paths_with_spotlight() {
        Ok(paths) if !paths.is_empty() => ("spotlight", paths),
        Ok(_) => {
            eprintln!("[app-catalog] spotlight discovery returned no usable app paths");
            ("folder-scan", discover_app_paths_with_folder_scan())
        }
        Err(e) => {
            eprintln!("[app-catalog] spotlight discovery failed error=\"{}\"", e);
            ("folder-scan", discover_app_paths_with_folder_scan())
        }
    };

    let apps = build_installed_apps_from_paths(paths);

    eprintln!(
        "[app-catalog] discovery source={} count={} duration_ms={}",
        source,
        apps.len(),
        started_at.elapsed().as_millis()
    );

    apps
}

fn discover_app_paths_with_spotlight() -> Result<Vec<PathBuf>, String> {
    let output = Command::new("mdfind")
        .arg("kMDItemContentTypeTree == 'com.apple.application-bundle'")
        .output()
        .map_err(|e| format!("Failed to run Spotlight app discovery: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Spotlight app discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let paths = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| is_supported_app_location(path))
        .collect::<Vec<_>>();

    Ok(normalize_app_paths(paths))
}

fn discover_app_paths_with_folder_scan() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for dir in supported_app_locations() {
        if !dir.exists() {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            paths.push(entry.path());
        }
    }

    normalize_app_paths(paths)
}

fn build_installed_apps_from_paths(paths: Vec<PathBuf>) -> Vec<InstalledApp> {
    let mut apps = paths
        .into_iter()
        .map(|path| {
            let info_plist = path.join("Contents").join("Info.plist");
            let name = read_app_display_name(&path, &info_plist);
            let bundle_id = read_bundle_id(&info_plist).unwrap_or_else(|| safe_file_name(&path));

            InstalledApp {
                name,
                path: path.to_string_lossy().to_string(),
                bundle_id,
                cached_icon: read_cached_app_icon(&path)
                    .map(|icon_bytes| png_bytes_to_data_url(&icon_bytes)),
            }
        })
        .collect::<Vec<_>>();

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    apps
}

fn normalize_app_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for path in paths {
        if !path.exists() || path.extension().and_then(|e| e.to_str()) != Some("app") {
            continue;
        }

        let dedupe_key = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .to_string_lossy()
            .to_string();

        if seen.insert(dedupe_key) {
            normalized.push(path);
        }
    }

    normalized
}

fn supported_app_locations() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Library/CoreServices/Applications"),
        PathBuf::from(format!("{}/Applications", home)),
    ]
}

fn is_supported_app_location(path: &Path) -> bool {
    supported_app_locations()
        .iter()
        .any(|location| path.starts_with(location))
}

fn store_installed_apps_cache(apps: Vec<InstalledApp>) -> Result<Vec<InstalledApp>, String> {
    let apps = store_installed_apps_memory_cache(apps)?;

    if let Err(e) = save_installed_apps_disk_cache(&apps) {
        eprintln!(
            "[app-catalog] app metadata disk save failed error=\"{}\"",
            e
        );
    }

    Ok(apps)
}

fn store_installed_apps_memory_cache(apps: Vec<InstalledApp>) -> Result<Vec<InstalledApp>, String> {
    let mut cache = INSTALLED_APPS_CACHE
        .lock()
        .map_err(|e| format!("Failed to lock app cache: {}", e))?;
    *cache = apps.clone();

    Ok(apps)
}

fn load_installed_apps_disk_cache() -> Option<Vec<InstalledApp>> {
    let cache_path = app_catalog_disk_cache_path();
    let raw = fs::read_to_string(cache_path).ok()?;
    let disk_cache: AppCatalogDiskCache = serde_json::from_str(&raw).ok()?;
    if disk_cache.version != APP_CATALOG_DISK_CACHE_VERSION {
        return None;
    }

    let apps = disk_cache
        .apps
        .into_iter()
        .filter_map(installed_app_from_disk_cache)
        .collect::<Vec<_>>();

    if apps.is_empty() {
        None
    } else {
        Some(apps)
    }
}

fn save_installed_apps_disk_cache(apps: &[InstalledApp]) -> Result<(), String> {
    let cache_path = app_catalog_disk_cache_path();
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create app catalog cache directory: {}", e))?;
    }

    let disk_cache = AppCatalogDiskCache {
        version: APP_CATALOG_DISK_CACHE_VERSION,
        apps: apps
            .iter()
            .map(|app| CachedInstalledApp {
                name: app.name.clone(),
                path: app.path.clone(),
                bundle_id: app.bundle_id.clone(),
            })
            .collect(),
    };

    let json = serde_json::to_string(&disk_cache)
        .map_err(|e| format!("Failed to serialize app catalog cache: {}", e))?;
    fs::write(cache_path, json).map_err(|e| format!("Failed to write app catalog cache: {}", e))
}

fn installed_app_from_disk_cache(cached: CachedInstalledApp) -> Option<InstalledApp> {
    let app_path = PathBuf::from(&cached.path);
    if !app_path.exists() {
        return None;
    }

    Some(InstalledApp {
        name: cached.name,
        path: cached.path,
        bundle_id: cached.bundle_id,
        cached_icon: read_cached_app_icon(&app_path)
            .map(|icon_bytes| png_bytes_to_data_url(&icon_bytes)),
    })
}

fn app_catalog_disk_cache_path() -> PathBuf {
    app_data_dir().join("app-catalog-cache.json")
}

fn app_data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("workspace-dock");
    }

    std::env::temp_dir().join("workspace-dock-data")
}

#[tauri::command]
fn get_dock_items() -> Result<String, String> {
    let path = dock_items_path();
    if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Failed to read dock items file: {}", e))
    } else {
        Ok(DEFAULT_DOCK_ITEMS_JSON.to_string())
    }
}

#[tauri::command]
fn set_dock_items(items_json: String) -> Result<(), String> {
    let path = dock_items_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create dock data directory: {}", e))?;
    }
    fs::write(&path, &items_json).map_err(|e| format!("Failed to write dock items file: {}", e))
}

fn dock_items_path() -> PathBuf {
    std::env::temp_dir().join("workspace-dock-data/dock_items.json")
}

fn read_bundle_id(info_plist: &Path) -> Option<String> {
    read_bundle_string(info_plist, "CFBundleIdentifier")
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
    let icon_name = read_bundle_string(info_plist, "CFBundleIconFile")
        .ok_or_else(|| format!("Failed to read icon metadata from {}", info_plist.display()))?;

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

    let raw_ptr: *mut NSPanel = Retained::into_raw(panel);
    if let Ok(mut guard) = DOCK_PANEL_PTR.lock() {
        if let Some(nn) = NonNull::new(raw_ptr.cast::<AnyObject>()) {
            *guard = Some(SendablePtr(nn));
        }
    }

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

#[cfg(target_os = "macos")]
fn install_active_app_tracking<R: tauri::Runtime>(app_handle: tauri::AppHandle<R>) {
    emit_current_dock_visibility(&app_handle);

    let activation_handle = app_handle.clone();
    let activation_block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        emit_current_dock_visibility(&activation_handle);
    });

    let space_handle = app_handle.clone();
    let space_block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        emit_current_dock_visibility(&space_handle);
    });

    let workspace = NSWorkspace::sharedWorkspace();
    let notification_center = workspace.notificationCenter();
    let activation_observer = unsafe {
        notification_center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceDidActivateApplicationNotification),
            None,
            None,
            &activation_block,
        )
    };
    let space_observer = unsafe {
        notification_center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceActiveSpaceDidChangeNotification),
            None,
            None,
            &space_block,
        )
    };

    let _activation_observer = Retained::into_raw(activation_observer);
    let _space_observer = Retained::into_raw(space_observer);
    let _activation_block = RcBlock::into_raw(activation_block);
    let _space_block = RcBlock::into_raw(space_block);
}

#[cfg(target_os = "macos")]
fn emit_current_dock_visibility<R: tauri::Runtime>(app_handle: &tauri::AppHandle<R>) {
    let _ = app_handle.emit("native-dock-visibility", current_dock_visibility());
}

#[cfg(target_os = "macos")]
fn current_dock_visibility() -> DockVisibilityPayload {
    let hidden = should_hide_dock_for_bundle_id(frontmost_app_bundle_id().as_deref());

    DockVisibilityPayload { hidden }
}

#[cfg(not(target_os = "macos"))]
fn current_dock_visibility() -> DockVisibilityPayload {
    DockVisibilityPayload { hidden: false }
}

#[cfg(target_os = "macos")]
fn should_hide_dock_for_bundle_id(bundle_id: Option<&str>) -> bool {
    bundle_id
        .map(|bundle_id| bundle_id != FINDER_BUNDLE_ID && bundle_id != WORKSPACE_DOCK_BUNDLE_ID)
        .unwrap_or(false)
}

#[cfg(target_os = "macos")]
fn frontmost_app_bundle_id() -> Option<String> {
    let workspace = NSWorkspace::sharedWorkspace();
    workspace.frontmostApplication().and_then(|app| {
        app.bundleIdentifier()
            .map(|bundle_id| bundle_id.to_string())
    })
}

#[cfg(not(target_os = "macos"))]
fn configure_dock_window_interaction<R: tauri::Runtime>(
    _app: &mut tauri::App<R>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn install_active_app_tracking<R: tauri::Runtime>(_app_handle: tauri::AppHandle<R>) {}

#[tauri::command]
fn resize_dock_panel(width: f64, height: f64, x: f64, y: f64) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _mtm = MainThreadMarker::new().ok_or("Must be on main thread")?;
        let guard = DOCK_PANEL_PTR
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let sendable_ptr = guard.as_ref().ok_or("No dock panel stored")?;
        let panel = unsafe { sendable_ptr.0.cast::<NSPanel>().as_ref() };

        let new_frame = NSRect::new(NSPoint::new(x, y), NSSize::new(width, height));
        panel.setFrame_display(new_frame, true);

        if let Some(content_view) = panel.contentView() {
            content_view.setFrame(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width, height),
            ));
        }

        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Not supported on this platform".to_string())
    }
}

#[tauri::command]
fn get_dock_panel_frame() -> Result<(f64, f64, f64, f64), String> {
    #[cfg(target_os = "macos")]
    {
        let guard = DOCK_PANEL_PTR
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;
        let sendable_ptr = guard.as_ref().ok_or("No dock panel stored")?;
        let panel = unsafe { sendable_ptr.0.cast::<NSPanel>().as_ref() };
        let frame = panel.frame();
        Ok((
            frame.size.width,
            frame.size.height,
            frame.origin.x,
            frame.origin.y,
        ))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Not supported on this platform".to_string())
    }
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
            install_active_app_tracking(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_display_name,
            get_app_icon,
            get_dock_items,
            get_dock_visibility,
            get_dock_panel_frame,
            list_installed_apps,
            open_dock_item,
            refresh_installed_apps,
            resize_dock_panel,
            set_dock_items,
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
