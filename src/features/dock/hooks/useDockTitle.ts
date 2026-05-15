import { useCallback, useState, type FocusEvent, type MouseEvent } from "react";
import type { DockTitle } from "../types";

export function useDockTitle() {
  const [dockTitle, setDockTitle] = useState<DockTitle | null>(null);

  const showDockTitle = useCallback((label: string, element: HTMLElement) => {
    const rect = element.getBoundingClientRect();

    setDockTitle({
      label,
      left: rect.left + rect.width / 2,
      top: Math.max(6, rect.top - 34),
    });
  }, []);

  const handleTitleMouse = useCallback(
    (label: string, e: MouseEvent<HTMLElement>) => {
      showDockTitle(label, e.currentTarget);
    },
    [showDockTitle],
  );

  const handleTitleFocus = useCallback(
    (label: string, e: FocusEvent<HTMLElement>) => {
      showDockTitle(label, e.currentTarget);
    },
    [showDockTitle],
  );

  const clearDockTitle = useCallback(() => {
    setDockTitle(null);
  }, []);

  return {
    clearDockTitle,
    dockTitle,
    handleTitleFocus,
    handleTitleMouse,
    showDockTitle,
  };
}
