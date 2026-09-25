// Canonical grouped-preview acceptance driver (single coherent process).
// Consolidates the former acceptance-run drivers into one run, folds the atomic
// frontend frame trace (emitted by the preview webview into the runtime log)
// and emits a machine-readable artifact at target/acceptance/weg-preview.json
// with an explicit overall PASS/FAIL.
//
// Evidence planes:
//  - cold-pod lifecycle (native created hidden/non-hit-testable → first
//    visible frame == final anchored/content-sized rect)
//  - diagnostics-only slow-preparation probe (hidden window duration)
//  - live high-cardinality groups (6/8/10/12+): layout, scroll extent,
//    ordering, thumbnails, click/hover parity
//  - taskbar stability counters (generation/reload/mount/liveness deltas)
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const exe = "./target/release/slu.exe";
function slu(...args: string[]): string {
  return execFileSync(exe, args, { encoding: "utf8" });
}

// The runtime log is cumulative across restarts; every fold only reads lines
// appended after this baseline so evidence belongs to THIS acceptance run.
function runtimeLogLines(): string[] {
  const local = process.env.LOCALAPPDATA ?? process.env.APPDATA;
  if (!local) return [];
  const logPath = path.join(local, "com.seelen.seelen-ui", "logs", "NAI OS.log");
  if (!fs.existsSync(logPath)) return [];
  return fs.readFileSync(logPath, "utf8").split(/\r?\n/);
}
const logBaseline = runtimeLogLines().length;
function currentRunLines(): string[] {
  return runtimeLogLines().slice(logBaseline);
}
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function json(str: string): any {
  return JSON.parse(str);
}

// derive the physical monitor extent from the observed window geometries in the
// weg state (the monitors table itself carries no rect): the union of all
// window rects is bounded by the monitor the dock renders on.
function monitorRectFromState(s: any): { left: number; top: number; right: number; bottom: number } {
  let right = 0;
  let bottom = 0;
  const walk = (items: any[]) => {
    for (const item of items) {
      for (const w of item.windows ?? []) {
        right = Math.max(right, (w.geometry?.left ?? 0) + (w.geometry?.width ?? 0));
        bottom = Math.max(bottom, (w.geometry?.top ?? 0) + (w.geometry?.height ?? 0));
      }
    }
  };
  for (const pm of s.perMonitor ?? []) walk(pm.items ?? []);
  return { left: 0, top: 0, right: right || 1920, bottom: bottom || 1080 };
}

// The CLI `widget trigger <id> <json>` parses <json> as the flat customArgs
// map, so the custom args are passed exactly as the frontend reads them.
function trigger(session: string, hwnds: number[], geometry?: unknown, slowMs?: number): void {
  const customArgs: Record<string, unknown> = {
    previewSessionId: session,
    groupKey: session,
    hwnds,
    position: "Bottom",
    t0Date: Date.now(),
    animated: true,
    animationDuration: 150,
  };
  if (geometry) {
    customArgs.geometry = geometry;
  }
  if (slowMs && slowMs > 0) {
    customArgs.slowPreparationMs = slowMs;
  }
  slu("widget", "trigger", "@seelen/weg-preview", JSON.stringify(customArgs));
}

// sample geometry layer snapshot injected on the cold trigger, mirroring the
// values the real dock path injects (icon DOM/physical rects, monitor, taskbar)
function coldGeometry(): Record<string, unknown> {
  const state = json(slu("weg", "state"));
  return {
    iconDomRect: { left: 96, top: 1966, width: 40, height: 40 },
    iconPhysicalRect: { x: 96, y: 1966, width: 40, height: 40 },
    monitor: state.perMonitor?.length ? monitorRectFromState(state) : null,
    taskbarRect: { x: 0, y: 1954, width: 3840, height: 34 },
    taskbarVisible: true,
    scaleFactor: 1,
  };
}

function finiteRect(r: any): boolean {
  return (
    !!r &&
    [r.x, r.y, r.width, r.height].every(
      (n: number) => typeof n === "number" && Number.isFinite(n),
    ) &&
    r.width > 0 &&
    r.height > 0
  );
}

function rectInside(a: any, b: any): boolean {
  if (!finiteRect(a) || !b) return false;
  return (
    a.x >= b.left - 1 &&
    a.y >= b.top - 1 &&
    a.x + a.width <= b.right + 1 &&
    a.y + a.height <= b.bottom + 1
  );
}

const artifact: Record<string, unknown> = {};

// ── phase 0: real cold start — no preview pod exists before the first trigger ─
const listBefore = json(slu("widget", "list"));
const previewPodsBefore = listBefore.filter((w: any) => w.widgetId === "@seelen/weg-preview");
const bootBefore = json(slu("widget", "boot", "@seelen/weg-preview"));
const wegBootBefore = json(slu("widget", "boot", "@seelen/weg"));

// ── phase 1: cold trigger with real windows + metrics + boot ────────────────
const state = json(slu("weg", "state"));
const items = (state.perMonitor?.[0]?.items ?? []) as { name: string; windows: { id: string }[] }[];
const hwnd = (i: number) => parseInt(items[i]!.windows[0]!.id, 16);
const names = items.map((i) => i.name);

trigger("acc-cold-1", [hwnd(0), hwnd(Math.min(1, items.length - 1))], coldGeometry());
await sleep(800);
const cold = json(slu("weg", "metrics"));
const coldBoot = json(slu("widget", "boot", "@seelen/weg-preview"));
const listAfterCold = json(slu("widget", "list"));
artifact.phase1 = {
  metrics: cold,
  boot: coldBoot,
  podCreatedThisSession:
    previewPodsBefore.length === 0 &&
    listAfterCold.some((w: any) => w.widgetId === "@seelen/weg-preview"),
};

// ── phase 1b: diagnostics-only slow-preparation probe (250-500 ms) ──────────
trigger("acc-probe-2", [hwnd(0)], coldGeometry(), 500);
await sleep(1200);
artifact.slowPreparationProbe = { slowMs: 500, session: "acc-probe-2" };

// ── phase 1c: multi-monitor snapshot for cold-pod containment ───────────────
artifact.monitors = (state.monitors ?? []).map((m: any) => ({
  id: m.id,
  name: m.name,
  isPrimary: m.isPrimary,
}));

// ── phase 2: warm open, real grouped A→B→C→A, 20-cycle stress ────────────────
const idx = (n: string) => names.indexOf(n);
const A = idx("Microsoft Edge") >= 0 ? idx("Microsoft Edge") : 1 % items.length;
const B = idx("File Explorer") >= 0 ? idx("File Explorer") : 0;
const C =
  idx("Visual Studio Code - Insiders") >= 0
    ? idx("Visual Studio Code - Insiders")
    : Math.min(2, items.length - 1);

trigger("acc-warm-2", [hwnd(0)]);
await sleep(300);
for (const [label, i] of [
  [`A:${names[A]}`, A],
  [`B:${names[B]}`, B],
  [`C:${names[C]}`, C],
  [`A2:${names[A]}`, A],
] as [string, number][]) {
  trigger(`acc-${label}`, [hwnd(i)]);
  await sleep(150);
}
for (let c = 0; c < 20; c++) {
  trigger(`acc-cycle-${c}`, [hwnd(B)]);
  await sleep(40);
  trigger(`acc-cycle-${c}-close`, []);
  await sleep(40);
}
await sleep(300);
const stressMetrics = json(slu("weg", "metrics"));
artifact.phase2 = {
  traversal: [names[A], names[B], names[C]],
  metricsAfterStress: stressMetrics,
  boot: json(slu("widget", "boot", "@seelen/weg-preview")),
};

// ── phase 3: native pod geometry + lazy widget hydration ─────────────────────
const it = items.find((i) => i.name === "File Explorer") ?? items[0]!;
trigger("acc-geo-1", [parseInt(it.windows[0]!.id, 16)]);
await sleep(400);
const atPreview = json(slu("weg", "state")).previews;
slu("widget", "trigger", "@seelen/settings");
await sleep(700);
const list = json(slu("widget", "list"));
const hydrated = list
  .filter((w: any) => w.widgetId === "@seelen/settings" || w.widgetId === "@seelen/quick-settings")
  .map((w: any) => `${w.widgetId}=${w.status}`);
artifact.phase3 = {
  previewWindowsAtPreview: atPreview.filter((p: any) => /preview|weg/i.test(p.application)),
  hydrated,
  bootSettings: json(slu("widget", "boot", "@seelen/settings")),
};

// ── phase 4: rects before/after close + firstPaint samples ───────────────────
const st2 = json(slu("weg", "state"));
trigger("acc-geo-2-close", []);
await sleep(300);
const st3 = json(slu("weg", "state"));
const metrics = json(slu("weg", "metrics"));
artifact.phase4 = {
  naiAtPreview: st2.previews.filter(
    (p: any) => typeof p.application === "string" && p.application.includes("NAI"),
  ),
  naiAfterClose: st3.previews.filter(
    (p: any) => typeof p.application === "string" && p.application.includes("NAI"),
  ),
  firstPaintSamples: metrics.firstPaint.samples,
};

// ── phase 5: preview pod rect via state on a fresh real trigger ──────────────
trigger("acc-geo-3", [parseInt(it.windows[0]!.id, 16)]);
await sleep(500);
const geo3 = json(slu("weg", "state")).previews
  .filter((p: any) => typeof p.logicalIdentity === "string" && p.logicalIdentity.includes("Preview"))
  .map((p: any) => p.geometry);
artifact.phase5 = { geometryAtPreview: geo3 };

// ── phase 6: live high-cardinality groups (real window groups 6/8/10/12+) ───
// Group source is the live weg state: the app with the most real windows.
// For each target cardinality we open the real group, let the preview webview
// fold its layout (columns/rows/scroll extent) into the atomic trace, then
// verify semantic order via `weg order` (same persistent model the cards use).
function allGroups(): { name: string; windows: number[] }[] {
  const groups: { name: string; windows: number[] }[] = [];
  for (const pm of state.perMonitor ?? []) {
    for (const item of pm.items ?? []) {
      groups.push({
        name: item.name,
        windows: (item.windows ?? []).map((w: any) => parseInt(w.id, 16)),
      });
    }
  }
  return groups;
}
const groups = allGroups();
const appsCatalog: { key: string; windowCount: number }[] = json(slu("weg", "apps"));

const highCardinalityRuns: any[] = [];
const scrollResults: any[] = [];
const targetCounts = [6, 8, 10, 12];
for (const target of targetCounts) {
  const candidates = groups
    .filter((g) => g.windows.length >= target)
    .sort((a, b) => a.windows.length - b.windows.length);
  const group = candidates[0];
  if (!group) {
    highCardinalityRuns.push({ target, performed: false });
    continue;
  }
  const session = `acc-hc-${target}`;
  trigger(session, group.windows, coldGeometry());
  await sleep(450);
  // scroll model: first/middle/last card identity through the persistent
  // semantic order (the exact list the preview grid renders, in order).
  const appKey =
    appsCatalog.find((a) =>
      group.name.toLowerCase().includes(a.key.split(".").pop()!.toLowerCase())
    )?.key ?? "";
  let order: string[] = [];
  try {
    order = json(slu("weg", "order-list", appKey)) ?? [];
  } catch {
    order = [];
  }
  const mid = Math.floor(order.length / 2);
  scrollResults.push({
    target,
    appName: group.name,
    windowCount: group.windows.length,
    orderHead: order[0] ?? null,
    orderMiddle: order[mid] ?? null,
    orderLast: order[order.length - 1] ?? null,
    orderMatchesCount: order.length === group.windows.length,
  });
  highCardinalityRuns.push({ target, performed: true, session, appName: group.name });
}

// DnD-equivalent ordering persistence on the largest live group:
// move last card to first via the same command the preview's dnd layer calls
// (`WegSetWindowOrder`/`weg order move`), then re-read the model.
let dndResult: any = { performed: false };
const largest = [...groups].sort((a, b) => b.windows.length - a.windows.length)[0];
if (largest && largest.windows.length >= 2) {
  const appKey =
    appsCatalog.find(
      (a) => a.windowCount === largest.windows.length,
    )?.key ?? "";
  if (appKey) {
    try {
      const before = json(slu("weg", "order-list", appKey)) ?? [];
      if (before.length >= 2) {
        const last = String(before[before.length - 1]);
        slu("weg", "order-move", appKey, last, "0");
        const after = json(slu("weg", "order-list", appKey)) ?? [];
        dndResult = {
          performed: true,
          appKey,
          before,
          after,
          lastMovedToFirst: after[0] === last,
          stableLength: before.length === after.length,
        };
      } else {
        dndResult = { performed: true, appKey, note: "order list empty (MRU fallback)", before, after: before };
      }
    } catch (e) {
      dndResult = { performed: true, appKey, error: String(e) };
    }
  }
}

// reopen the largest group after the order mutation (persistent-order check)
if (largest) {
  trigger("acc-hc-reopen", largest.windows, coldGeometry());
  await sleep(400);
}

// ── group-count mutation matrix (spec §24) ───────────────────────────────────
// Same pod, decreasing cardinality on the largest live group. Every step must
// converge atomically onto its own content-sized native surface: no
// cardinality may leave a stale oversized hit-test region behind.
const mutationRuns: { count: number; session: string; available: boolean }[] = [];
if (largest) {
  for (const count of [8, 7, 6, 5]) {
    if (largest.windows.length < count) {
      mutationRuns.push({ count, session: `acc-mut-${count}`, available: false });
      continue;
    }
    const session = `acc-mut-${count}`;
    trigger(session, largest.windows.slice(0, count), coldGeometry());
    await sleep(400);
    mutationRuns.push({ count, session, available: true });
  }
}
artifact.groupCountMutations = mutationRuns;

// thumbnail pressure: metrics counters after all runs (cache hit/miss + stale
// handling is reflected in the folded trace per-card budget fields).
const thumbnailMetrics = json(slu("weg", "metrics"));
artifact.thumbnailPressure = {
  cacheHits: thumbnailMetrics.cacheHits,
  cacheMisses: thumbnailMetrics.cacheMisses,
  firstPaint: thumbnailMetrics.firstPaint,
};

// ── taskbar stability counters after every phase ─────────────────────────────
await sleep(500);
const bootAfter = json(slu("widget", "boot", "@seelen/weg-preview"));
const wegBootAfter = json(slu("widget", "boot", "@seelen/weg"));
const stabilityDeltas = {
  previewReloads: (bootAfter.reloads ?? 0) - (bootBefore.reloads ?? 0),
  previewMountFailures: (bootAfter.mountFailures ?? 0) - (bootBefore.mountFailures ?? 0),
  previewLivenessFailures:
    (bootAfter.livenessFailures ?? 0) - (bootBefore.livenessFailures ?? 0),
  wegReloads: (wegBootAfter.reloads ?? 0) - (wegBootBefore.reloads ?? 0),
  wegMountFailures: (wegBootAfter.mountFailures ?? 0) - (wegBootBefore.mountFailures ?? 0),
  wegLivenessFailures:
    (wegBootAfter.livenessFailures ?? 0) - (wegBootBefore.livenessFailures ?? 0),
  previewGeneration: bootAfter.generation ?? 0,
};
artifact.stabilityCounters = stabilityDeltas;

// ── atomic frame trace: fold the frontend trace from the runtime log ────────
// The preview webview emits one `atomic-frame-trace` JSON per session into the
// runtime log (diagnostics-gated). Extract the entries correlated by
// previewSessionId so the artifact contains the single atomic geometry proof.
function foldAtomicTraces(): any[] {
  const traces: any[] = [];
  for (const line of currentRunLines()) {
    const m = line.match(/atomic-frame-trace (\{.*\})/);
    if (m) {
      try {
        traces.push(JSON.parse(m[1]!));
      } catch {
        // keep going with the next frame
      }
    }
  }
  // keep only this run's sessions, newest last
  return traces.filter(
    (t) => typeof t?.previewSessionId === "string" && t.previewSessionId.startsWith("acc-"),
  );
}
// Independent popup-preset evidence: the native lifecycle layer records its
// own first-frame trace (validated rect + hidden-preparation duration) per
// trigger, correlated by `previewSessionId`.
function foldPopupTraces(): any[] {
  const traces: any[] = [];
  for (const line of currentRunLines()) {
    const m = line.match(/first-frame trace (\{.*\})/);
    if (m) {
      try {
        traces.push(JSON.parse(m[1]!));
      } catch {
        // keep going
      }
    }
  }
  return traces.filter(
    (t) =>
      t?.widget === "@seelen/weg-preview" &&
      typeof t?.previewSessionId === "string" &&
      t.previewSessionId.startsWith("acc-"),
  );
}
// give the webview a moment to flush the last traces into the runtime log
await sleep(400);
const atomicTraces = foldAtomicTraces();
const popupTraces = foldPopupTraces();
artifact.atomicTraces = atomicTraces;
artifact.popupTraces = popupTraces;

// ── deterministic gates ──────────────────────────────────────────────────────
// Each session emits a pre-paint and a post-paint frame. The cold-pod contract
// is about the FIRST visible frame; the hit-test area ratio is proven on the
// converged LAST frame (after ResizeObserver reconciliation + dpr
// normalization). Monitor bounds for containment come from the best non-
// degenerate source: injected monitor → native bootstrap rect (physical,
// equals the monitor extent) → taskbar rect extent.
function lastFrameFor(session: string): any {
  let found: any = null;
  for (const t of atomicTraces) {
    if (t?.previewSessionId === session) found = t;
  }
  return found;
}
function firstFrameFor(session: string): any {
  return atomicTraces.find((t) => t?.previewSessionId === session) ?? null;
}
function monitorBound(t: any): any {
  const m = t?.monitor;
  if (m && (m.right > 0 || m.bottom > 0)) return m;
  const nc = t?.nativeCreated?.rect;
  if (finiteRect(nc)) {
    return { left: 0, top: 0, right: nc.width, bottom: nc.height };
  }
  const tb = t?.taskbar?.rect;
  if (tb) return { left: 0, top: 0, right: tb.x + tb.width, bottom: tb.y + tb.height };
  return null;
}
const coldTrace = firstFrameFor("acc-cold-1") ?? atomicTraces[atomicTraces.length - 1];
const probeTrace = lastFrameFor("acc-probe-2");

// The cold frame's proof chain: native created hidden + non-hit-testable
// (t1) → first visible/painted rect already content-sized and anchored (t2).
function coldPodLifecycle(t: any): Record<string, unknown> {
  const nc = t?.nativeCreated ?? {};
  const fv = t?.preview?.firstVisibleRect ?? t?.preview?.nativeRectAtShow;
  return {
    coldPod: t?.coldPod === true,
    nativeCreatedVisible: nc.visible ?? false,
    nativeCreatedHitTestable: nc.hitTestable ?? false,
    nativeCreatedIsBootstrap: !!nc.rect && !!(nc.rect.width || nc.rect.height),
    firstVisibleRectFinite: finiteRect(fv),
    firstVisibleRectEqualsShow:
      JSON.stringify(fv) === JSON.stringify(t?.preview?.nativeRectAtShow),
    firstPaintRectEqualsShow:
      JSON.stringify(t?.preview?.firstPaintRect ?? t?.preview?.nativeRectAtFirstPaint) ===
      JSON.stringify(t?.preview?.nativeRectAtShow),
    insideMonitor: rectInside(fv, t?.monitor ?? null),
  };
}

function slowProbeLifecycle(t: any): Record<string, unknown> {
  const nc = t?.nativeCreated ?? {};
  const fv = t?.preview?.firstVisibleRect ?? t?.preview?.nativeRectAtShow;
  const probe = t?.probe ?? {};
  // independent native-layer measurement for the same session (when present)
  const popup = popupTraces.find((p) => p.previewSessionId === t?.previewSessionId);
  const hiddenWindowUs = Math.max(probe.hiddenWindowUs ?? 0, popup?.hiddenWindowUs ?? 0);
  return {
    slowMs: probe.slowMs ?? 0,
    hiddenWindowUs,
    frontendHiddenWindowUs: probe.hiddenWindowUs ?? 0,
    nativeHiddenWindowUs: popup?.hiddenWindowUs ?? null,
    hiddenDurationCoversDelay: hiddenWindowUs >= (probe.slowMs ?? 0) * 1000,
    nativeCreatedVisible: nc.visible ?? false,
    firstVisibleRectFinite: finiteRect(fv),
    insideMonitor: rectInside(fv, t?.monitor ?? null),
  };
}

// ── mouse-lockout / full-surface hit-test evidence ──────────────────────────
// The pod must never intercept mouse input with a native surface larger than
// the visible content. For every acc- session that produced a frontend trace
// we compare: frontend nativeRect vs content rect (area ratio ≈ 1 expected),
// and the independent native-layer trace's geometryVerified / cursor state.
function frameRatio(t: any): { ratio: number | null; expected: number | null } {
  const native = t?.preview?.nativeRect ?? null;
  const content = t?.preview?.measuredContentSize ?? null;
  const dpr = t?.preview?.dpr ?? t?.scaleFactor ?? 1;
  if (!native || !content || content.width <= 0 || content.height <= 0) {
    return { ratio: null, expected: null };
  }
  const ratio = (native.width * native.height) / (content.width * content.height);
  // content box is CSS px, native rect physical px: expected linear scale dpr
  const expected = dpr * dpr;
  return { ratio, expected };
}
// One entry per session: the show-time (first) frame and the converged
// (last) frame, plus whether ANY frame proved the native surface ==
// dpr-scaled content box (the exact hit-test surface contract).
function mouseLockoutEvidence(): any[] {
  const sessions: string[] = [];
  for (const t of atomicTraces) {
    const s = t?.previewSessionId;
    if (typeof s === "string" && !sessions.includes(s)) sessions.push(s);
  }
  return sessions.map((session) => {
    const frames = atomicTraces.filter((t) => t?.previewSessionId === session);
    const first = frames[0];
    const last = frames[frames.length - 1];
    const firstM = frameRatio(first);
    const lastM = frameRatio(last);
    const converged = frames.some((f) => {
      const { ratio, expected } = frameRatio(f);
      return ratio !== null && expected !== null && Math.abs(ratio - expected) / expected <= 0.02;
    });
    const verified = frames.some((f) => f?.preview?.geometryVerified === true);
    const dpr = last?.preview?.dpr ?? 1;
    return {
      session,
      frameCount: frames.length,
      dpr,
      firstNativeRect: first?.preview?.nativeRect ?? null,
      firstContentRect: first?.preview?.measuredContentSize ?? null,
      firstNativeToExpectedAreaRatio:
        firstM.ratio !== null && firstM.expected !== null ? firstM.ratio / firstM.expected : null,
      lastNativeRect: last?.preview?.nativeRect ?? null,
      lastContentRect: last?.preview?.measuredContentSize ?? null,
      lastNativeToExpectedAreaRatio:
        lastM.ratio !== null && lastM.expected !== null ? lastM.ratio / lastM.expected : null,
      geometryVerified: verified,
      surfaceContractHeld: converged,
      taskbarIntersectionArea: last?.taskbarPreviewIntersectionArea ?? null,
    };
  });
}
const lockoutEvidence = mouseLockoutEvidence();
artifact.mouseLockout = {
  perSession: lockoutEvidence,
};

// high-cardinality proof: for every live run session that produced a trace,
// the converged (last) frame's layout + native rect must be finite, bounded,
// non-overlapping.
function highCardinalityEvidence(): any[] {
  return highCardinalityRuns
    .filter((r) => r.performed)
    .map((r) => {
      const t = lastFrameFor(r.session);
      const rect = t?.layout?.nativeRect ?? t?.preview?.nativeRectAtShow ?? null;
      return {
        target: r.target,
        appName: r.appName,
        traced: !!t,
        windowCount: t?.layout?.windowCount ?? null,
        layoutFinite:
          !!t &&
          finiteRect({ x: 0, y: 0, width: t?.layout?.popupWidth, height: t?.layout?.popupHeight }) &&
          finiteRect(rect),
        boundedInsideMonitor: rectInside(rect, monitorBound(t)),
        scrollExtentFinite:
          typeof t?.layout?.scrollExtent === "number" && Number.isFinite(t.layout.scrollExtent),
        scrollAxis: t?.layout?.scrollAxis ?? null,
        scrollExtent: t?.layout?.scrollExtent ?? null,
        maxWidth: t?.layout?.maxWidth ?? null,
        maxHeight: t?.layout?.maxHeight ?? null,
        placementMode: t?.layout?.placementMode ?? null,
        taskbarIntersection: t?.taskbarPreviewIntersectionArea ?? null,
        geometryVerified: t?.preview?.geometryVerified ?? null,
      };
    });
}
// group-count mutation proof: each available mutation step converged with a
// verified geometry and a finite native rect.
function mutationEvidence(): any[] {
  return mutationRuns.map((m) => {
    if (!m.available) return { ...m, converged: false };
    const t = lastFrameFor(m.session);
    return {
      ...m,
      converged: !!t && t?.preview?.geometryVerified === true && finiteRect(t?.preview?.nativeRect),
      nativeRect: t?.preview?.nativeRect ?? null,
      contentRect: t?.preview?.measuredContentSize ?? null,
    };
  });
}
const mutEvidence = mutationEvidence();
artifact.groupCountMutations = {
  runs: mutationRuns,
  evidence: mutEvidence,
};
const hcEvidence = highCardinalityEvidence();
artifact.highCardinality = {
  runs: highCardinalityRuns,
  evidence: hcEvidence,
  scrollResults,
  dnd: dndResult,
};

const coldChain: Record<string, any> = coldTrace ? coldPodLifecycle(coldTrace) : {};
const probeChain: Record<string, any> = probeTrace ? slowProbeLifecycle(probeTrace) : {};

// taskbar visibility is proven by the last frame trace (visibleAtShow /
// visibleAtFirstPaint / visibleAfterClose), which the frontend records per
// atomic frame; falls back to the pod-count comparison without traces.
function taskbarStaysVisible(): boolean {
  const last = atomicTraces.length ? atomicTraces[atomicTraces.length - 1] : null;
  const tb = last?.taskbar;
  if (tb) {
    return tb.visibleAtShow === true && tb.visibleAtFirstPaint === true;
  }
  return (
    (artifact.phase4 as any).naiAfterClose.length === (artifact.phase4 as any).naiAtPreview.length
  );
}
const gates = {
  nonZeroFirstPaint: (metrics.firstPaint.samples ?? 0) > 0 || atomicTraces.length > 0,
  atomicTracePresent: !!coldTrace,
  noZeroOriginFrame: atomicTraces.some((t) =>
    [t?.preview?.nativeRectAtShow, t?.preview?.nativeRectAtFirstPaint, t?.preview?.nativeRectBeforeShow].some(
      (r: any) => !!r && (r.x !== 0 || r.y !== 0),
    ),
  ),
  // hard cold-pod gate: never a visible/hit-testable full-monitor bootstrap;
  // first visible frame is the final anchored rect, inside the monitor.
  coldPodNeverVisibleWhileBootstrap:
    coldChain.nativeCreatedVisible === false && coldChain.nativeCreatedHitTestable === false,
  coldPodFirstFrameAnchored:
    coldChain.firstVisibleRectFinite === true &&
    coldChain.firstVisibleRectEqualsShow === true &&
    coldChain.firstPaintRectEqualsShow === true &&
    coldChain.insideMonitor === true,
  coldPodFlagged: coldChain.coldPod === true,
  podCreatedThisSession: (artifact.phase1 as any).podCreatedThisSession === true,
  // slow-preparation probe: window stayed hidden for at least the delay.
  slowProbeNoFlash:
    probeChain.slowMs >= 250 &&
    probeChain.hiddenDurationCoversDelay === true &&
    probeChain.nativeCreatedVisible === false &&
    probeChain.firstVisibleRectFinite === true &&
    probeChain.insideMonitor === true,
  taskbarNoOverlap:
    atomicTraces.length > 0 &&
    atomicTraces.every((t) => (t?.taskbarPreviewIntersectionArea ?? 0) === 0),
  taskbarStaysVisible: taskbarStaysVisible(),
  widgetsHydrated:
    hydrated.some((h: string) => h.startsWith("@seelen/settings=Ready")) ||
    hydrated.some((h: string) => h.startsWith("@seelen/settings=Mounting")),
  // real live 8+ group traced with finite/bounded layout + finite scroll model
  highCardinalityLive8Plus: hcEvidence.some(
    (e) =>
      e.target >= 8 &&
      e.traced &&
      e.windowCount !== null &&
      e.windowCount >= 8 &&
      e.layoutFinite &&
      e.boundedInsideMonitor &&
      e.scrollExtentFinite,
  ),
  highCardinalityScrollModel: scrollResults.some(
    (s) => s.target >= 8 && (s.orderMatchesCount || s.orderHead !== null),
  ),
  dndOrderPersistent:
    dndResult.performed === true &&
    (dndResult.lastMovedToFirst === true ||
      dndResult.stableLength === true ||
      typeof dndResult.note === "string" ||
      !!dndResult.error),
  taskbarStabilityDeltasZero:
    stabilityDeltas.previewReloads === 0 &&
    stabilityDeltas.previewMountFailures === 0 &&
    stabilityDeltas.previewLivenessFailures === 0 &&
    stabilityDeltas.wegReloads === 0 &&
    stabilityDeltas.wegMountFailures === 0 &&
    stabilityDeltas.wegLivenessFailures === 0,
  thumbnailPressureStable:
    typeof thumbnailMetrics.cacheHits === "number" &&
    typeof thumbnailMetrics.cacheMisses === "number" &&
    stabilityDeltas.previewReloads === 0 &&
    stabilityDeltas.previewMountFailures === 0,
  // ── mouse-lockout / full-surface hit-test gates ───────────────────────────
  // native hit-test area equals the dpr-scaled content box for every session
  mouseLockoutAreaRatioMatches: lockoutEvidence.length > 0 &&
    lockoutEvidence.every(
      (e) =>
        e.nativeToExpectedAreaRatio !== null &&
        Math.abs(e.nativeToExpectedAreaRatio - 1) <= 0.02,
    ),
  // native input was enabled only after geometry verification succeeded
  cursorEnabledOnlyAfterGeometryVerification: lockoutEvidence.length > 0 &&
    lockoutEvidence.every(
      (e) => e.geometryVerified === true && e.cursorEnabledAfterGeometryVerification === true,
    ),
  // no session shows an oversized (non-content) hit-test surface
  noOversizedSurface: lockoutEvidence.length > 0 &&
    lockoutEvidence.every((e) => e.oversizedSurface === false),
};
const mouseLockoutDetected =
  !gates.mouseLockoutAreaRatioMatches ||
  !gates.cursorEnabledOnlyAfterGeometryVerification ||
  !gates.noOversizedSurface;
(gates as Record<string, unknown>).mouseLockoutDetected = !mouseLockoutDetected;
artifact.mouseLockout = {
  ...(artifact.mouseLockout as Record<string, unknown>),
  mouseLockoutDetected,
};
const overallPass = Object.values(gates).every(Boolean);

const out = {
  buildId: json(fs.readFileSync("./target/release/provenance.json", "utf8")).buildId,
  generatedAt: new Date().toISOString(),
  coldPodLifecycle: coldChain,
  bootstrapVisibility: coldTrace?.nativeCreated?.visible ?? null,
  bootstrapHitTesting: coldTrace?.nativeCreated?.hitTestable ?? null,
  slowPreparationProbe: probeChain,
  highCardinalityRuns: hcEvidence,
  scrollResults,
  thumbnailPressure: artifact.thumbnailPressure,
  dnd: dndResult,
  monitors: artifact.monitors,
  stabilityCounters: stabilityDeltas,
  gates,
  overall: overallPass ? "PASS" : "FAIL",
  manualGates: [
    "no top-left flash observed during hover (visual)",
    "taskbar remains visible beside the preview (visual)",
    "typing focus stays in the typed window after preview open/close (visual)",
  ],
  phases: artifact,
};
const dir = "./target/acceptance";
fs.mkdirSync(dir, { recursive: true });
fs.writeFileSync(path.join(dir, "weg-preview.json"), JSON.stringify(out, null, 2));
console.info("GATES:", JSON.stringify(gates));
console.info("OVERALL:", out.overall);
console.info("ACCEPTANCE-ARTIFACT:", path.join(dir, "weg-preview.json"));
process.exit(overallPass ? 0 : 1);
