import type { Rect } from "@seelen-ui/lib/types";
import { FancyToolbarSide, HideMode, SeelenWegSide } from "@seelen-ui/lib/types";

export interface WegSizes {
  /** item size in config units (before monitor scale factor) */
  size: number;
  padding: number;
  margin: number;
}

export interface WegWorkAreaInput {
  monitor: Rect;
  scaleFactor: number;
  toolbar: {
    enabled: boolean;
    enabledOnMonitor: boolean;
    position: FancyToolbarSide;
    itemSize: number;
    padding: number;
    margin: number;
  };
}

export interface WegRectsInput extends WegWorkAreaInput {
  isTouch: boolean;
  dock: WegSizes & { position: SeelenWegSide };
}

export interface WegRect {
  hitboxRect: Rect;
  webviewRect: Rect;
}

/**
 * Pure mirror of weg `workArea`: removes the toolbar strip from the monitor rect.
 * NOTE: intentionally no hideMode check here, the same as the original derivation.
 */
export function computeWegWorkArea(input: WegWorkAreaInput): Rect {
  const workArea = { ...input.monitor };
  const { toolbar, scaleFactor } = input;

  if (!toolbar.enabled || !toolbar.enabledOnMonitor) {
    return workArea;
  }

  const tbSize = Math.round(
    (toolbar.itemSize + toolbar.padding * 2 + toolbar.margin * 2) * scaleFactor,
  );

  switch (toolbar.position) {
    case FancyToolbarSide.Top:
      return { ...workArea, top: workArea.top + tbSize };
    case FancyToolbarSide.Bottom:
      return { ...workArea, bottom: workArea.bottom - tbSize };
  }

  return workArea;
}

/**
 * Pure mirror of weg `widgetRect`: the dock has two rects.
 * - `hitboxRect`: the real reserved area (exact stripe), registered as appbar.
 * - `webviewRect`: what the webview is sized to. On non-touch it is a half
 *   monitor window (for the fullscreen preview mode) aligned to the dock side.
 */
export function computeWegRects(input: WegRectsInput): WegRect {
  const wa = computeWegWorkArea(input);
  const hitboxRect: Rect = { ...wa };
  const webviewRect: Rect = { ...wa };

  const size = Math.round(
    (input.dock.size + input.dock.padding * 2 + input.dock.margin * 2) *
      input.scaleFactor,
  );
  const isTouch = input.isTouch;

  switch (input.dock.position) {
    case SeelenWegSide.Left:
      hitboxRect.right = hitboxRect.left + size;
      webviewRect.right = isTouch ? hitboxRect.right : wa.right - Math.round((wa.right - wa.left) / 2);
      break;
    case SeelenWegSide.Right:
      hitboxRect.left = hitboxRect.right - size;
      webviewRect.left = isTouch ? hitboxRect.left : wa.left + Math.round((wa.right - wa.left) / 2);
      break;
    case SeelenWegSide.Top:
      hitboxRect.bottom = hitboxRect.top + size;
      webviewRect.bottom = isTouch ? hitboxRect.bottom : wa.top + Math.round((wa.bottom - wa.top) / 2);
      break;
    case SeelenWegSide.Bottom:
      hitboxRect.top = hitboxRect.bottom - size;
      webviewRect.top = isTouch ? hitboxRect.top : wa.bottom - Math.round((wa.bottom - wa.top) / 2);
      break;
  }

  return { hitboxRect, webviewRect };
}

export function isHorizontalDockSide(position: SeelenWegSide): boolean {
  return position === SeelenWegSide.Top || position === SeelenWegSide.Bottom;
}

/** card widths (config px) at which the grouped preview is anchored to the clicked icon */
export const SMALL_CARD_MAX = 200;

/**
 * Size-aware placement of the grouped window preview:
 * - small cards (config width <= 200, e.g. 128/170): anchored next to the clicked dock item
 * - large cards (config width >= 200+ e.g. 256): centered on the screen (both axis Center)
 *
 * The threshold is on the config value itself so it is stable across DPI scale
 * factors (the monitor `scaleFactor` only multiplies the resulting rects).
 */
export function isLargePreviewCard(cardWidth: number, _scaleFactor?: number): boolean {
  return Math.max(cardWidth, 64) >= SMALL_CARD_MAX;
}

export function computeScreenCenter(
  rect: Rect,
): { x: number; y: number } {
  return {
    x: rect.left + Math.round((rect.right - rect.left) / 2),
    y: rect.top + Math.round((rect.bottom - rect.top) / 2),
  };
}

export { HideMode };
