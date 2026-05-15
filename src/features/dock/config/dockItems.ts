import type { DockItem } from "../types";

export const dockItems = [
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
] satisfies DockItem[];

export const enableDockMagnification = false;
