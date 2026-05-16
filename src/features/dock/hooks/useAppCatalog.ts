import { useCallback, useEffect, useRef, useState } from "react";
import type { InstalledApp } from "../types";
import {
  getAppIcon,
  listInstalledApps,
  refreshInstalledApps,
} from "../services/dockService";
import { pngBytesToDataUrl } from "../utils/iconUtils";

export type AppCatalogStatus = "loading" | "ready" | "refreshing" | "error";

let initialAppsPromise: Promise<InstalledApp[]> | null = null;

function getInitialApps() {
  if (!initialAppsPromise) {
    initialAppsPromise = listInstalledApps().catch((e) => {
      initialAppsPromise = null;
      throw e;
    });
  }

  return initialAppsPromise;
}

function collectCachedIcons(nextApps: InstalledApp[]) {
  const cachedIcons: Record<string, string> = {};

  for (const app of nextApps) {
    if (!app.cachedIcon) continue;
    cachedIcons[app.path] = app.cachedIcon;
  }

  return cachedIcons;
}

export function useAppCatalog() {
  const [apps, setApps] = useState<InstalledApp[]>([]);
  const [icons, setIcons] = useState<Record<string, string>>({});
  const [status, setStatus] = useState<AppCatalogStatus>("loading");
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const iconsRef = useRef<Record<string, string>>({});
  const iconPromisesRef = useRef<Record<string, Promise<string | null>>>({});

  const loadIcon = useCallback(async (appPath: string) => {
    const cached = iconsRef.current[appPath];
    if (cached) return cached;

    const pending = iconPromisesRef.current[appPath];
    if (pending) return pending;

    const startedAt = performance.now();
    const promise = getAppIcon(appPath)
      .then((iconBytes) => {
        const dataUrl = pngBytesToDataUrl(iconBytes);
        if (!mountedRef.current) return dataUrl;

        setIcons((currentIcons) => {
          if (currentIcons[appPath]) return currentIcons;
          const nextIcons = { ...currentIcons, [appPath]: dataUrl };
          iconsRef.current = nextIcons;
          return nextIcons;
        });

        console.debug(
          `[app-catalog] icon loaded path="${appPath}" duration_ms=${Math.round(
            performance.now() - startedAt,
          )}`,
        );

        return dataUrl;
      })
      .catch((e) => {
        console.debug(
          `[app-catalog] icon failed path="${appPath}" duration_ms=${Math.round(
            performance.now() - startedAt,
          )} error="${String(e)}"`,
        );
        return null;
      })
      .finally(() => {
        delete iconPromisesRef.current[appPath];
      });

    iconPromisesRef.current[appPath] = promise;
    return promise;
  }, []);

  const loadApps = useCallback(async (mode: "initial" | "refresh") => {
    const startedAt = performance.now();

    try {
      setStatus((currentStatus) =>
        mode === "refresh" && currentStatus !== "loading" ? "refreshing" : "loading",
      );
      setError(null);

      const nextApps = mode === "refresh" ? await refreshInstalledApps() : await getInitialApps();
      if (mode === "refresh") {
        initialAppsPromise = Promise.resolve(nextApps);
      }

      if (!mountedRef.current) return nextApps;

      const cachedIcons = collectCachedIcons(nextApps);
      setApps(nextApps);
      if (Object.keys(cachedIcons).length > 0) {
        setIcons((currentIcons) => {
          let changed = false;
          const mergedIcons = { ...currentIcons };

          for (const [appPath, iconUrl] of Object.entries(cachedIcons)) {
            if (mergedIcons[appPath]) continue;
            mergedIcons[appPath] = iconUrl;
            changed = true;
          }

          if (!changed) return currentIcons;
          iconsRef.current = mergedIcons;
          return mergedIcons;
        });

        console.debug(
          `[app-catalog] cached icons attached count=${Object.keys(cachedIcons).length}`,
        );
      }
      setStatus("ready");

      console.debug(
        `[app-catalog] ${mode} app list loaded count=${
          nextApps.length
        } duration_ms=${Math.round(performance.now() - startedAt)}`,
      );

      return nextApps;
    } catch (e) {
      const message = String(e);
      if (mountedRef.current) {
        setError(message);
        setStatus("error");
      }

      console.debug(
        `[app-catalog] ${mode} app list failed duration_ms=${Math.round(
          performance.now() - startedAt,
        )} error="${message}"`,
      );

      return [];
    }
  }, []);

  const refresh = useCallback(async () => {
    return loadApps("refresh");
  }, [loadApps]);

  useEffect(() => {
    mountedRef.current = true;

    void loadApps("initial");

    return () => {
      mountedRef.current = false;
    };
  }, [loadApps]);

  return {
    apps,
    icons,
    status,
    error,
    refresh,
    loadIcon,
  };
}
