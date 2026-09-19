import type { Rect } from "@seelen-ui/lib/types";
import { Alignment, FancyToolbarSide, HideMode, SeelenWegSide } from "@seelen-ui/lib/types";

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
 * Native surface rectangles for the dock. `webviewRect` is the real window
 * rect of the `@seelen/weg` webview and is derived from content + work area:
 * - touch: exactly the stripe (same as the hitbox),
 * - non-touch: the full work area (the document is layered hit-test so the
 *   transparent area is click-through). No half-monitor guesses.
 * `@seelen/weg-preview` is a separate Popup preset surface and never shares
 * these rects.
 */
export function computeWegRects(input: WegRectsInput): WegRect {
  const wa = computeWegWorkArea(input);
  const hitboxRect: Rect = { ...wa };
  const webviewRect: Rect = { ...wa };

  const stripeSize = Math.round(
    (input.dock.size + input.dock.padding * 2 + input.dock.margin * 2) *
      input.scaleFactor,
  );

  const stripe: Rect = { ...wa };
  switch (input.dock.position) {
    case SeelenWegSide.Left:
      stripe.right = stripe.left + stripeSize;
      break;
    case SeelenWegSide.Right:
      stripe.left = stripe.right - stripeSize;
      break;
    case SeelenWegSide.Top:
      stripe.bottom = stripe.top + stripeSize;
      break;
    default:
      stripe.top = stripe.bottom - stripeSize;
      break;
  }
  Object.assign(hitboxRect, stripe);

  // Only a touch surface is allowed to shrink to the stripe: the document
  // receives pointer events directly there. On desktop the window keeps the
  // full work area so the layered hit-test math stays exact.
  if (input.isTouch) {
    Object.assign(webviewRect, stripe);
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

/** sensible margin between the dock strip and the preview card (css px) */
export const PREVIEW_EDGE_GAP = 6;

export interface PreviewAnchorInput {
  /** true when the card is "large" -> centered on screen */
  large: boolean;
  /** full monitor rect (physical px) */
  monitor: Rect;
  /** the dock hitbox rect (physical px) */
  hitbox: Rect;
  /** item center in css px relative to the dock webview viewport */
  itemCenterX: number;
  itemCenterY: number;
  dockSide: SeelenWegSide;
  scaleFactor: number;
}

export interface PreviewAnchorResult {
  x: number;
  y: number;
  alignX: Alignment;
  alignY: Alignment;
}

/**
 * Size-aware preview placement:
 * - large card: centered on the screen (both axis `Center`)
 * - small card: anchored to the dock strip with a sensible gap. The align
 *   semantics then subtract the preview's own size from this point, i.e. the
 *   final top = anchorY - (height + margin) for `End`, exactly:
 *   `offset = preview height + margin`.
 */
export function computePreviewAnchor(input: PreviewAnchorInput): PreviewAnchorResult {
  if (input.large) {
    const center = computeScreenCenter(input.monitor);
    return { x: center.x, y: center.y, alignX: Alignment.Center, alignY: Alignment.Center };
  }

  const gap = Math.round(PREVIEW_EDGE_GAP * (input.scaleFactor || 1));
  const toPhysicalX = (n: number) => input.hitbox.left + Math.round(n * (input.scaleFactor || 1));
  const toPhysicalY = (n: number) => input.hitbox.top + Math.round(n * (input.scaleFactor || 1));

  // Native geometry invariant: the anchor origin must stay inside the monitor,
  // so the aligned popup rect is always contained by the work area.
  const clampX = (n: number) => Math.max(input.monitor.left, Math.min(input.monitor.right, n));
  const clampY = (n: number) => Math.max(input.monitor.top, Math.min(input.monitor.bottom, n));

  let anchor: PreviewAnchorResult;
  switch (input.dockSide) {
    case SeelenWegSide.Bottom:
      anchor = {
        x: toPhysicalX(input.itemCenterX),
        y: input.hitbox.top - gap,
        alignX: Alignment.Center,
        alignY: Alignment.End,
      };
      break;
    case SeelenWegSide.Top:
      anchor = {
        x: toPhysicalX(input.itemCenterX),
        y: input.hitbox.bottom + gap,
        alignX: Alignment.Center,
        alignY: Alignment.Start,
      };
      break;
    case SeelenWegSide.Left:
      anchor = {
        x: input.hitbox.right + gap,
        y: toPhysicalY(input.itemCenterY),
        alignX: Alignment.Start,
        alignY: Alignment.Center,
      };
      break;
    default:
      anchor = {
        x: input.hitbox.left - gap,
        y: toPhysicalY(input.itemCenterY),
        alignX: Alignment.End,
        alignY: Alignment.Center,
      };
      break;
  }
  anchor.x = clampX(anchor.x);
  anchor.y = clampY(anchor.y);
  return anchor;
}

export { HideMode };
