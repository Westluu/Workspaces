import { useEffect, useRef } from "react";
import type { InstalledApp } from "../../dock/types";

type AppTileProps = {
  app: InstalledApp;
  iconUrl?: string;
  onLoadIcon: (appPath: string) => Promise<string | null>;
  onSelect: (app: InstalledApp) => void;
};

export function AppTile({ app, iconUrl, onLoadIcon, onSelect }: AppTileProps) {
  const tileRef = useRef<HTMLButtonElement>(null);
  const requestedIconRef = useRef(false);

  useEffect(() => {
    if (iconUrl || requestedIconRef.current) return;

    function requestIcon() {
      if (requestedIconRef.current) return;
      requestedIconRef.current = true;
      void onLoadIcon(app.path);
    }

    const tile = tileRef.current;
    if (!tile || typeof IntersectionObserver === "undefined") {
      requestIcon();
      return;
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (!entry?.isIntersecting) return;
        requestIcon();
        observer.disconnect();
      },
      {
        rootMargin: "120px",
        threshold: 0.01,
      },
    );

    observer.observe(tile);

    return () => observer.disconnect();
  }, [app.path, iconUrl, onLoadIcon]);

  return (
    <button
      ref={tileRef}
      className="app-tile"
      onClick={() => onSelect(app)}
      title={app.name}
    >
      <span className="app-tile-icon">
        {iconUrl ? (
          <img src={iconUrl} alt="" draggable={false} />
        ) : (
          <span className="app-tile-fallback" aria-hidden="true">
            {app.name
              .split(/\s+/)
              .map((w) => w[0])
              .join("")
              .slice(0, 2)
              .toUpperCase()}
          </span>
        )}
      </span>
      <span className="app-tile-label">{app.name}</span>
    </button>
  );
}
