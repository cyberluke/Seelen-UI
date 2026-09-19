import { assertEquals } from "@std/assert";
import { FancyToolbarSide, SeelenWegSide } from "@seelen-ui/lib/types";
import {
  computeScreenCenter,
  computeWegRects,
  computeWegWorkArea,
  isHorizontalDockSide,
  isLargePreviewCard,
} from "./geometry.ts";

const MONITOR_4K = { left: 0, top: 0, right: 3840, bottom: 2160 };

const input = {
  monitor: MONITOR_4K,
  scaleFactor: 2,
  toolbar: {
    enabled: true,
    enabledOnMonitor: true,
    position: FancyToolbarSide.Bottom,
    itemSize: 16,
    padding: 8,
    margin: 0,
  },
  isTouch: false,
  dock: {
    enabled: true,
    enabledOnMonitor: true,
    hideMode: "Never" as const,
    position: SeelenWegSide.Right,
    size: 35,
    padding: 3,
    margin: 0,
  },
};

Deno.test("weg workArea: subtracts only bottom on tb-bottom", () => {
  const wa = computeWegWorkArea(input);
  assertEquals(wa, { left: 0, top: 0, right: 3840, bottom: 2096 });
});

Deno.test(
  "weg rects: right side, non-touch -> webview left = wa.mid, hitbox left = right-size",
  () => {
    const { hitboxRect, webviewRect } = computeWegRects(input);
    // size = (35+6)*2 = 82 ; wa.right 3840 -> hitbox.left 3758 ; webview.left 0+1920
    assertEquals(hitboxRect, { left: 3758, top: 0, right: 3840, bottom: 2096 });
    assertEquals(webviewRect, {
      left: 1920,
      top: 0,
      right: 3840,
      bottom: 2096,
    });
  },
);

Deno.test("weg rects: right side, touch -> webview == hitbox", () => {
  const { hitboxRect, webviewRect } = computeWegRects({
    ...input,
    isTouch: true,
  });
  assertEquals(webviewRect, hitboxRect);
});

Deno.test("weg rects: odd mid rounding uses Math.round", () => {
  const base = {
    ...input,
    monitor: { left: 0, top: 0, right: 2001, bottom: 1001 },
  };
  const { webviewRect } = computeWegRects(base);
  // wa width 2001 -> half 1000.5 -> 1001 (round)
  assertEquals(webviewRect.left, 1001);
});

Deno.test("weg workArea: disabled toolbar changes nothing", () => {
  const wa = computeWegWorkArea({
    ...input,
    toolbar: { ...input.toolbar, enabled: false },
  });
  assertEquals(wa, MONITOR_4K);
});

Deno.test("weg sides: horizontal detection", () => {
  assertEquals(isHorizontalDockSide(SeelenWegSide.Top), true);
  assertEquals(isHorizontalDockSide(SeelenWegSide.Bottom), true);
  assertEquals(isHorizontalDockSide(SeelenWegSide.Left), false);
  assertEquals(isHorizontalDockSide(SeelenWegSide.Right), false);
});

Deno.test("weg all four sides: finite, within workArea", () => {
  const { hitboxRect, webviewRect } = computeWegRects(input);
  const wa = computeWegWorkArea(input);
  for (const rect of [hitboxRect, webviewRect]) {
    assertEquals(
      Number.isFinite(rect.left) && Number.isFinite(rect.right),
      true,
    );
    assertEquals(
      Number.isFinite(rect.top) && Number.isFinite(rect.bottom),
      true,
    );
    assertEquals(rect.left >= wa.left && rect.right <= wa.right, true);
    assertEquals(rect.top >= wa.top && rect.bottom <= wa.bottom, true);
    if (rect.left === rect.right || rect.top === rect.bottom) {
      throw new Error("degenerate rect");
    }
  }
});

// ── size-aware preview placement ──────────────────────────────────────────

Deno.test("placement: small card sizes -> anchored to icon", () => {
  assertEquals(isLargePreviewCard(128, 2), false);
  assertEquals(isLargePreviewCard(170, 2), false);
  assertEquals(isLargePreviewCard(128, 1), false);
});

Deno.test("placement: threshold boundary 200", () => {
  assertEquals(isLargePreviewCard(200, 1), true);
  assertEquals(isLargePreviewCard(199, 1), false);
});

Deno.test("placement: large card (256) -> centered flag", () => {
  assertEquals(isLargePreviewCard(256, 2), true);
  assertEquals(isLargePreviewCard(256, 1), true);
});

Deno.test("placement: min 64 clamp respected, scale-invariant", () => {
  assertEquals(isLargePreviewCard(0, 2), false); // max(0,64)=64
  assertEquals(isLargePreviewCard(0, 4), false); // raw 64 < 200 regardless of scale
  assertEquals(isLargePreviewCard(0), false); // undefined scale ok
});

Deno.test("screen center: full 4k and offset monitor", () => {
  assertEquals(computeScreenCenter({ left: 0, top: 0, right: 3840, bottom: 2160 }), {
    x: 1920,
    y: 1080,
  });
  assertEquals(computeScreenCenter({ left: 3840, top: 0, right: 5760, bottom: 1080 }), {
    x: 4800,
    y: 540,
  });
});

Deno.test("screen center: odd sizes round consistently", () => {
  const c = computeScreenCenter({ left: 0, top: 0, right: 2001, bottom: 1001 });
  assertEquals(c.x, 1001); // round(1000.5)
  assertEquals(c.y, 501);
});
