import { Alignment } from "@seelen-ui/types";
import { Mutex } from "../../../utils/async.ts";
import { adjustPositionByPlacement, fitIntoMonitor } from "../positioning.ts";
import { Widget_2 } from "./2_triggering.ts";

/**
 * Explicit-validity frame model.
 *
 * `initialized` is the single source of truth for "the geometry is known".
 * The tuple (0,0,0,0) alone NEVER means "known": an uninitialized frame is
 * `initialized: false` and must not be consumed by placement code until it is
 * explicitly hydrated (see `hydrateFromNative` / `executeAutoSize`).
 */
export interface OptimisticFrame {
  initialized: boolean;
  x: number;
  y: number;
  width: number;
  height: number;
}

export const OPTIMISTIC_FRAME = new Mutex<OptimisticFrame>({
  initialized: false,
  x: 0,
  y: 0,
  width: 0,
  height: 0,
});

function isFrameValid(frame: { width: number; height: number }): boolean {
  return Number.isFinite(frame.width) && frame.width > 0 &&
    Number.isFinite(frame.height) && frame.height > 0;
}

/** Hard invariant used before any `show()` of an autosized popup. */
export function isOptimisticFrameValid(frame: OptimisticFrame): boolean {
  return frame.initialized && isFrameValid(frame) && Number.isFinite(frame.x) &&
    Number.isFinite(frame.y);
}

/**
 * First-use hydration of the optimistic frame from the already-probed native
 * geometry. This is eager and deterministic: it does not wait for a future
 * `onResized` / `onMoved` event to make the state valid. Native events become
 * pure reconciliation afterwards.
 */
export function initOptimisticFrame(widget: Widget_2): void {
  OPTIMISTIC_FRAME.runExclusive((ref) => {
    hydrateFromNative(ref, widget);
  });

  widget.onResized((e) => {
    OPTIMISTIC_FRAME.runExclusive((ref) => {
      ref.width = e.width;
      ref.height = e.height;
      if (isFrameValid(ref)) {
        ref.initialized = true;
      }
    });
  });

  widget.onMoved((e) => {
    OPTIMISTIC_FRAME.runExclusive((ref) => {
      ref.x = e.x;
      ref.y = e.y;
      if (isFrameValid(ref)) {
        ref.initialized = true;
      }
    });
  });
}

function hydrateFromNative(ref: OptimisticFrame, widget: Widget_2): void {
  if (ref.initialized) return;
  const native = widget.frame;
  if (isFrameValid(native)) {
    ref.x = native.x;
    ref.y = native.y;
    ref.width = native.width;
    ref.height = native.height;
    ref.initialized = true;
  }
}

interface AutoSizerState {
  enabled: boolean;
  /** From which side the widget will grow */
  originX?: Alignment | null;
  /** From which side the widget will grow */
  originY?: Alignment | null;
  element: HTMLElement;
  fitOnScreen: boolean;
  /** Anchor of the latest trigger, used for the absolute first-frame placement. */
  anchor?: { x: number; y: number } | null;
}

export class Widget_3 extends Widget_2 {
  protected autoSize: AutoSizerState = {
    enabled: false,
    element: document.body,
    fitOnScreen: true,
    anchor: null,
  };

  constructor() {
    super();
    this.executeAutoSize = this.executeAutoSize.bind(this);
  }

  protected setupAutoSizer(element: HTMLElement, fitOnScreen: boolean): void {
    this.autoSize = { ...this.autoSize, element, fitOnScreen, enabled: true };

    // Disable resizing by the user
    this.window.setResizable(false);

    // Runs synchronously before the popup-preset trigger callback (registered
    // later), so the anchor/align are stored before `executeAutoSize()` reads
    // them in the same trigger dispatch.
    this.onTrigger(({ desiredPosition, alignX, alignY }) => {
      this.autoSize.originX = alignX;
      this.autoSize.originY = alignY;
      this.autoSize.anchor = desiredPosition
        ? { x: desiredPosition.x, y: desiredPosition.y }
        : null;
    });

    const observer = new ResizeObserver(this.executeAutoSize);
    observer.observe(element, {
      box: "border-box",
    });
  }

  /**
   * Converges the native window onto the final geometry.
   *
   * - Anchored (trigger path): absolute placement computed from the icon
   *   anchor + freshly measured size. Runs while hidden, before `show()`, so
   *   the first visible frame is already the final one (no delta from a
   *   previous/zero frame).
   * - Unanchored (ResizeObserver reconciliation): delta math against the last
   *   *valid* frame; the first measurement establishes the absolute size.
   *
   * The local optimistic frame is updated synchronously by
   * `__unsafe_setSelfPosition`, so placement code never depends on the async
   * native event to see the new geometry.
   */
  protected async executeAutoSize(): Promise<OptimisticFrame> {
    const guard = await OPTIMISTIC_FRAME.acquire();
    try {
      const ref = guard.value;
      if (!ref.initialized) {
        hydrateFromNative(ref, this);
      }

      const dpr = globalThis.window.devicePixelRatio || 1;
      const measured = {
        width: Math.ceil(this.autoSize.element.scrollWidth * dpr),
        height: Math.ceil(this.autoSize.element.scrollHeight * dpr),
      };

      const anchor = this.autoSize.anchor;

      // ── anchored: one absolute pass (position + size), before show() ──
      if (anchor) {
        const placed = adjustPositionByPlacement({
          frame: {
            x: anchor.x,
            y: anchor.y,
            width: measured.width,
            height: measured.height,
          },
          originX: this.autoSize.originX,
          originY: this.autoSize.originY,
        });

        const unchanged = ref.initialized && ref.x === placed.x && ref.y === placed.y &&
          ref.width === placed.width && ref.height === placed.height;

        if (!unchanged) {
          await this.__unsafe_setSelfPosition(
            {
              left: placed.x,
              top: placed.y,
              right: placed.x + placed.width,
              bottom: placed.y + placed.height,
            },
            ref,
          );
        }
        return { ...ref };
      }

      // ── no anchor: size-only correction, delta against a valid frame ──
      if (!ref.initialized) {
        const origin = this.position;
        const first = fitIntoMonitor({ x: origin.x, y: origin.y, ...measured });
        await this.__unsafe_setSelfPosition(
          {
            left: first.x,
            top: first.y,
            right: first.x + first.width,
            bottom: first.y + first.height,
          },
          ref,
        );
        return { ...ref };
      }

      const widthDiff = measured.width - ref.width;
      const heightDiff = measured.height - ref.height;

      // Only update if the difference is more than 1px (avoid infinite loops from decimal differences)
      if (widthDiff === 0 && heightDiff === 0) {
        return { ...ref };
      }

      let frame = {
        x: ref.x,
        y: ref.y,
        width: measured.width,
        height: measured.height,
      };

      if (this.autoSize.originX === Alignment.Center) {
        frame.x -= widthDiff / 2;
      } else if (this.autoSize.originX === Alignment.End) {
        frame.x -= widthDiff;
      }

      if (this.autoSize.originY === Alignment.Center) {
        frame.y -= heightDiff / 2;
      } else if (this.autoSize.originY === Alignment.End) {
        frame.y -= heightDiff;
      }

      if (this.autoSize.fitOnScreen) {
        frame = fitIntoMonitor(frame);
      }

      await this.__unsafe_setSelfPosition(
        {
          left: frame.x,
          top: frame.y,
          right: frame.x + frame.width,
          bottom: frame.y + frame.height,
        },
        ref,
      );
      return { ...ref };
    } finally {
      guard.release();
    }
  }
}
