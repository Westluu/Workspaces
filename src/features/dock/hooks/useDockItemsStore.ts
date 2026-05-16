import { useCallback, useEffect, useState } from "react";
import type { DockItem, DockItemInput } from "../types";
import { getDockItems, setDockItems } from "../services/dockService";

function generateId(): string {
  return crypto.randomUUID();
}

function ensureIds(items: DockItem[]): DockItem[] {
  return items.map((item) => ({
    ...item,
    id: item.id || generateId(),
  }));
}

export function useDockItemsStore() {
  const [items, setItems] = useState<DockItem[]>([]);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    getDockItems()
      .then((json) => {
        try {
          const parsed = JSON.parse(json) as DockItem[];
          setItems(ensureIds(parsed));
        } catch {
          setItems([]);
        }
        setIsLoaded(true);
      })
      .catch(() => {
        setItems([]);
        setIsLoaded(true);
      });
  }, []);

  const persist = useCallback((nextItems: DockItem[]) => {
    setItems(nextItems);
    setDockItems(JSON.stringify(nextItems)).catch(() => {});
  }, []);

  const addItem = useCallback(
    (item: DockItemInput) => {
      const newItem = { ...item, id: generateId() } as DockItem;
      persist([...items, newItem]);
    },
    [items, persist],
  );

  const removeItem = useCallback(
    (itemId: string) => {
      persist(items.filter((item) => item.id !== itemId));
    },
    [items, persist],
  );

  const reorderItems = useCallback(
    (reorderedItems: DockItem[]) => {
      persist(reorderedItems);
    },
    [persist],
  );

  return {
    items,
    isLoaded,
    addItem,
    removeItem,
    reorderItems,
  };
}
