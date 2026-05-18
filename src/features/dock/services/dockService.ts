import { invoke } from "@tauri-apps/api/core";
import type { DockItem, InstalledApp } from "../types";

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

export function getDockItems() {
  return invoke<string>("get_dock_items");
}

export function setDockItems(items: string) {
  return invoke("set_dock_items", { items });
}

export function listInstalledApps() {
  return invoke<InstalledApp[]>("list_installed_apps");
}

export function refreshInstalledApps() {
  return invoke<InstalledApp[]>("refresh_installed_apps");
}

export async function resizeDockPanel(
  width: number,
  height: number,
  x: number,
  y: number
): Promise<void> {
  await invoke("resize_dock_panel", { width, height, x, y });
}

export async function getDockPanelFrame(): Promise<{
  width: number;
  height: number;
  x: number;
  y: number;
}> {
  const [width, height, x, y] = await invoke<[number, number, number, number]>(
    "get_dock_panel_frame"
  );
  return { width, height, x, y };
}

export async function focusDockPanel(): Promise<void> {
  await invoke("focus_dock_panel");
}
