import { useState } from "react";
import type { DockItem, DockItemInput, InstalledApp } from "../dock/types";
import { ApplicationsTab } from "./tabs/ApplicationsTab";
import { FoldersTab } from "./tabs/FoldersTab";
import { LinksTab } from "./tabs/LinksTab";
import "./add-item.css";

type AddItemModalProps = {
  dockItems: DockItem[];
  installedApps: InstalledApp[];
  appIcons: Record<string, string>;
  isLoading: boolean;
  onAddItem: (item: DockItemInput) => void;
  onClose: () => void;
  onLoadAppIcon: (appPath: string) => Promise<string | null>;
};

type TabId = "applications" | "folders" | "links";

export function AddItemModal({
  dockItems,
  installedApps,
  appIcons,
  isLoading,
  onAddItem,
  onClose,
  onLoadAppIcon,
}: AddItemModalProps) {
  const [activeTab, setActiveTab] = useState<TabId>("applications");
  const [searchQuery, setSearchQuery] = useState("");

  const dockAppPaths = new Set(
    dockItems
      .filter((item): item is DockItem & { type: "app" } => item.type === "app")
      .map((item) => item.appPath),
  );

  const filteredApps = installedApps.filter(
    (app) =>
      app.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      app.bundleId.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  const suggestions = filteredApps.filter((app) => !dockAppPaths.has(app.path));
  const recentApps = suggestions.slice(-3).reverse();
  const suggestionApps = suggestions.filter(
    (app) => !recentApps.some((r) => r.path === app.path),
  );

  function handleAddApp(app: InstalledApp) {
    onAddItem({
      type: "app",
      label: app.name,
      appPath: app.path,
    });
  }

  return (
    <div className="add-item-modal">
      <div className="add-item-header">
        <span className="add-item-drag-handle" />
        <h2 className="add-item-title">Add Item</h2>
      </div>

      <div className="add-item-tabs">
        <button
          className={`add-item-tab ${activeTab === "applications" ? "active" : ""}`}
          onClick={() => setActiveTab("applications")}
        >
          Applications
        </button>
        <button
          className={`add-item-tab ${activeTab === "folders" ? "active" : ""}`}
          onClick={() => setActiveTab("folders")}
        >
          Folders
        </button>
        <button
          className={`add-item-tab ${activeTab === "links" ? "active" : ""}`}
          onClick={() => setActiveTab("links")}
        >
          Links
        </button>
      </div>

      <div className="add-item-body">
        {activeTab === "applications" && (
          <ApplicationsTab
            appIcons={appIcons}
            isLoading={isLoading}
            recentApps={recentApps}
            searchQuery={searchQuery}
            suggestions={suggestionApps}
            onAddApp={handleAddApp}
            onLoadAppIcon={onLoadAppIcon}
            onSearchChange={setSearchQuery}
          />
        )}
        {activeTab === "folders" && <FoldersTab onAddItem={onAddItem} />}
        {activeTab === "links" && <LinksTab onAddItem={onAddItem} />}
      </div>

      <div className="add-item-footer">
        <span className="add-item-hint">Drag an item into the dock</span>
        <button className="add-item-done" onClick={onClose}>
          Done
        </button>
      </div>
    </div>
  );
}
