# Add Item Modal — Implementation Plan

Target UI: a floating "Add Item" modal that opens from an edit-mode dock, with **Applications / Folders / Links** tabs, a search bar, **Suggestions** and **Recents** grids, and drag-into-dock support. Edit mode also adds a cyan outline around the dock, minus badges on each item, a dashed `+` slot, and a "Done" button.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the current state.

## What the screenshot shows vs. what we have

| Element in screenshot | Status today |
| --- | --- |
| Dock at the bottom with app icons | Have it |
| Cyan **edit-mode outline** around the dock | Missing |
| Minus badges on each item to remove | Missing |
| Dashed `+` slot at the right end of the dock | Missing |
| Floating **"Add Item"** modal above the dock | Missing (no second window) |
| Tabs: **Applications / Folders / Links** | Only `app` / `url` types exist |
| Search bar | Missing |
| **Suggestions** grid (Notion, Figma, Slack, …) | Missing (no app discovery) |
| **Recents** grid | Missing (no usage history) |
| "Drag an item into the dock" (drag from modal → dock) | Missing |
| **Done** button (exits edit mode) | Missing |
| Persistence of any of this | Items hardcoded in [config/dockItems.ts](src/features/dock/config/dockItems.ts) |

## What's missing — by layer

### 1. Data model & types
Files: [src/features/dock/types.ts](src/features/dock/types.ts), [src-tauri/src/lib.rs](src-tauri/src/lib.rs)

- Add `FolderDockItem` (`type: "folder"`, `path: string`).
- `UrlDockItem` already covers Links — relabel in UI only.
- Mirror in Rust `DockItemRequest` and route folders through `open` like apps.

### 2. Persistence
Currently hardcoded in [src/features/dock/config/dockItems.ts](src/features/dock/config/dockItems.ts).

- Move items into a JSON store on disk (Tauri `path::app_data_dir()`), or use `tauri-plugin-store`.
- Add IPC commands `get_dock_items` / `set_dock_items`.
- A `useDockItemsStore` hook in [src/features/dock/hooks/](src/features/dock/hooks/) replaces the imported constant.

### 3. Edit mode (cyan outline + minus badges + dashed `+` slot + Done)
- New `useDockEditMode` hook holding `isEditing` boolean.
- Triggered by right-click (or long-press) on the dock background — the natural macOS analogue.
- `Dock` gains an `editing` prop. When editing it renders:
  - An outline overlay around the dock,
  - A `RemoveBadge` on each `DockItemButton`,
  - An `AddItemSlot` after the items,
  - A "Done" affordance somewhere accessible.
- `dockbar`'s `sortable` is already on; reorder writes back to the store.

### 4. App discovery (Suggestions / Search results)
Files: [src-tauri/src/lib.rs](src-tauri/src/lib.rs), [src/features/dock/services/dockService.ts](src/features/dock/services/dockService.ts)

- New Rust command `list_installed_apps()` that scans `/Applications`, `/System/Applications`, `~/Applications` for `*.app` bundles and returns `[{ name, path, bundleId }]`.
- Reuse existing `read_app_display_name` + `get_app_icon` so the modal grid uses the same icons as the dock.
- Cache the list in Rust (refresh on demand) — scanning is the slow part.

### 5. Recents
- Track `lastOpenedAt` per item in the same store the dock items live in.
- "Recents" in the modal = top N apps by `lastOpenedAt` that aren't already in the dock.
- Bump `lastOpenedAt` inside `open_dock_item` (Rust) or in `DockFeature.handleOpenItem` (TS).

### 6. The Add Item modal itself
New feature folder:

```
src/features/add-item/
  AddItemModal.tsx
  tabs/
    ApplicationsTab.tsx
    FoldersTab.tsx
    LinksTab.tsx
  components/
    AppGrid.tsx
    SearchBar.tsx
    AppTile.tsx
```

- See **Window architecture decision** below for *where* this renders.
- Folders tab → native folder picker via `@tauri-apps/plugin-dialog`.
- Links tab → URL input + favicon fetch.

### 7. Drag from modal → dock
- Native HTML5 drag-and-drop from `AppTile` into `dock-wrapper`. `dockbar` accepts `dock-item` children; the drop handler calls `addDockItem(item)`.

## Architectural decision: where does the modal live?

The current dock window is **560×108 px**, transparent, re-parented into a custom borderless `NSPanel`. The modal in the screenshot is ~640×600 px and sits above the dock. Three options:

### Option A — Resize the existing panel when the modal opens
- Grow window to ~640×720, render modal + dock in the same React tree.
- Simple TS-side, but the underlying `NSPanel` must be resized in Rust too.
- The transparent area outside the dock has to stay click-through-friendly.

### Option B — Spawn a second Tauri window/panel for the modal (**recommended**)
- Clean separation; modal has its own background/blur; dock window stays tiny.
- Requires a second `windows[]` entry in [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) and a `show_add_item_modal` Rust command.
- Communicate via Tauri events.

### Option C — Render the modal inside the dock window using DOM overflow
- The dock window's `width=560 height=108` clips everything outside.
- Would require dropping the small fixed size and using a much larger transparent window with click-through outside the dock area — fights the current native panel setup.

**Recommendation: Option B.** Matches how macOS apps usually do this and keeps the existing dock window pristine.

## Phased rollout

### Phase 1 — Persistence
Replace hardcoded `dockItems` with an on-disk store + IPC. Nothing visual changes; unblocks everything else.

### Phase 2 — Edit mode skeleton
Right-click → cyan outline, minus badges, working remove + reorder + "Done". Still no add flow.

### Phase 3 — Add via Folders / Links tabs first
No app scanning needed yet. The `+` slot opens the modal (window option B); Folders and Links tabs work end-to-end.

### Phase 4 — App scanner + Suggestions / Recents grid
Implement `list_installed_apps`, recents tracking, search.

### Phase 5 — Drag-from-modal-into-dock
Nicer than click-to-add, but the click path from Phase 3/4 already works.

## Open questions

- Modal window: option **A** or **B** (recommend **B**)?
- Edit-mode trigger: right-click only, or also a button somewhere?
- Recents source: just our own `lastOpenedAt`, or also pull from macOS LaunchServices history?
- Should Suggestions be hardcoded "popular apps" or derived from `lastOpenedAt` / install date?
