import { useAppCatalog } from "./useAppCatalog";

export function useInstalledAppsCache() {
  const catalog = useAppCatalog();

  return {
    installedApps: catalog.apps,
    appIcons: catalog.icons,
    isLoading: catalog.status === "loading",
  };
}
