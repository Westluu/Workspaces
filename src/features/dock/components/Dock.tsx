import "dockbar";
import type { FocusEvent, MouseEvent } from "react";
import type { DockItem } from "../types";
import { DockItemButton } from "./DockItemButton";

type DockProps = {
  appIconUrls: Record<string, string>;
  appLabels: Record<string, string>;
  dockItems: DockItem[];
  editing: boolean;
  enableMagnification: boolean;
  onAddItemClick: () => void;
  onClearTitle: () => void;
  onIconError: (itemId: string) => void;
  onOpenItem: (item: DockItem) => void;
  onRemoveItem: (itemId: string) => void;
  onTitleFocus: (label: string, e: FocusEvent<HTMLElement>) => void;
  onTitleMouse: (label: string, e: MouseEvent<HTMLElement>) => void;
};

export function Dock({
  appIconUrls,
  appLabels,
  dockItems,
  editing,
  enableMagnification,
  onAddItemClick,
  onClearTitle,
  onIconError,
  onOpenItem,
  onRemoveItem,
  onTitleFocus,
  onTitleMouse,
}: DockProps) {
  return (
    <div className={`dock ${editing ? "editing" : ""}`}>
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
            {editing ? (
              <div className="dock-item-wrapper">
                <button
                  className="remove-badge"
                  aria-label={`Remove ${appLabels[item.id] ?? item.label}`}
                  onClick={() => onRemoveItem(item.id)}
                >
                  −
                </button>
                <DockItemButton
                  appIconUrl={appIconUrls[item.id]}
                  item={item}
                  itemLabel={appLabels[item.id] ?? item.label}
                  onIconError={onIconError}
                  onOpen={onOpenItem}
                  onTitleBlur={onClearTitle}
                  onTitleFocus={onTitleFocus}
                  onTitleMouse={onTitleMouse}
                />
              </div>
            ) : (
              <DockItemButton
                appIconUrl={appIconUrls[item.id]}
                item={item}
                itemLabel={appLabels[item.id] ?? item.label}
                onIconError={onIconError}
                onOpen={onOpenItem}
                onTitleBlur={onClearTitle}
                onTitleFocus={onTitleFocus}
                onTitleMouse={onTitleMouse}
              />
            )}
          </dock-item>
        ))}
        <dock-item>
          <button
            className="add-item-slot"
            aria-label="Add item"
            data-dock-label="Add Item"
            onClick={onAddItemClick}
            onMouseEnter={(e) => onTitleMouse("Add Item", e)}
            onMouseMove={(e) => onTitleMouse("Add Item", e)}
            onFocus={(e) => onTitleFocus("Add Item", e)}
            onBlur={onClearTitle}
          >
            +
          </button>
        </dock-item>
      </dock-wrapper>
    </div>
  );
}
