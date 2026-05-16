import type { DockItem } from "../types";

export function pngBytesToDataUrl(iconBytes: number[]) {
  const bytes = new Uint8Array(iconBytes);
  let binary = "";

  for (let index = 0; index < bytes.length; index += 8192) {
    binary += String.fromCharCode(...bytes.slice(index, index + 8192));
  }

  return `data:image/png;base64,${btoa(binary)}`;
}

export function getFallbackIcon(item: DockItem) {
  if (item.type === "url") {
    return "LH";
  }

  if (item.type === "folder") {
    return "📁";
  }

  return item.label
    .split(/\s+/)
    .map((word) => word[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}
