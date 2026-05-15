import type { FocusEvent, MouseEvent } from "react";
import type { DockItem } from "../types";
import { getFallbackIcon } from "../utils/iconUtils";

type DockItemButtonProps = {
  appIconUrl?: string;
  item: DockItem;
  itemLabel: string;
  onIconError: (itemId: string) => void;
  onOpen: (item: DockItem) => void;
  onTitleBlur: () => void;
  onTitleFocus: (label: string, e: FocusEvent<HTMLElement>) => void;
  onTitleMouse: (label: string, e: MouseEvent<HTMLElement>) => void;
};

export function DockItemButton({
  appIconUrl,
  item,
  itemLabel,
  onIconError,
  onOpen,
  onTitleBlur,
  onTitleFocus,
  onTitleMouse,
}: DockItemButtonProps) {
  return (
    <button
      className="dock-item"
      type="button"
      aria-label={`Open ${itemLabel}`}
      data-dock-label={itemLabel}
      onMouseEnter={(e) => onTitleMouse(itemLabel, e)}
      onMouseMove={(e) => onTitleMouse(itemLabel, e)}
      onFocus={(e) => onTitleFocus(itemLabel, e)}
      onBlur={onTitleBlur}
      onClick={() => onOpen(item)}
    >
      {appIconUrl ? (
        <img
          src={appIconUrl}
          alt=""
          draggable={false}
          onError={() => onIconError(item.id)}
        />
      ) : (
        <span
          className={`fallback-icon ${item.type === "url" ? "url-icon" : ""}`}
          aria-hidden="true"
        >
          {getFallbackIcon(item)}
        </span>
      )}
    </button>
  );
}
