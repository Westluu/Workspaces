import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

const workspace = {
  name: "Boba Frontend",
  dockItems: [
    {
      id: "vscode",
      label: "VS Code",
      appPath: "/Applications/Visual Studio Code.app",
    },
  ],
};

function pngBytesToDataUrl(iconBytes: number[]) {
  const bytes = new Uint8Array(iconBytes);
  let binary = "";

  for (let index = 0; index < bytes.length; index += 8192) {
    binary += String.fromCharCode(...bytes.slice(index, index + 8192));
  }

  return `data:image/png;base64,${btoa(binary)}`;
}

function App() {
  const [error, setError] = useState<string | null>(null);
  const [appIconUrl, setAppIconUrl] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;

    invoke<number[]>("get_app_icon", {
      appPath: workspace.dockItems[0].appPath,
    })
      .then((iconBytes) => {
        if (isMounted) {
          setAppIconUrl(pngBytesToDataUrl(iconBytes));
        }
      })
      .catch(() => {
        if (isMounted) {
          setAppIconUrl(null);
        }
      });

    return () => {
      isMounted = false;
    };
  }, []);

  async function openVSCode() {
    try {
      setError(null);
      await invoke("open_app", {
        appPath: workspace.dockItems[0].appPath,
      });
    } catch (e) {
      setError(String(e));
    }
  }

  function startDrag() {
    getCurrentWindow().startDragging();
  }

  return (
    <main className="dock-shell">
      <section className="dock" onMouseDown={startDrag}>
        <span className="workspace-label">{workspace.name}</span>
        <button
          className="dock-item"
          aria-label={`Open ${workspace.dockItems[0].label}`}
          title={workspace.dockItems[0].label}
          onMouseDown={(e) => e.stopPropagation()}
          onClick={openVSCode}
        >
          <span
            className={`app-icon ${appIconUrl ? "app-icon-real" : ""}`}
            aria-hidden="true"
          >
            {appIconUrl ? (
              <img
                className="app-icon-image"
                src={appIconUrl}
                alt=""
                draggable={false}
                onError={() => setAppIconUrl(null)}
              />
            ) : (
              "VS"
            )}
          </span>
          <span className="app-label">{workspace.dockItems[0].label}</span>
        </button>
      </section>
      {error && <span className="error-toast">{error}</span>}
    </main>
  );
}

export default App;
