import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

type NativeDockMouseMove = {
  x: number;
  y: number;
};

type UseNativeDockMouseTrackingParams = {
  clearDockTitle: () => void;
  showDockTitle: (label: string, element: HTMLElement) => void;
};

export function useNativeDockMouseTracking({
  clearDockTitle,
  showDockTitle,
}: UseNativeDockMouseTrackingParams) {
  useEffect(() => {
    let disposed = false;
    let unlistenMove: (() => void) | undefined;
    let unlistenLeave: (() => void) | undefined;

    async function setupListeners() {
      unlistenMove = await listen<NativeDockMouseMove>(
        "native-dock-mouse-move",
        ({ payload }) => {
          const element = document
            .elementFromPoint(payload.x, payload.y)
            ?.closest<HTMLElement>("[data-dock-label]");

          if (!element) {
            clearDockTitle();
            return;
          }

          showDockTitle(element.dataset.dockLabel ?? "", element);
        },
      );

      unlistenLeave = await listen("native-dock-mouse-leave", clearDockTitle);

      if (disposed) {
        unlistenMove();
        unlistenLeave();
      }
    }

    void setupListeners();

    return () => {
      disposed = true;
      unlistenMove?.();
      unlistenLeave?.();
    };
  }, [clearDockTitle, showDockTitle]);
}
