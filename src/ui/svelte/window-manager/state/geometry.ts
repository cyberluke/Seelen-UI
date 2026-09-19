import type { Rect } from "@seelen-ui/lib/types";
import { FancyToolbarSide, HideMode, SeelenWegSide } from "@seelen-ui/lib/types";

export interface WmRectInput {
  monitor: Rect;
  scaleFactor: number;
  isTouch: boolean;
  toolbar: {
    enabled: boolean;
    enabledOnMonitor: boolean;
    hideMode: HideMode;
    position: FancyToolbarSide;
    itemSize: number;
    padding: number;
    margin: number;
  };
  weg: {
    enabled: boolean;
    enabledOnMonitor: boolean;
    hideMode: HideMode;
    position: SeelenWegSide;
    size: number;
    padding: number;
    margin: number;
  };
}

/**
 * Pure mirror of the window-manager canvas `widgetRect` derivation
 * (monitor rect minus the opaque/always-on shell bars).
 */
export function computeWmCanvasRect(input: WmRectInput): Rect {
  const rect: Rect = { ...input.monitor };
  const { toolbar, weg, scaleFactor, isTouch } = input;

  if (
    toolbar.enabled &&
    toolbar.enabledOnMonitor &&
    (toolbar.hideMode === HideMode.Never || isTouch)
  ) {
    const tbSize = Math.round(
      (toolbar.itemSize + toolbar.padding * 2 + toolbar.margin * 2) *
        scaleFactor,
    );
    // NOTE: Left/Right intentionally not subtracted here, see the css-column comment
    // in fancy-toolbar/state/geometry.ts
    switch (toolbar.position) {
      case FancyToolbarSide.Top:
        rect.top += tbSize;
        break;
      case FancyToolbarSide.Bottom:
        rect.bottom -= tbSize;
        break;
    }
  }

  if (
    weg.enabled &&
    weg.enabledOnMonitor &&
    (weg.hideMode === HideMode.Never || isTouch)
  ) {
    const wegSize = Math.round(
      (weg.size + weg.padding * 2 + weg.margin * 2) * scaleFactor,
    );
    switch (weg.position) {
      case SeelenWegSide.Top:
        rect.top += wegSize;
        break;
      case SeelenWegSide.Bottom:
        rect.bottom -= wegSize;
        break;
      case SeelenWegSide.Left:
        rect.left += wegSize;
        break;
      case SeelenWegSide.Right:
        rect.right -= wegSize;
        break;
    }
  }

  return rect;
}
