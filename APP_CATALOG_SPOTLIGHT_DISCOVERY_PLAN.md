# App Catalog Spotlight Discovery Plan

## Summary

Replace the current top-level folder scan with Spotlight-backed app discovery while preserving the existing App Catalog API and UI behavior. The folder scan remains the fallback path, and the disk metadata cache still protects cold-start modal performance.

Use `mdfind` for this slice rather than `NSMetadataQuery`. It gives us Spotlight speed and coverage without introducing native query lifecycle, run loop, or callback complexity yet.

Status: implemented in `src-tauri/src/lib.rs`. The docs in `ADD_ITEM_MODAL_APP_LOADING.md` now record this as Slice 6.

## Target Flow

```txt
list_installed_apps
  memory cache
  disk metadata cache
    background refresh
  Spotlight discovery
    fallback folder scan
```

```txt
refresh_installed_apps
  clear memory cache
  run Spotlight discovery
  fallback folder scan if Spotlight fails or returns empty
  save memory cache
  save disk metadata cache
```

## Implementation Changes

- Keep the frontend contract unchanged: `InstalledApp[]` still contains `name`, `path`, `bundleId`, and optional `cachedIcon`.
- In `src-tauri/src/lib.rs`, split discovery into smaller pieces:
  - `discover_app_paths_with_spotlight() -> Result<Vec<PathBuf>, String>`
  - `discover_app_paths_with_folder_scan() -> Vec<PathBuf>`
  - `build_installed_apps_from_paths(paths: Vec<PathBuf>) -> Vec<InstalledApp>`
  - `discover_installed_apps()` chooses Spotlight first, folder scan second.
- Use this Spotlight query:
  ```sh
  mdfind "kMDItemContentTypeTree == 'com.apple.application-bundle'"
  ```
- Filter Spotlight results to app locations we intentionally support:
  - `/Applications`
  - `/System/Applications`
  - `/System/Library/CoreServices/Applications`
  - `~/Applications`
- Normalize and dedupe paths before building rows:
  - require existing paths
  - require `.app` extension
  - dedupe by canonical path when possible, otherwise by raw path string
- Preserve existing row-building behavior:
  - display name: `mdls kMDItemDisplayName`, then `CFBundleDisplayName`, then `CFBundleName`, then filename fallback
  - bundle ID: `CFBundleIdentifier`, then filename fallback
  - icons: attach `cachedIcon` from the PNG cache only when already available
- Add `[app-catalog]` logs showing whether discovery used `spotlight` or `folder-scan`, app count, and duration.
- Update `ADD_ITEM_MODAL_APP_LOADING.md` with Slice 6:
  - Spotlight is primary discovery.
  - Folder scan remains fallback.
  - Disk cache remains the cold-start path.
  - Spotlight can discover nested apps that the old top-level scan missed.

## Test Plan

- Run `cargo check`.
- Run `npm run build`.
- Manually verify:
  - Add Item modal opens quickly.
  - Apps still appear with cached icons.
  - Refresh still returns apps.
  - Logs identify `spotlight` discovery on normal runs.
  - Folder-scan fallback still returns apps if Spotlight fails or returns no usable paths.
  - Nested apps, if indexed by Spotlight, can appear in the catalog.

## Assumptions

- This slice does not add `NSMetadataQuery`, directory watching, or live updates.
- This slice does not change the modal UI or add a refresh button.
- Existing disk metadata cache and lazy icon loading remain in place.
- Folder scan fallback remains intentionally conservative and can be improved separately.
