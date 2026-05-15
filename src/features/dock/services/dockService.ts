import { invoke } from "@tauri-apps/api/core";
import type { DockItem } from "../types";

export function openDockItem(item: DockItem) {
  return invoke("open_dock_item", {
    item,
  });
}

export function getAppIcon(appPath: string) {
  return invoke<number[]>("get_app_icon", {
    appPath,
  });
}

export function getAppDisplayName(appPath: string) {
  return invoke<string>("get_app_display_name", {
    appPath,
  });
}
