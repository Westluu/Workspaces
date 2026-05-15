import type { DetailedHTMLProps, HTMLAttributes } from "react";

declare module "react/jsx-runtime" {
  namespace JSX {
    interface IntrinsicElements {
      "dock-wrapper": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          allowDragDelete?: boolean;
          disabled?: boolean;
          direction?: "horizontal" | "vertical";
          easing?: string;
          gap?: number | string;
          maxRange?: number | string;
          maxScale?: number | string;
          padding?: number | string;
          position?: "top" | "bottom" | "left" | "right";
          size?: number | string;
          sortable?: boolean;
          willChange?: boolean;
          "allow-drag-delete"?: boolean | string;
          "max-range"?: number | string;
          "max-scale"?: number | string;
          "will-change"?: boolean | string;
        },
        HTMLElement
      >;
      "dock-item": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          direction?: "horizontal" | "vertical";
          easing?: string;
          gap?: number | string;
          scale?: number | string;
          size?: number | string;
          width?: number | string;
        },
        HTMLElement
      >;
      "dock-separator": DetailedHTMLProps<
        HTMLAttributes<HTMLElement> & {
          direction?: "horizontal" | "vertical";
          size?: number | string;
          thickness?: number | string;
        },
        HTMLElement
      >;
    }
  }
}
