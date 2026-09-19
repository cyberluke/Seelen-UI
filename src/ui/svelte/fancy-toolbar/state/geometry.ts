import type { Rect } from "@seelen-ui/lib/types";
import { FancyToolbarSide } from "@seelen-ui/lib/types";

export interface ToolbarSizes {
  itemSize: number;
  padding: number;
  margin: number;
}

/**
 * Pure mirror of the toolbar `widgetRect` derivation.
 *
 * NOTE: the webview of the fancy-toolbar for `Left/Right` intentionally
 * spans the full monitor rect (only the inner css column is `--config-height`).
 * This is the same convention the window-manager canvas subtraction uses.
 */
export function computeToolbarRect(
  monitor: Rect,
  sizes: ToolbarSizes,
  scaleFactor: number,
  position: FancyToolbarSide,
): Rect {
  const height = Math.round(
    (sizes.itemSize + sizes.padding * 2 + sizes.margin * 2) * scaleFactor,
  );

  const rect: Rect = { ...monitor };

  // exactly mirrors the $derived logic, incl. the intentional fall-through:
  // for Left/Right the webview covers the whole monitor (css column is inset)
  switch (position) {
    case FancyToolbarSide.Top:
      rect.bottom = monitor.top + height;
      break;
    case FancyToolbarSide.Bottom:
      rect.top = monitor.bottom - height;
      break;
  }

  return rect;
}
