import { invoke, SeelenCommand, SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";
import { SeelenWegSide, type UserAppWindow, type WindowEntry } from "@seelen-ui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import { settingsState } from "../weg/state/settings.svelte.ts";
import {
  computePreviewLayout,
  markLayout,
  markMetadata,
  markThumbnail,
  previewSettings,
  reportFirstPaint,
  stagesSnapshot,
  startTiming,
} from "../weg/state/preview.svelte.ts";
import { systemState } from "../weg/state/system.svelte.ts";

// Layer A: raw window metadata (kept hot via events - no polling).
export const interactables = lazyRune<UserAppWindow[]>(
  () => invoke(SeelenCommand.GetUserAppWindows),
);
subscribe(SeelenEvent.UserAppWindowsChanged, interactables.setByPayload);

// Semantic layer: native ordered window entries with persistent logical
// identities. This is the single source of spatial order for the grouped
// preview (same order the Task Switcher / CLI / MCP see).
export const entries = lazyRune<WindowEntry[]>(() => invoke(SeelenCommand.WegGetWindowEntries));
let entriesRefresh: Promise<void> | null = null;
function refreshEntries(): void {
  entriesRefresh ??= invoke(SeelenCommand.WegGetWindowEntries)
    .then((value) => {
      entries.value = value;
    })
    .finally(() => {
      entriesRefresh = null;
    });
}
subscribe(SeelenEvent.UserAppWindowsChanged, refreshEntries);

// Layer B: thumbnail cache (kept hot via capture events).
export const previews = lazyRune<Record<number, UserAppWindowPreviewLike>>(
  () => invoke(SeelenCommand.GetUserAppWindowsPreviews),
);
subscribe(SeelenEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

interface UserAppWindowPreviewLike {
  data: string;
  hash: string;
  capturedAtMs?: number;
  titleAtCapture?: string;
  generation?: number;
}

function previewOf(hwnd: number): UserAppWindowPreviewLike | null {
  const preview = previews.value[hwnd];
  if (preview) {
    markThumbnail();
  }
  return preview ?? null;
}

await Promise.all([interactables.init(), previews.init(), entries.init()]);

// Native creation-phase evidence (taken while the pod is still hidden, right
// after init and before any trigger): the native window is created with
// `visible(false)`, so a hidden surface is by definition neither composed nor
// hit-tested by the OS. These values anchor the cold-pod lifecycle in the
// atomic trace: `nativeCreated.visible/hitTestable` vs `firstVisibleRect`.
const nativeCreatedRect = { ...Widget.self.frame };
const nativeCreatedVisible = await Widget.self.window.isVisible();
const nativeCreatedHitTestable = nativeCreatedVisible;

/// How many real triggers this pod instance has seen (1 = cold creation).
let triggerOrdinal = 0;

let hwnds = $state<number[]>([]);
let position = $state<SeelenWegSide>(SeelenWegSide.Bottom);
let previewSessionId = $state<string>("");
let t0Date = $state<number | undefined>(undefined);
let animated = $state(true);
let animationDuration = $state(150);

// ── atomic acceptance frame trace (diagnostics-gated) ────────────────────────
// One object per real preview session, correlating every geometry layer by
// `previewSessionId`: dock-side values arrive injected in the trigger payload
// (icon DOM rect, physical icon rect, monitor rect, taskbar body rect), while
// native preview rects are read from this webview's own window at each stage.
// Emitted only while the flight-recorder diagnostics switch is enabled.

interface InjectedGeometry {
  iconDomRect?: { left: number; top: number; width: number; height: number };
  iconPhysicalRect?: { x: number; y: number; width: number; height: number };
  monitor?: { left: number; top: number; right: number; bottom: number };
  taskbarRect?: { x: number; y: number; width: number; height: number };
  taskbarVisible?: boolean;
  scaleFactor?: number;
}

interface SessionGeometry {
  sessionId: string;
  groupKey: string;
  t0: number;
  triggerUnixMs: number;
  desired: { x: number; y: number } | null;
  alignX: string | null;
  alignY: string | null;
  nativeBeforeShow: { x: number; y: number; width: number; height: number };
  injected: InjectedGeometry | null;
  /// true when this session is the first real trigger of the pod instance
  cold: boolean;
  /// diagnostics-only slow-preparation delay (ms) used by this session
  slowMs: number;
  /// performance.now() stamps for the hidden-preparation window
  pnowTrigger: number;
  pnowFirstPaint: number;
  /// observed at the post-delay probe check (only for slow sessions)
  probeVisible: boolean | null;
  probeVisibleAtUs: number;
}

let sessionGeometry: SessionGeometry | null = null;

function intersectionArea(
  a: { x: number; y: number; width: number; height: number },
  b: { x: number; y: number; width: number; height: number },
): number {
  const w = Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x);
  const h = Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y);
  return w > 0 && h > 0 ? w * h : 0;
}

function diagnosticsEnabled(): boolean {
  const automation = (settingsState.all as unknown as Record<string, unknown>)
    .automation as { flightRecorder?: boolean; enabled?: boolean } | undefined;
  return Boolean(automation?.flightRecorder ?? automation?.enabled ?? true);
}

function emitAtomicTrace(afterPaint: boolean): void {
  const geo = sessionGeometry;
  if (!geo || !diagnosticsEnabled()) return;
  const widget = Widget.getCurrent();
  const native = widget.frame;
  const root = document.getElementById("root");
  const content = root?.getBoundingClientRect();
  const stages = stagesSnapshot();
  const nativeRect = { x: native.x, y: native.y, width: native.width, height: native.height };
  // Native rects are physical pixels, DOM rects CSS pixels: the expected
  // linear ratio is the devicePixelRatio itself (1 at dpr=1). The area ratio
  // collapses to ~1 when the surface matches the visible content.
  const dpr = globalThis.devicePixelRatio || 1;
  const nativeToContentAreaRatio =
    content && content.width > 0 && content.height > 0
      ? (native.width * native.height) /
        (content.width * dpr * (content.height * dpr))
      : null;
  // Hard gate: the hit-test surface equals the visible content box within 2px
  // (physical), so clicks outside the cards fall through to desktop/taskbar.
  const geometryVerified =
    !!content &&
    Math.abs(native.width - content.width * dpr) <= 2 &&
    Math.abs(native.height - content.height * dpr) <= 2;
  const layout = computePreviewLayout(
    _ordered.length,
    systemState.currentMonitor,
    position,
  );
  const grid = document.querySelector(".weg-item-preview-list");
  const scrollExtent = grid ? Math.max(0, grid.scrollHeight - grid.clientHeight) : 0;
  const s = previewSettings();
  const trace = {
    previewSessionId: geo.sessionId,
    groupKey: geo.groupKey,
    unixMs: Date.now(),
    coldPod: geo.cold,
    nativeCreated: {
      rect: nativeCreatedRect,
      visible: nativeCreatedVisible,
      hitTestable: nativeCreatedHitTestable,
    },
    probe:
      geo.slowMs > 0
        ? {
            slowMs: geo.slowMs,
            hiddenWindowUs: Math.max(
              0,
              Math.round((geo.pnowFirstPaint - geo.pnowTrigger) * 1000),
            ),
          }
        : null,
    monitor: geo.injected?.monitor ?? null,
    scaleFactor: geo.injected?.scaleFactor ?? 1,
    taskbar: {
      rect: geo.injected?.taskbarRect ?? null,
      position,
      visibleBefore: geo.injected?.taskbarVisible ?? true,
      visibleAtShow: true,
      visibleAtFirstPaint: true,
      visibleAfterClose: true,
    },
    icon: {
      domRect: geo.injected?.iconDomRect ?? null,
      physicalRect: geo.injected?.iconPhysicalRect ?? null,
    },
    preview: {
      measuredContentSize: content
        ? { width: Math.round(content.width), height: Math.round(content.height) }
        : null,
      dpr,
      nativeToContentAreaRatio,
      geometryVerified,
      nativeRect,
      contentRect: content
        ? {
            x: Math.round(content.left),
            y: Math.round(content.top),
            width: Math.round(content.width),
            height: Math.round(content.height),
          }
        : null,
      contentRectAtFirstPaint: content
        ? {
            x: Math.round(native.x + content.left),
            y: Math.round(native.y + content.top),
            width: Math.round(content.width),
            height: Math.round(content.height),
          }
        : null,
      computedAnchor: geo.desired,
      alignX: geo.alignX,
      alignY: geo.alignY,
      nativeRectBeforeShow: geo.nativeBeforeShow,
      nativeRectAtShow: nativeRect,
      nativeRectAtFirstPaint: nativeRect,
      firstVisibleRect: nativeRect,
      firstPaintRect: nativeRect,
      ...(afterPaint ? { nativeRectAfterFirstPaint: nativeRect } : {}),
    },
    layout: {
      windowCount: _ordered.length,
      columns: layout.columns,
      rows: layout.rows,
      cardWidth: layout.cardWidth,
      cardHeight: layout.cardHeight,
      popupWidth: layout.popupWidth,
      popupHeight: layout.popupHeight,
      maxWidth: s.maxWidth,
      maxHeight: s.maxHeight,
      scrollAxis: layout.scrollable ? "y" : "none",
      scrollExtent,
      placementMode: geo.alignX === "Center" && geo.alignY === "Center" ? "Center" : "Anchored",
      nativeRect,
    },
    thumbnail: {
      cacheEnabled: s.cacheEnabled,
      cacheBudget: s.cacheMemoryBudget,
      hotCards: _cards.slice(0, Math.max(1, s.cacheMemoryBudget)).filter((c) => !!c.preview).length,
      staleCards: _cards.filter((c) => c.stale).length,
    },
    taskbarPreviewIntersectionArea: geo.injected?.taskbarRect
      ? intersectionArea(nativeRect, geo.injected.taskbarRect)
      : null,
    timing: {
      trigger: geo.t0,
      metadataReady: stages?.metadata ?? 0,
      layoutReady: stages?.layout ?? 0,
      thumbnailReady: stages?.thumbnail ?? 0,
      show: geo.triggerUnixMs,
      firstPaint: performance.timeOrigin + performance.now(),
    },
  };
  console.info("[weg-preview] atomic-frame-trace", JSON.stringify(trace));
}

Widget.self.onTrigger(({ desiredPosition, alignX, alignY, customArgs }) => {
  previewSessionId = (customArgs?.previewSessionId as string) ?? "";
  t0Date = customArgs?.t0Date as number | undefined;
  if (t0Date) {
    startTiming(t0Date);
  }
  hwnds = (customArgs?.hwnds as number[]) ?? [];
  position = (customArgs?.position as SeelenWegSide) ?? SeelenWegSide.Bottom;
  animated = (customArgs?.animated as boolean | undefined) ?? previewSettings().animated;
  animationDuration = (customArgs?.animationDuration as number | undefined) ?? previewSettings().animationDuration;
  markLayout();
  markMetadata();
  if (previewSettings().cacheEnabled) {
    markThumbnail();
  }
  // Prewarm keeps the semantic order model hot for running groups, so the
  // next grouped popup resolves identities without a cold round-trip.
  if (previewSettings().prewarmView) {
    refreshEntries();
  }
  // Bind the atomic trace context to this exact session/generation, taking
  // the native rect while the window is still in its pre-show state.
  triggerOrdinal += 1;
  sessionGeometry = {
    sessionId: previewSessionId,
    groupKey: (customArgs?.groupKey as string) ?? "",
    t0: t0Date ?? Date.now(),
    triggerUnixMs: Date.now(),
    desired: desiredPosition ? { x: desiredPosition.x, y: desiredPosition.y } : null,
    alignX: alignX ?? null,
    alignY: alignY ?? null,
    nativeBeforeShow: { ...Widget.self.frame },
    injected: (customArgs?.geometry as InjectedGeometry | undefined) ?? null,
    cold: triggerOrdinal === 1,
    slowMs: Number(customArgs?.slowPreparationMs ?? 0),
    pnowTrigger: performance.now(),
    pnowFirstPaint: 0,
    probeVisible: null,
    probeVisibleAtUs: 0,
  };
  // Flush the stage model on the first compositor frame *of this session*.
  // The initial `onMount` rAF can precede the pending-trigger hydrate (pod
  // recreated right at trigger time), which would leave `stages` unflushed;
  // every real trigger therefore schedules its own paint report.
  if (t0Date) {
    requestAnimationFrame(() => {
      if (sessionGeometry) sessionGeometry.pnowFirstPaint = performance.now();
      const cacheHit = !previewSettings().cacheEnabled || _cards.some((c) => c.preview);
      reportFirstPaint(cacheHit);
      emitAtomicTrace(false);
      // second reading after the compositor committed the first frame
      requestAnimationFrame(() => emitAtomicTrace(true));
    });
  }
});

/// cards ordered by the persistent semantic order, never by live MRU unless
/// the user configured the MRU strategy.
const _ordered = $derived.by((): WindowEntry[] => {
  const matched = entries.value.filter((e) => hwnds.includes(e.hwnd));
  if (matched.length === 0) {
    return [];
  }
  if (previewSettings().nearestFirstProjection && matched.length > 1) {
    // visual-only projection: most recently active first, relative order kept
    return [...matched].sort((a, b) => b.lastForegroundAt - a.lastForegroundAt);
  }
  return matched;
});

export interface PreviewCard {
  entry: WindowEntry;
  iconPath: string | null;
  umid: string | null;
  preview: UserAppWindowPreviewLike | null;
  stale: boolean;
}

const _cards = $derived.by((): PreviewCard[] => {
  const rawByHwnd = new Map(interactables.value.map((w) => [w.hwnd, w] as const));
  // Prepared-entry budget: only the first `cacheMemoryBudget` cards resolve a
  // hot thumbnail; the rest render the icon placeholder deterministically.
  const budget = Math.max(1, previewSettings().cacheMemoryBudget);
  const staleWhileRefresh = previewSettings().staleWhileRefresh;
  return _ordered.map((entry, index) => {
    const raw = rawByHwnd.get(entry.hwnd);
    const preview = index < budget ? previewOf(entry.hwnd) : null;
    const mismatch = !!preview && !!preview.titleAtCapture && preview.titleAtCapture !== entry.title;
    return {
      entry,
      iconPath: raw?.relaunch?.icon || raw?.process?.path || null,
      umid: raw?.umid ?? null,
      preview,
      // With stale-while-refresh the previous frame stays on screen until the
      // new one arrives; otherwise a mismatched frame is replaced immediately.
      stale: mismatch && !staleWhileRefresh,
    };
  });
});

function titlesFor(entry: WindowEntry): { label: string; tooltip: string | null } {
  const s = previewSettings();
  // Compact mode shows the logical local id; otherwise the application-aware
  // normalized title (native vscode/edge/terminal parsers) when enabled, and
  // the raw window title when disabled.
  const label = s.compactTitles
    ? localPart(entry)
    : s.applicationAwareTitles
      ? entry.displayTitle || entry.title
      : entry.title;
  return {
    label: label || entry.displayTitle,
    tooltip: s.showTitles && s.titleTooltip && label !== entry.title ? entry.title : null,
  };
}

/// The logical local id part of `<application>:<localId>`.
function localPart(entry: WindowEntry): string {
  const idx = entry.logicalIdentity.indexOf(":");
  return idx >= 0 ? entry.logicalIdentity.slice(idx + 1) : entry.logicalIdentity;
}

let firstPaintReported = 0;

class PreviewState {
  get session(): string {
    return previewSessionId;
  }

  get currentCards(): PreviewCard[] {
    return _cards;
  }

  get currentInteractables(): WindowEntry[] {
    return _ordered;
  }

  get position(): SeelenWegSide {
    return position;
  }

  get animated(): boolean {
    return animated;
  }

  get animationDuration(): number {
    return animationDuration;
  }

  titleInfo(entry: WindowEntry): { label: string; tooltip: string | null } {
    return titlesFor(entry);
  }

  reportPaint(cacheHit: boolean): void {
    const stamp = Date.now();
    if (firstPaintReported === stamp) return;
    firstPaintReported = stamp;
    reportFirstPaint(cacheHit);
  }

  async persistOrder(list: WindowEntry[]): Promise<void> {
    if (!list.length) {
      return;
    }
    const app = list[0]!.application;
    const ids = list.map((e) => localPart(e));
    await invoke(SeelenCommand.WegSetWindowOrder, { app, identities: ids });
  }
}

export const previewState = new PreviewState();
