import type { InstalledApp } from "../../dock/types";
import { AppTile } from "./AppTile";

type AppGridProps = {
  apps: InstalledApp[];
  appIcons: Record<string, string>;
  onLoadAppIcon: (appPath: string) => Promise<string | null>;
  onSelectApp: (app: InstalledApp) => void;
};

export function AppGrid({ apps, appIcons, onLoadAppIcon, onSelectApp }: AppGridProps) {
  if (apps.length === 0) {
    return <p className="add-item-empty">No apps found</p>;
  }

  return (
    <div className="add-item-grid">
      {apps.map((app) => (
        <AppTile
          key={app.path}
          app={app}
          iconUrl={appIcons[app.path]}
          onLoadIcon={onLoadAppIcon}
          onSelect={onSelectApp}
        />
      ))}
    </div>
  );
}
