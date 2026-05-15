import { useState } from "react";
import { dockItems, enableDockMagnification } from "./config/dockItems";
import { Dock } from "./components/Dock";
import { DockTitle } from "./components/DockTitle";
import { ErrorToast } from "./components/ErrorToast";
import { useDockIcons } from "./hooks/useDockIcons";
import { useDockLabels } from "./hooks/useDockLabels";
import { useDockTitle } from "./hooks/useDockTitle";
import { useNativeDockMouseTracking } from "./hooks/useNativeDockMouseTracking";
import { openDockItem } from "./services/dockService";
import type { DockItem } from "./types";
import "./dock.css";

export function DockFeature() {
  const [error, setError] = useState<string | null>(null);
  const { appLabels } = useDockLabels(dockItems);
  const { appIconUrls, removeIcon } = useDockIcons(dockItems);
  const {
    clearDockTitle,
    dockTitle,
    handleTitleFocus,
    handleTitleMouse,
    showDockTitle,
  } = useDockTitle();

  useNativeDockMouseTracking({
    clearDockTitle,
    showDockTitle,
  });

  async function handleOpenItem(item: DockItem) {
    try {
      setError(null);
      await openDockItem(item);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main className="playground">
      {dockTitle && <DockTitle dockTitle={dockTitle} />}
      <Dock
        appIconUrls={appIconUrls}
        appLabels={appLabels}
        dockItems={dockItems}
        enableMagnification={enableDockMagnification}
        onClearTitle={clearDockTitle}
        onIconError={removeIcon}
        onOpenItem={handleOpenItem}
        onTitleFocus={handleTitleFocus}
        onTitleMouse={handleTitleMouse}
      />
      {error && <ErrorToast error={error} />}
    </main>
  );
}
