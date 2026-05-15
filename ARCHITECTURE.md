# Workspace Dock Architecture

This document describes the current MVP slice: one floating Tauri dock with one hardcoded workspace and three dock items.

## High-Level App Shape

```mermaid
flowchart TB
  User[User]

  subgraph TauriApp["Tauri Desktop App"]
    subgraph Window["Floating Dock Window"]
      Config["tauri.conf.json<br/>470x112, transparent,<br/>borderless, always-on-top"]
      WebView["WebView"]
    end

    subgraph Frontend["React Frontend"]
      App["App.tsx"]
      Styles["App.css"]
      Workspace["Hardcoded workspace data<br/>Boba Frontend"]
      DockItems["Dock items<br/>VS Code, Chrome, Localhost"]
      IconState["appIconUrls state"]
      ErrorState["error state"]
    end

    subgraph Bridge["Tauri IPC Bridge"]
      Invoke["invoke(...)"]
      WindowApi["getCurrentWindow().startDragging()"]
    end

    subgraph RustBackend["Rust Backend"]
      OpenItem["open_dock_item(item)"]
      GetIcon["get_app_icon(app_path)"]
      IconHelpers["Icon helpers<br/>read Info.plist<br/>convert .icns to PNG"]
    end
  end

  subgraph MacOS["macOS System"]
    OpenCmd["open"]
    Plutil["plutil"]
    Sips["sips"]
    AppBundles["App bundles<br/>VS Code, Chrome"]
    Localhost["http://localhost:3000"]
    TempPng["Temp icon PNG<br/>workspace-dock-icons"]
  end

  User --> Window
  Config --> Window
  WebView --> App
  App --> Styles
  App --> Workspace
  Workspace --> DockItems
  DockItems --> Invoke
  App --> WindowApi
  Invoke --> OpenItem
  Invoke --> GetIcon
  OpenItem --> OpenCmd
  OpenCmd --> AppBundles
  OpenCmd --> Localhost
  GetIcon --> IconHelpers
  IconHelpers --> Plutil
  IconHelpers --> Sips
  Plutil --> AppBundles
  Sips --> TempPng
  GetIcon --> IconState
  OpenItem --> ErrorState
```

## Frontend Component And State Flow

```mermaid
flowchart LR
  App["App component"]
  Workspace["workspace constant<br/>name + dockItems"]
  IconState["appIconUrls<br/>Record&lt;itemId, data URL&gt;"]
  ErrorState["error<br/>null or message"]
  DockShell["main.dock-shell"]
  Dock["section.dock"]
  WorkspaceChip["span.workspace-label"]
  Map["dockItems.map(...)"]
  Button["button.dock-item"]
  Icon["span.app-icon"]
  Img["img.app-icon-image"]
  Fallback["Fallback text: VS"]
  Label["span.app-label"]
  Toast["span.error-toast"]

  App --> Workspace
  App --> IconState
  App --> ErrorState
  App --> DockShell
  DockShell --> Dock
  Dock --> WorkspaceChip
  Dock --> Map
  Map --> Button
  Button --> Icon
  Button --> Label
  IconState --> Icon
  Icon -->|when loaded| Img
  Icon -->|when missing or failed| Fallback
  ErrorState -->|when set| Toast
```

## App Icon Loading Data Flow

```mermaid
sequenceDiagram
  participant React as React App.tsx
  participant IPC as Tauri invoke
  participant Rust as Rust get_app_icon
  participant Plutil as macOS plutil
  participant Sips as macOS sips
  participant FS as Temp PNG Cache

  React->>IPC: invoke("get_app_icon", { appPath })
  IPC->>Rust: get_app_icon(app_path)
  Rust->>Plutil: read CFBundleIconFile from Contents/Info.plist
  Plutil-->>Rust: icon name, e.g. Code.icns
  Rust->>Sips: convert .icns to PNG at 256px
  Sips->>FS: write workspace-dock-icons/*.png
  Rust->>FS: read PNG bytes
  Rust-->>IPC: Vec<u8>
  IPC-->>React: number[]
  React->>React: convert bytes to base64 data URL
  React->>React: set appIconUrl
```

## Launch Data Flow

```mermaid
sequenceDiagram
  participant User
  participant React as React App.tsx
  participant IPC as Tauri invoke
  participant Rust as Rust open_dock_item
  participant Mac as macOS open
  participant Target as App or URL Target

  User->>React: click dock item
  React->>React: clear error state
  React->>IPC: invoke("open_dock_item", { item })
  IPC->>Rust: open_dock_item(item)
  Rust->>Mac: open appPath or url
  Mac->>Target: launch/focus app or open URL
  Rust-->>React: Ok or error string
  React->>React: show error toast only on failure
```

## Drag Data Flow

```mermaid
sequenceDiagram
  participant User
  participant Dock as section.dock
  participant Button as button.dock-item
  participant WindowApi as Tauri Window API
  participant Window as Floating Dock Window

  User->>Dock: mouse down on dock background or workspace chip
  Dock->>WindowApi: getCurrentWindow().startDragging()
  WindowApi->>Window: move native window

  User->>Button: mouse down on dock item
  Button->>Button: stopPropagation()
  Button--xDock: drag does not start
```

## Current Boundaries

```mermaid
flowchart TB
  Hardcoded["Hardcoded today"]
  Future["Future data layer"]

  Hardcoded --> H1["Workspace name"]
  Hardcoded --> H2["Dock items"]
  Hardcoded --> H3["App paths and localhost URL"]

  Future --> F1["JSON workspace store"]
  Future --> F2["Workspace manager"]
  Future --> F3["Add/edit/remove/reorder items"]
  Future --> F4["Open All"]
```

Current storage is still hardcoded in `src/App.tsx`. The next architectural step is to move workspace data behind a store command or frontend store module before adding workspace creation and item management.
