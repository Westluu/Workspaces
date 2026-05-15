import { useEffect, useState } from "react";
import type { DockItem } from "../types";
import { getAppDisplayName } from "../services/dockService";

export function useDockLabels(dockItems: DockItem[]) {
  const [appLabels, setAppLabels] = useState<Record<string, string>>({});

  useEffect(() => {
    let isMounted = true;
    const appItems = dockItems.filter((item) => item.type === "app");

    appItems.forEach((item) => {
      getAppDisplayName(item.appPath)
        .then((label) => {
          if (isMounted) {
            setAppLabels((currentLabels) => ({
              ...currentLabels,
              [item.id]: label,
            }));
          }
        })
        .catch(() => {
          if (isMounted) {
            removeLabel(item.id);
          }
        });
    });

    return () => {
      isMounted = false;
    };
  }, [dockItems]);

  function removeLabel(itemId: string) {
    setAppLabels((currentLabels) => {
      const { [itemId]: _removedLabel, ...remainingLabels } = currentLabels;
      return remainingLabels;
    });
  }

  return {
    appLabels,
  };
}
