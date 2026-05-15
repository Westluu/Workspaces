import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

type NativeDockVisibility = {
  hidden: boolean;
};

type UseDockVisibilityParams = {
  clearDockTitle: () => void;
};

export function useDockVisibility({ clearDockTitle }: UseDockVisibilityParams) {
  const [isDockHidden, setIsDockHidden] = useState(false);
  const shouldAutoHideRef = useRef(false);

  useEffect(() => {
    let disposed = false;
    let unlistenVisibility: (() => void) | undefined;
    let unlistenMove: (() => void) | undefined;
    let unlistenLeave: (() => void) | undefined;

    function applyVisibility(payload: NativeDockVisibility) {
      shouldAutoHideRef.current = payload.hidden;
      setIsDockHidden(payload.hidden);

      if (payload.hidden) {
        clearDockTitle();
      }
    }

    async function setupListeners() {
      invoke<NativeDockVisibility>("get_dock_visibility")
        .then((payload) => {
          if (!disposed) {
            applyVisibility(payload);
          }
        })
        .catch(() => {});

      unlistenVisibility = await listen<NativeDockVisibility>(
        "native-dock-visibility",
        ({ payload }) => {
          applyVisibility(payload);
        },
      );

      unlistenMove = await listen("native-dock-mouse-move", () => {
        if (shouldAutoHideRef.current) {
          setIsDockHidden(false);
        }
      });

      unlistenLeave = await listen("native-dock-mouse-leave", () => {
        if (shouldAutoHideRef.current) {
          clearDockTitle();
          setIsDockHidden(true);
        }
      });

      if (disposed) {
        unlistenVisibility();
        unlistenMove();
        unlistenLeave();
      }
    }

    void setupListeners();

    return () => {
      disposed = true;
      unlistenVisibility?.();
      unlistenMove?.();
      unlistenLeave?.();
    };
  }, [clearDockTitle]);

  return {
    isDockHidden,
  };
}
