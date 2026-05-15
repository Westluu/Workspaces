import type { DockTitle as DockTitleState } from "../types";

type DockTitleProps = {
  dockTitle: DockTitleState;
};

export function DockTitle({ dockTitle }: DockTitleProps) {
  return (
    <span
      className="dock-title"
      style={{
        left: dockTitle.left,
        top: dockTitle.top,
      }}
    >
      {dockTitle.label}
    </span>
  );
}
