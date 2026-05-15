# Workspace Dock Architecture

This document describes the current state of the app: a Tauri + React floating dock that hosts a hardcoded list of items, runs as a native macOS `NSPanel`, and auto-hides based on which app is frontmost.

## High-Level App Shape

```mermaid
flowchart TB
  User[User]

  subgraph TauriApp["Tauri Desktop App"]
    subgraph Window["Floating Dock Window"]
      Config["tauri.conf.json<br/>560x108, transparent,<br/>borderless, always-on-top,<br/>macOSPrivateApi"]
      WebView["WebView (re-parented<br/>into native NSPanel)"]
    end

    subgraph Frontend["React Frontend (src/features/dock)"]
      App["App.tsx → DockFeature"]
      Config2["config/dockItems.ts<br/>hardcoded items + flags"]
      Components["components/<br/>Dock, DockItemButton,<br/>DockTitle, ErrorToast"]
      Hooks["hooks/<br/>useDockIcons, useDockLabels,<br/>useDockTitle, useDockVisibility,<br/>useNativeDockMouseTracking"]
      Services["services/dockService.ts<br/>open_dock_item,<br/>get_app_icon,<br/>get_app_display_name"]
      Dockbar["dockbar npm pkg<br/>&lt;dock-wrapper&gt; / &lt;dock-item&gt;<br/>web components"]
    end

    subgraph Bridge["Tauri IPC Bridge"]
      Invoke["invoke(...)"]
      Listen["listen(event, ...)"]
    end

    subgraph RustBackend["Rust Backend (src-tauri/src/lib.rs)"]
      OpenItem["open_dock_item(item)"]
      GetIcon["get_app_icon(app_path)"]
      GetName["get_app_display_name(app_path)"]
      GetVis["get_dock_visibility()"]
      NativePanel["install_native_dock_panel<br/>re-parents webview into NSPanel"]
      DockOrient["set_macos_dock_orientation<br/>moves system Dock to left,<br/>restores on exit"]
      MouseTrack["install_inactive_mouse_tracking<br/>NSEvent global mouse monitor"]
      AppTrack["install_active_app_tracking<br/>NSWorkspace activation +<br/>active space change observers"]
      SignalHandler["install_dock_restore_signal_handler<br/>SIGINT/SIGTERM → restore Dock"]
    end
  end

  subgraph MacOS["macOS System"]
    OpenCmd["open"]
    Plutil["plutil"]
    Mdls["mdls"]
    Sips["sips"]
    Defaults["defaults / killall Dock"]
    AppBundles["App bundles<br/>VS Code, Chrome, ..."]
    Localhost["http://localhost:3000"]
    TempPng["Temp icon PNG cache<br/>workspace-dock-icons"]
    Workspace["NSWorkspace<br/>frontmost app + space"]
  end

  User --> WebView
  Config --> Window
  WebView --> App
  App --> Components
  App --> Hooks
  Components --> Dockbar
  Hooks --> Services
  Services --> Invoke
  Hooks --> Listen
  Invoke --> OpenItem
  Invoke --> GetIcon
  Invoke --> GetName
  Invoke --> GetVis
  OpenItem --> OpenCmd
  GetIcon --> Plutil
  GetIcon --> Sips
  GetName --> Mdls
  GetName --> Plutil
  Plutil --> AppBundles
  Mdls --> AppBundles
  Sips --> TempPng
  OpenCmd --> AppBundles
  OpenCmd --> Localhost
  NativePanel --> WebView
  DockOrient --> Defaults
  AppTrack --> Workspace
  AppTrack -->|emits<br/>native-dock-visibility| Listen
  MouseTrack -->|emits<br/>native-dock-mouse-move /<br/>native-dock-mouse-leave| Listen
  SignalHandler --> DockOrient
```

## Frontend Component And State Flow

```mermaid
flowchart LR
  App["App.tsx"]
  Feature["DockFeature"]
  Items["dockItems<br/>(hardcoded)"]
  IconsHook["useDockIcons"]
  LabelsHook["useDockLabels"]
  TitleHook["useDockTitle"]
  VisibilityHook["useDockVisibility"]
  NativeMouseHook["useNativeDockMouseTracking"]
  DockService["openDockItem"]
  Main["main.playground<br/>(adds .dock-hidden when hidden)"]
  DockTitle["DockTitle<br/>(floating tooltip)"]
  Dock["Dock"]
  Wrapper["dock-wrapper<br/>(dockbar web component)"]
  ItemSlot["dock-item × N"]
  ItemButton["DockItemButton"]
  Img["img.app-icon"]
  Fallback["fallback-icon span<br/>(initials or LH)"]
  ThemeBtn["theme-toggle button (☾)"]
  Toast["ErrorToast"]

  App --> Feature
  Feature --> Items
  Items --> IconsHook
  Items --> LabelsHook
  Feature --> TitleHook
  Feature --> VisibilityHook
  Feature --> NativeMouseHook
  Feature --> DockService
  Feature --> Main
  Main --> DockTitle
  Main --> Dock
  Dock --> Wrapper
  Wrapper --> ItemSlot
  Wrapper --> ThemeBtn
  ItemSlot --> ItemButton
  ItemButton --> Img
  ItemButton --> Fallback
  IconsHook --> ItemButton
  LabelsHook --> ItemButton
  Main --> Toast
```

## App Icon and Label Loading

```mermaid
sequenceDiagram
  participant React as useDockIcons / useDockLabels
  participant IPC as Tauri invoke
  participant Rust as Rust commands
  participant Mdls as macOS mdls
  participant Plutil as macOS plutil
  participant Sips as macOS sips
  participant FS as Temp PNG Cache

  par Icon
    React->>IPC: invoke("get_app_icon", { appPath })
    IPC->>Rust: get_app_icon(app_path)
    Rust->>Plutil: read CFBundleIconFile from Info.plist
    Plutil-->>Rust: icon name
    Rust->>Sips: convert .icns → 256px PNG
    Sips->>FS: write workspace-dock-icons/*.png
    Rust->>FS: read PNG bytes
    Rust-->>IPC: Vec<u8>
    IPC-->>React: number[]
    React->>React: pngBytesToDataUrl + setAppIconUrls
  and Label
    React->>IPC: invoke("get_app_display_name", { appPath })
    IPC->>Rust: get_app_display_name(app_path)
    Rust->>Mdls: kMDItemDisplayName (Spotlight)
    Mdls-->>Rust: display name (or empty)
    alt Spotlight returned a value
      Rust-->>IPC: name
    else fall back
      Rust->>Plutil: CFBundleDisplayName, then CFBundleName
      Plutil-->>Rust: bundle string (or empty)
      Rust-->>IPC: name or filename-derived fallback
    end
    IPC-->>React: string
    React->>React: setAppLabels
  end
```

## Launch Data Flow

```mermaid
sequenceDiagram
  participant User
  participant React as DockFeature
  participant Service as dockService
  participant IPC as Tauri invoke
  participant Rust as open_dock_item
  participant Mac as macOS open
  participant Target as App or URL

  User->>React: click DockItemButton
  React->>React: setError(null)
  React->>Service: openDockItem(item)
  Service->>IPC: invoke("open_dock_item", { item })
  IPC->>Rust: open_dock_item(DockItemRequest)
  Rust->>Mac: Command::new("open").arg(target)
  Mac->>Target: launch / focus app or open URL
  Rust-->>IPC: Ok or Err(string)
  IPC-->>React: resolves or rejects
  React->>React: on reject → setError(message) → ErrorToast
```

## Dock Auto-Hide (Native Tracking)

The dock window is shown only when the frontmost app is **Finder** or **Workspace Dock** itself; for any other foreground app the React layer applies `.dock-hidden` (CSS slides it off-screen). Native mouse-move events let the dock react to hover even though it lives in an `NSPanel` that doesn't take focus.

```mermaid
sequenceDiagram
  participant Mac as macOS
  participant Rust as Rust (NSWorkspace + NSEvent observers)
  participant Tauri as Tauri event bus
  participant Vis as useDockVisibility
  participant Mouse as useNativeDockMouseTracking
  participant DOM as React DOM

  Note over Rust: install_active_app_tracking on startup
  Mac->>Rust: NSWorkspaceDidActivateApplicationNotification
  Rust->>Rust: should_hide_dock_for_bundle_id(...)
  Rust-->>Tauri: emit "native-dock-visibility" { hidden }
  Tauri-->>Vis: payload
  Vis->>DOM: setIsDockHidden + clearDockTitle when hidden

  Note over Rust: install_inactive_mouse_tracking (NSEvent global monitor)
  Mac->>Rust: mouseMoved (any space)
  Rust->>Rust: translate to dock-relative coords
  alt inside panel rect
    Rust-->>Tauri: emit "native-dock-mouse-move" { x, y }
    Tauri-->>Mouse: payload
    Mouse->>DOM: elementFromPoint → showDockTitle / clearDockTitle
    Tauri-->>Vis: same event reused → if auto-hidden, reveal
  else outside panel rect
    Rust-->>Tauri: emit "native-dock-mouse-leave"
    Tauri-->>Mouse: clearDockTitle
    Tauri-->>Vis: re-hide if auto-hide is active
  end
```

## Native macOS Window Setup

The dock isn't a normal Tauri window. On startup the Rust setup re-parents the WebView into a custom `NSPanel` so it can float above all spaces without stealing focus, and tweaks the system Dock so the user has free vertical real estate.

```mermaid
flowchart TB
  Run["run() in lib.rs"]
  Sig["install_dock_restore_signal_handler<br/>(blocks SIGINT/SIGTERM,<br/>restores system Dock orientation<br/>before exit)"]
  Move["move_system_dock_left<br/>(defaults write com.apple.dock<br/>orientation left + killall Dock)"]
  Position["position_dock_window<br/>(center horizontally,<br/>6px from screen bottom)"]
  Install["install_native_dock_panel"]
  Fallback["configure_dock_window_interaction<br/>(used only if NSPanel install fails)"]
  Track["install_active_app_tracking"]
  Exit["RunEvent::Exit / ExitRequested<br/>→ restore_system_dock_bottom"]

  subgraph PanelDetails["NSPanel install details"]
    Alloc["alloc NSPanel<br/>(Borderless | NonactivatingPanel)"]
    Configure["floating, non-key,<br/>accepts mouse moved,<br/>does not hide on deactivate,<br/>level 20, clear bg"]
    Reparent["ns_view.removeFromSuperview()<br/>panel.setContentView(ns_view)"]
    OrderOut["original window orderOut"]
    OrderFront["panel.orderFrontRegardless"]
    Mouse["install_inactive_mouse_tracking"]
  end

  Run --> Sig
  Run --> Move
  Run --> Position
  Run --> Install
  Install --> Alloc
  Alloc --> Configure
  Configure --> Reparent
  Reparent --> OrderOut
  OrderOut --> OrderFront
  OrderFront --> Mouse
  Install -.->|on error| Fallback
  Run --> Track
  Run --> Exit
```

## Current Boundaries

```mermaid
flowchart TB
  Hardcoded["Hardcoded today"]
  Future["Not yet implemented"]

  Hardcoded --> H1["src/features/dock/config/dockItems.ts<br/>3 fixed items: VS Code, Chrome, localhost:3000"]
  Hardcoded --> H2["enableDockMagnification flag (currently false)"]
  Hardcoded --> H3["theme-toggle button is decorative<br/>(no theme switching wired up)"]
  Hardcoded --> H4["Auto-hide rule: visible only when<br/>Finder or this app is frontmost"]

  Future --> F1["Persistent storage for items<br/>(no JSON / Tauri store yet)"]
  Future --> F2["Add / edit / remove / reorder items in UI"]
  Future --> F3["Multiple workspaces"]
  Future --> F4["Folder and Link item types<br/>(types.ts only has app and url)"]
  Future --> F5["App picker / file picker / URL input"]
  Future --> F6["Drag-and-drop into the dock from a modal<br/>(dockbar web component already supports sortable)"]
```

## Type Model (current)

```ts
// src/features/dock/types.ts
type AppDockItem = { id: string; type: "app"; label: string; appPath: string };
type UrlDockItem = { id: string; type: "url"; label: string; url: string };
type DockItem    = AppDockItem | UrlDockItem;
type DockTitle   = { label: string; left: number; top: number };
```

Rust mirrors this with `DockItemRequest::App { app_path }` / `DockItemRequest::Url { url }` (camelCase serde tag).

## Key Files

| Concern | File |
| --- | --- |
| Entry | [src/main.tsx](src/main.tsx), [src/App.tsx](src/App.tsx) |
| Feature root | [src/features/dock/DockFeature.tsx](src/features/dock/DockFeature.tsx) |
| Item config | [src/features/dock/config/dockItems.ts](src/features/dock/config/dockItems.ts) |
| Components | [src/features/dock/components/](src/features/dock/components/) |
| Hooks | [src/features/dock/hooks/](src/features/dock/hooks/) |
| IPC client | [src/features/dock/services/dockService.ts](src/features/dock/services/dockService.ts) |
| Styling | [src/features/dock/dock.css](src/features/dock/dock.css), [src/App.css](src/App.css) |
| Web component types | [src/custom-elements.d.ts](src/custom-elements.d.ts) |
| Rust backend | [src-tauri/src/lib.rs](src-tauri/src/lib.rs) |
| Window config | [src-tauri/tauri.conf.json](src-tauri/tauri.conf.json) |
