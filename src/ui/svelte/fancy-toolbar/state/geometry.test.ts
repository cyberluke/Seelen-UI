import { assertEquals } from "@std/assert";
import { FancyToolbarSide } from "@seelen-ui/lib/types";
import { computeToolbarRect } from "./geometry.ts";

const MONITOR_4K = { left: 0, top: 0, right: 3840, bottom: 2160 };
const SIZES = { itemSize: 16, padding: 8, margin: 0 };

Deno.test("toolbar rect: Top subtracts from top only", () => {
  const rect = computeToolbarRect(MONITOR_4K, SIZES, 2, FancyToolbarSide.Top);
  // (16 + 8*2 + 0) * 2 = 64 => bottom = 0 + 64
  assertEquals(rect, { left: 0, top: 0, right: 3840, bottom: 64 });
});

Deno.test("toolbar rect: Bottom subtracts from bottom only", () => {
  const rect = computeToolbarRect(
    MONITOR_4K,
    SIZES,
    2,
    FancyToolbarSide.Bottom,
  );
  assertEquals(rect, { left: 0, top: 2096, right: 3840, bottom: 2160 });
});

// FancyToolbarSide has only Top/Bottom (Left/Right collapse to the
// full-monitor rect through the same fall-through branch).
Deno.test("toolbar rect: fall-through keeps full monitor rect", () => {
  const rect = computeToolbarRect(MONITOR_4K, SIZES, 2, FancyToolbarSide.Top);
  assertEquals(rect.left, 0);
  assertEquals(rect.top, 0);
  assertEquals(rect.right, 3840);
});

Deno.test("toolbar rect: scale factor 1 rounding", () => {
  const rect = computeToolbarRect(
    { left: 0, top: 0, right: 1920, bottom: 1080 },
    SIZES,
    1,
    FancyToolbarSide.Top,
  );
  assertEquals(rect.bottom, 32);
});

Deno.test("toolbar rect: fractional scale factor rounds to integer", () => {
  const rect = computeToolbarRect(
    MONITOR_4K,
    { itemSize: 16, padding: 8, margin: 0 },
    1.25,
    FancyToolbarSide.Top,
  );
  // 32 * 1.25 = 40
  assertEquals(rect.bottom, 40);
  assertEquals(Number.isInteger(rect.bottom), true);
});
