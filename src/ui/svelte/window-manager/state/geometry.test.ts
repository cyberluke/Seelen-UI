import { assertEquals } from "@std/assert";
import { FancyToolbarSide, HideMode, SeelenWegSide } from "@seelen-ui/lib/types";
import { computeWmCanvasRect } from "./geometry.ts";

const MONITOR_4K = { left: 0, top: 0, right: 3840, bottom: 2160 };
const SCALE = 2;

const tb = (position: FancyToolbarSide) => ({
  enabled: true,
  enabledOnMonitor: true,
  hideMode: HideMode.Never,
  position,
  itemSize: 16,
  padding: 8,
  margin: 0,
});

const dock = (position: SeelenWegSide) => ({
  enabled: true,
  enabledOnMonitor: true,
  hideMode: HideMode.Never,
  position,
  size: 35,
  padding: 3,
  margin: 0,
});

Deno.test(
  "wm canvas: 4k@200% tb-bottom + weg-right (matches live rects)",
  () => {
    const rect = computeWmCanvasRect({
      monitor: MONITOR_4K,
      scaleFactor: SCALE,
      isTouch: false,
      toolbar: tb(FancyToolbarSide.Bottom),
      weg: dock(SeelenWegSide.Right),
    });
    // tbSize 64 -> bottom 2096; wegSize 82 -> right 3758 ; top 0
    assertEquals(rect, { left: 0, top: 0, right: 3758, bottom: 2096 });
  },
);

Deno.test("wm canvas: tb-top subtracts top only", () => {
  const rect = computeWmCanvasRect({
    monitor: MONITOR_4K,
    scaleFactor: SCALE,
    isTouch: false,
    toolbar: tb(FancyToolbarSide.Top),
    weg: dock(SeelenWegSide.Right),
  });
  assertEquals(rect, { left: 0, top: 64, right: 3758, bottom: 2160 });
});

Deno.test(
  "wm canvas: tb-left/right subtract nothing (full monitor minus weg only)",
  () => {
    // Top: top+64, bottom unchanged; Bottom: bottom-64 (tbSize=64), weg bottom -82.
    const top = computeWmCanvasRect({
      monitor: MONITOR_4K,
      scaleFactor: SCALE,
      isTouch: false,
      toolbar: tb(FancyToolbarSide.Top),
      weg: dock(SeelenWegSide.Bottom),
    });
    assertEquals(top, { left: 0, top: 64, right: 3840, bottom: 2078 });
    const bottom = computeWmCanvasRect({
      monitor: MONITOR_4K,
      scaleFactor: SCALE,
      isTouch: false,
      toolbar: tb(FancyToolbarSide.Bottom),
      weg: dock(SeelenWegSide.Bottom),
    });
    // both toolbar and weg subtract from bottom: 2160 - 64 - 82
    assertEquals(bottom, { left: 0, top: 0, right: 3840, bottom: 2014 });
  },
);

Deno.test("wm canvas: weg left/right/top/bottom subtract correctly", () => {
  const noTb = { ...tb(FancyToolbarSide.Top), enabled: false };
  const cases = new Map<SeelenWegSide, Rect[]>([
    [SeelenWegSide.Top, [{ left: 0, top: 82, right: 3840, bottom: 2160 }]],
    [SeelenWegSide.Bottom, [{ left: 0, top: 0, right: 3840, bottom: 2078 }]],
    [SeelenWegSide.Left, [{ left: 82, top: 0, right: 3840, bottom: 2160 }]],
    [SeelenWegSide.Right, [{ left: 0, top: 0, right: 3758, bottom: 2160 }]],
  ]);
  for (const [position, expected] of cases) {
    const rect = computeWmCanvasRect({
      monitor: MONITOR_4K,
      scaleFactor: SCALE,
      isTouch: false,
      toolbar: noTb,
      weg: dock(position),
    });
    assertEquals(rect, expected[0], `side ${position}`);
  }
  type Rect = ReturnType<typeof computeWmCanvasRect>;
});

Deno.test("wm canvas: Auto/OnOverlap hideMode without tb subtraction", () => {
  for (const hideMode of [HideMode.Always, HideMode.OnOverlap] as HideMode[]) {
    const rect = computeWmCanvasRect({
      monitor: MONITOR_4K,
      scaleFactor: SCALE,
      isTouch: false,
      toolbar: { ...tb(FancyToolbarSide.Bottom), hideMode },
      weg: dock(SeelenWegSide.Right),
    });
    // toolbar skipped, weg still subtracted
    assertEquals(
      rect,
      { left: 0, top: 0, right: 3758, bottom: 2160 },
      `tb hideMode ${hideMode}`,
    );
  }
});

Deno.test("wm canvas: touch mode treats non-Never as reserved too", () => {
  const rect = computeWmCanvasRect({
    monitor: MONITOR_4K,
    scaleFactor: SCALE,
    isTouch: true,
    toolbar: { ...tb(FancyToolbarSide.Top), hideMode: HideMode.OnOverlap },
    weg: dock(SeelenWegSide.Left),
  });
  assertEquals(rect, { left: 82, top: 64, right: 3840, bottom: 2160 });
});

Deno.test("wm canvas: per-monitor disable flags respected", () => {
  const rect = computeWmCanvasRect({
    monitor: MONITOR_4K,
    scaleFactor: SCALE,
    isTouch: false,
    toolbar: { ...tb(FancyToolbarSide.Top), enabledOnMonitor: false },
    weg: { ...dock(SeelenWegSide.Right), enabled: false },
  });
  assertEquals(rect, MONITOR_4K);
});

Deno.test("wm canvas: never returns NaN / undefined axis", () => {
  const rect = computeWmCanvasRect({
    monitor: MONITOR_4K,
    scaleFactor: 2,
    isTouch: false,
    toolbar: tb(FancyToolbarSide.Bottom),
    weg: dock(SeelenWegSide.Right),
  });
  for (const axis of Object.values(rect)) {
    assertEquals(Number.isFinite(axis), true);
  }
});

Deno.test("wm canvas: offset monitor origin respected", () => {
  const rect = computeWmCanvasRect({
    monitor: { left: 3840, top: 0, right: 7680, bottom: 2160 },
    scaleFactor: SCALE,
    isTouch: false,
    toolbar: tb(FancyToolbarSide.Bottom),
    weg: dock(SeelenWegSide.Right),
  });
  assertEquals(rect, { left: 3840, top: 0, right: 7598, bottom: 2096 });
});
