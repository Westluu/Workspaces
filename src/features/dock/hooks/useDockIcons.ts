import { useEffect, useState } from "react";
import type { DockItem } from "../types";
import { getAppIcon } from "../services/dockService";
import { pngBytesToDataUrl } from "../utils/iconUtils";

export function useDockIcons(dockItems: DockItem[]) {
  const [appIconUrls, setAppIconUrls] = useState<Record<string, string>>({});

  useEffect(() => {
    let isMounted = true;
    const appItems = dockItems.filter((item) => item.type === "app");

    appItems.forEach((item) => {
      getAppIcon(item.appPath)
        .then((iconBytes) => {
          if (isMounted) {
            setAppIconUrls((currentIcons) => ({
              ...currentIcons,
              [item.id]: pngBytesToDataUrl(iconBytes),
            }));
          }
        })
        .catch(() => {
          if (isMounted) {
            removeIcon(item.id);
          }
        });
    });

    return () => {
      isMounted = false;
    };
  }, [dockItems]);

  function removeIcon(itemId: string) {
    setAppIconUrls((currentIcons) => {
      const { [itemId]: _removedIcon, ...remainingIcons } = currentIcons;
      return remainingIcons;
    });
  }

  return {
    appIconUrls,
    removeIcon,
  };
}
