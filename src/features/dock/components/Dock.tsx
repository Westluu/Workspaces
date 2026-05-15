import "dockbar";
import type { FocusEvent, MouseEvent } from "react";
import type { DockItem } from "../types";
import { DockItemButton } from "./DockItemButton";

type DockProps = {
  appIconUrls: Record<string, string>;
  dockItems: DockItem[];
  enableMagnification: boolean;
  onClearTitle: () => void;
  onIconError: (itemId: string) => void;
  onOpenItem: (item: DockItem) => void;
  onTitleFocus: (label: string, e: FocusEvent<HTMLElement>) => void;
  onTitleMouse: (label: string, e: MouseEvent<HTMLElement>) => void;
};

export function Dock({
  appIconUrls,
  dockItems,
  enableMagnification,
  onClearTitle,
  onIconError,
  onOpenItem,
  onTitleFocus,
  onTitleMouse,
}: DockProps) {
  return (
    <div className="dock">
      <div className="light-border-frame" aria-hidden="true" />
      <dock-wrapper
        size="40"
        padding="8"
        gap="8"
        max-scale="2"
        max-range="200"
        disabled={!enableMagnification}
        sortable
        direction="horizontal"
        position="bottom"
        onMouseLeave={onClearTitle}
      >
        {dockItems.map((item) => (
          <dock-item key={item.id}>
            <DockItemButton
              appIconUrl={appIconUrls[item.id]}
              item={item}
              onIconError={onIconError}
              onOpen={onOpenItem}
              onTitleBlur={onClearTitle}
              onTitleFocus={onTitleFocus}
              onTitleMouse={onTitleMouse}
            />
          </dock-item>
        ))}
        <dock-item>
          <button
            className="dock-item theme-toggle"
            aria-label="Toggle theme"
            data-dock-label="Dark"
            onMouseEnter={(e) => onTitleMouse("Dark", e)}
            onMouseMove={(e) => onTitleMouse("Dark", e)}
            onFocus={(e) => onTitleFocus("Dark", e)}
            onBlur={onClearTitle}
          >
            <span aria-hidden="true">☾</span>
          </button>
        </dock-item>
      </dock-wrapper>
    </div>
  );
}
