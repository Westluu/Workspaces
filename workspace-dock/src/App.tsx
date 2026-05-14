import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

function App() {
  const [error, setError] = useState<string | null>(null);

  async function openVSCode() {
    try {
      setError(null);
      await invoke("open_app", {
        appPath: "/Applications/Visual Studio Code.app",
      });
    } catch (e) {
      setError(String(e));
    }
  }

  function startDrag() {
    getCurrentWindow().startDragging();
  }

  return (
    <main className="dock" onMouseDown={startDrag}>
      <span className="workspace-label">Boba Frontend</span>
      <button
        className="app-button"
        onMouseDown={(e) => e.stopPropagation()}
        onClick={openVSCode}
      >
        VS Code
      </button>
      {error && <span className="error-toast">{error}</span>}
    </main>
  );
}

export default App;
