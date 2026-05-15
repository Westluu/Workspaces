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

export type DockItem = AppDockItem | UrlDockItem;

export type DockTitle = {
  label: string;
  left: number;
  top: number;
};
