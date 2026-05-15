import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./App.css";

type AppDockItem = {
  id: string;
  type: "app";
  label: string;
  appPath: string;
};

type UrlDockItem = {
  id: string;
  type: "url";
  label: string;
  url: string;
};

type DockItem = AppDockItem | UrlDockItem;

const workspace = {
  name: "Boba Frontend",
  dockItems: [
    {
      id: "vscode",
      type: "app",
      label: "VS Code",
      appPath: "/Applications/Visual Studio Code.app",
    },
    {
      id: "chrome",
      type: "app",
      label: "Chrome",
      appPath: "/Applications/Google Chrome.app",
    },
    {
      id: "localhost",
      type: "url",
      label: "Localhost",
      url: "http://localhost:3000",
    },
  ] satisfies DockItem[],
};

function pngBytesToDataUrl(iconBytes: number[]) {
  const bytes = new Uint8Array(iconBytes);
  let binary = "";

  for (let index = 0; index < bytes.length; index += 8192) {
    binary += String.fromCharCode(...bytes.slice(index, index + 8192));
  }

  return `data:image/png;base64,${btoa(binary)}`;
}

function getFallbackIcon(item: DockItem) {
  if (item.type === "url") {
    return "LH";
  }

  return item.label
    .split(/\s+/)
    .map((word) => word[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

function App() {
  const [error, setError] = useState<string | null>(null);
  const [appIconUrls, setAppIconUrls] = useState<Record<string, string>>({});
  const [activeLabel, setActiveLabel] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    const appItems = workspace.dockItems.filter((item) => item.type === "app");

    appItems.forEach((item) => {
      invoke<number[]>("get_app_icon", {
        appPath: item.appPath,
      })
        .then((iconBytes) => {
          if (isMounted) {
            setAppIconUrls((currentIcons) => ({
              ...currentIcons,
              [item.id]: pngBytesToDataUrl(iconBytes),
            }));
          }
        })
        .catch(() => {
          if (isMounted) {
            setAppIconUrls((currentIcons) => {
              const { [item.id]: _removedIcon, ...remainingIcons } =
                currentIcons;
              return remainingIcons;
            });
          }
        });
    });

    return () => {
      isMounted = false;
    };
  }, []);

  async function openDockItem(item: DockItem) {
    try {
      setError(null);
      await invoke("open_dock_item", {
        item,
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
      {activeLabel && <span className="dock-tooltip">{activeLabel}</span>}
      <section
        className="dock"
        onMouseDown={startDrag}
        onMouseLeave={() => setActiveLabel(null)}
      >
        {workspace.dockItems.map((item) => {
          const appIconUrl = appIconUrls[item.id];

          return (
            <button
              key={item.id}
              className="dock-item"
              aria-label={`Open ${item.label}`}
              onMouseDown={(e) => e.stopPropagation()}
              onMouseEnter={() => setActiveLabel(item.label)}
              onFocus={() => setActiveLabel(item.label)}
              onBlur={() => setActiveLabel(null)}
              onClick={() => openDockItem(item)}
            >
              <span
                className={`app-icon ${appIconUrl ? "app-icon-real" : ""} ${
                  item.type === "url" ? "url-icon" : ""
                }`}
                aria-hidden="true"
              >
                {appIconUrl ? (
                  <img
                    className="app-icon-image"
                    src={appIconUrl}
                    alt=""
                    draggable={false}
                    onError={() =>
                      setAppIconUrls((currentIcons) => {
                        const { [item.id]: _removedIcon, ...remainingIcons } =
                          currentIcons;
                        return remainingIcons;
                      })
                    }
                  />
                ) : (
                  getFallbackIcon(item)
                )}
              </span>
              <span className="running-dot" aria-hidden="true" />
            </button>
          );
        })}
        <span className="dock-divider" aria-hidden="true" />
        <button
          className="dock-item add-dock-item"
          aria-label="Add item"
          onMouseDown={(e) => e.stopPropagation()}
          onMouseEnter={() => setActiveLabel("Add item")}
          onFocus={() => setActiveLabel("Add item")}
          onBlur={() => setActiveLabel(null)}
          onClick={() => setError("Add item UI is next")}
        >
          <span className="app-icon add-icon" aria-hidden="true">
            +
          </span>
        </button>
      </section>
      {error && <span className="error-toast">{error}</span>}
    </main>
  );
}

export default App;
