import type { InstalledApp } from "../../dock/types";
import { AppGrid } from "../components/AppGrid";
import { SearchBar } from "../components/SearchBar";

type ApplicationsTabProps = {
  appIcons: Record<string, string>;
  isLoading: boolean;
  recentApps: InstalledApp[];
  searchQuery: string;
  suggestions: InstalledApp[];
  onAddApp: (app: InstalledApp) => void;
  onLoadAppIcon: (appPath: string) => Promise<string | null>;
  onSearchChange: (query: string) => void;
};

export function ApplicationsTab({
  appIcons,
  isLoading,
  recentApps,
  searchQuery,
  suggestions,
  onAddApp,
  onLoadAppIcon,
  onSearchChange,
}: ApplicationsTabProps) {
  return (
    <>
      <div className="add-item-search-row">
        <SearchBar
          placeholder="Search applications"
          value={searchQuery}
          onChange={onSearchChange}
        />
        <button className="add-item-view-toggle" aria-label="Toggle view">
          ☰
        </button>
      </div>
      {isLoading ? (
        <p className="add-item-empty">Loading applications…</p>
      ) : (
        <>
          <div className="add-item-section">
            <h3 className="add-item-section-title">Suggestions</h3>
            <AppGrid
              apps={suggestions}
              appIcons={appIcons}
              onLoadAppIcon={onLoadAppIcon}
              onSelectApp={onAddApp}
            />
          </div>
          {recentApps.length > 0 && (
            <div className="add-item-section">
              <h3 className="add-item-section-title">Recents</h3>
              <AppGrid
                apps={recentApps}
                appIcons={appIcons}
                onLoadAppIcon={onLoadAppIcon}
                onSelectApp={onAddApp}
              />
            </div>
          )}
        </>
      )}
    </>
  );
}
