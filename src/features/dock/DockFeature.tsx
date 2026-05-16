import { useRef, useState } from "react";
import { enableDockMagnification } from "./config/dockItems";
import { Dock } from "./components/Dock";
import { DockTitle } from "./components/DockTitle";
import { ErrorToast } from "./components/ErrorToast";
import { useDockIcons } from "./hooks/useDockIcons";
import { useDockLabels } from "./hooks/useDockLabels";
import { useDockTitle } from "./hooks/useDockTitle";
import { useDockVisibility } from "./hooks/useDockVisibility";
import { useDockItemsStore } from "./hooks/useDockItemsStore";
import { useDockEditMode } from "./hooks/useDockEditMode";
import { useNativeDockMouseTracking } from "./hooks/useNativeDockMouseTracking";
import { useAppCatalog } from "./hooks/useAppCatalog";
import { openDockItem, resizeDockPanel, getDockPanelFrame } from "./services/dockService";
import { AddItemModal } from "../add-item/AddItemModal";
import type { DockItem, DockItemInput } from "./types";
import "./dock.css";

export function DockFeature() {
  const [error, setError] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);
  const { items, isLoaded, addItem, removeItem } = useDockItemsStore();
  const {
    apps: installedApps,
    icons: modalAppIcons,
    loadIcon: loadModalAppIcon,
    status: appCatalogStatus,
  } = useAppCatalog();
  const appsLoading = appCatalogStatus === "loading";
  const { isEditing, enterEditMode, exitEditMode } = useDockEditMode();
  const { appLabels } = useDockLabels(items);
  const { appIconUrls, removeIcon } = useDockIcons(items);
  const {
    clearDockTitle,
    dockTitle,
    handleTitleFocus,
    handleTitleMouse,
    showDockTitle,
  } = useDockTitle();

  // Ref on the .playground <main> element — once the modal mounts we measure the
  // rendered modal and dock sizes to expand the native panel.
  const playgroundRef = useRef<HTMLElement>(null);

  // Saves the panel's original frame before we expand it so we can restore
  // it exactly when the modal closes.
  const originalWindowRef = useRef<{
    width: number;
    height: number;
    x: number;
    y: number;
  } | null>(null);

  useNativeDockMouseTracking({
    clearDockTitle,
    showDockTitle,
  });
  const { isDockHidden } = useDockVisibility({
    clearDockTitle,
  });

  async function expandWindowForModal() {
    const el = playgroundRef.current;
    if (!el) return;

    // Get the current panel frame (macOS screen coordinates, origin is bottom-left).
    const frame = await getDockPanelFrame();

    // Save original for restore.
    originalWindowRef.current = {
      width: frame.width,
      height: frame.height,
      x: frame.x,
      y: frame.y,
    };

    // Read actual sizes from the DOM instead of relying on scrollHeight timing.
    // scrollHeight is unreliable here because the modal's content (app icons) loads
    // asynchronously — at measurement time the body may only have min-height rendered,
    // so we'd size the panel too small and the dock would be pushed off-screen once
    // the content fills in.
    const modalEl = el.querySelector(".add-item-modal") as HTMLElement | null;
    const dockEl = el.querySelector(".dock") as HTMLElement | null;

    // The modal's CSS max-height tells us the maximum space it will ever occupy,
    // regardless of how much content has loaded so far.
    const modalMax = modalEl
      ? parseFloat(getComputedStyle(modalEl).maxHeight) || 480
      : 480;
    const dockH = dockEl ? dockEl.offsetHeight : 0;

    // 8px top padding + modal max-height + 8px gap (margin-bottom on modal) + dock + 8px bottom padding
    const neededHeight = 8 + modalMax + 8 + dockH + 8;
    if (neededHeight <= frame.height) return;

    // In macOS screen coordinates y=0 is the bottom of the screen and increases
    // upward. The frame origin is the bottom-left corner. Keeping x and y the
    // same while increasing height expands the panel upward — the bottom edge
    // stays pinned at the dock's usual position.
    await resizeDockPanel(frame.width, neededHeight, frame.x, frame.y);
  }

  async function restoreWindow() {
    const orig = originalWindowRef.current;
    if (!orig) return;
    await resizeDockPanel(orig.width, orig.height, orig.x, orig.y);
    originalWindowRef.current = null;
  }

  async function handleOpenItem(item: DockItem) {
    if (isEditing) return;
    try {
      setError(null);
      await openDockItem(item);
    } catch (e) {
      setError(String(e));
    }
  }

  function handleAddItemClick() {
    const startedAt = performance.now();
    setShowModal(true);
    // Measure after React has inserted the modal into the DOM.
    requestAnimationFrame(() => {
      console.debug(
        `[app-catalog] modal open rendered duration_ms=${Math.round(
          performance.now() - startedAt,
        )}`,
      );
      expandWindowForModal();
    });
  }

  function handleCloseModal() {
    setShowModal(false);
    exitEditMode();
    restoreWindow();
  }

  function handleAddItem(item: DockItemInput) {
    addItem(item);
  }

  function handleRemoveItem(itemId: string) {
    removeItem(itemId);
  }

  function handleContextMenu(e: React.MouseEvent) {
    e.preventDefault();
    if (!isEditing) {
      enterEditMode();
    }
  }

  if (!isLoaded) return null;

  return (
    <main
      ref={playgroundRef}
      className={`playground${isDockHidden ? " dock-hidden" : ""}`}
      onContextMenu={handleContextMenu}
    >
      {!isDockHidden && dockTitle && <DockTitle dockTitle={dockTitle} />}
      {/* Modal sits above the dock as a normal flex child — no positioning tricks. */}
      {isEditing && showModal && (
        <AddItemModal
          dockItems={items}
          installedApps={installedApps}
          appIcons={modalAppIcons}
          isLoading={appsLoading}
          onAddItem={handleAddItem}
          onClose={handleCloseModal}
          onLoadAppIcon={loadModalAppIcon}
        />
      )}
      <Dock
        appIconUrls={appIconUrls}
        appLabels={appLabels}
        dockItems={items}
        editing={isEditing}
        enableMagnification={enableDockMagnification}
        onAddItemClick={handleAddItemClick}
        onClearTitle={clearDockTitle}
        onIconError={removeIcon}
        onOpenItem={handleOpenItem}
        onRemoveItem={handleRemoveItem}
        onTitleFocus={handleTitleFocus}
        onTitleMouse={handleTitleMouse}
      />
      {error && <ErrorToast error={error} />}
    </main>
  );
}
