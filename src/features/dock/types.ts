export type AppDockItem = {
  id: string;
  type: "app";
  label: string;
  appPath: string;
};

export type UrlDockItem = {
  id: string;
  type: "url";
  label: string;
  url: string;
};

export type FolderDockItem = {
  id: string;
  type: "folder";
  label: string;
  folderPath: string;
};

export type DockItem = AppDockItem | UrlDockItem | FolderDockItem;

/** Distributive Omit — preserves the discriminated union structure */
export type DistributiveOmit<T, K extends keyof T> = T extends unknown ? Omit<T, K> : never;

/** DockItem without `id` — use this instead of Omit<DockItem, "id"> */
export type DockItemInput = DistributiveOmit<DockItem, "id">;

export type DockTitle = {
  label: string;
  left: number;
  top: number;
};

export type InstalledApp = {
  name: string;
  path: string;
  bundleId: string;
  cachedIcon?: string;
};
