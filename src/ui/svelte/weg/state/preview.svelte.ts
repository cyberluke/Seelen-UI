import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { UserAppWindow } from "@seelen-ui/lib/types";
import { settingsState } from "./settings.svelte.ts";

export interface PreviewLayout {
  columns: number;
  rows: number;
  cardWidth: number;
  cardHeight: number;
  popupWidth: number;
  popupHeight: number;
  gap: number;
  padding: number;
  borderRadius: number;
  scrollable: boolean;
  scaleFactor: number;
}

export interface RawPreviewSettings {
  trigger?: "Hover" | "Click" | "HoverAndClick";
  cardWidth?: number;
  cardHeight?: number;
  aspectRatioMode?: "SourceWindow" | "Fixed";
  aspectRatioNum?: number;
  aspectRatioDen?: number;
  autoGrid?: boolean;
  columns?: number;
  minColumns?: number;
  maxColumns?: number;
  maxRows?: number;
  maxPopupWidth?: number;
  maxPopupHeight?: number;
  gap?: number;
  padding?: number;
  borderRadius?: number;
  animated?: boolean;
  animationDuration?: number;
  showTitles?: boolean;
  titleLines?: "OneLine" | "TwoLines";
  compactTitles?: boolean;
  titleTooltip?: boolean;
  applicationAwareTitles?: boolean;
  ordering?:
    | "Manual"
    | "Stable"
    | "Mru"
    | "Alphabetical"
    | "ApplicationDefined"
    | "Hybrid";
  hoverOpenDelay?: number;
  hoverCloseDelay?: number;
  pointerGraceRegion?: number;
  keepOpenOnTraversal?: boolean;
  cacheEnabled?: boolean;
  cacheMemoryBudget?: number;
  staleWhileRefresh?: boolean;
  prewarmView?: boolean;
  nearestFirstProjection?: boolean;
  groupedClickAction?: "OpenPreview" | "ActivateLastUsed" | "MinimizeRestoreGroup";
}

const FALLBACK: Required<RawPreviewSettings> = {
  trigger: "Hover",
  cardWidth: 256,
  cardHeight: 0,
  aspectRatioMode: "SourceWindow",
  aspectRatioNum: 16,
  aspectRatioDen: 9,
  autoGrid: true,
  columns: 3,
  minColumns: 1,
  maxColumns: 8,
  maxRows: 4,
  maxPopupWidth: 800,
  maxPopupHeight: 560,
  gap: 10,
  padding: 10,
  borderRadius: 10,
  animated: true,
  animationDuration: 150,
  showTitles: true,
  titleLines: "OneLine",
  compactTitles: true,
  titleTooltip: true,
  applicationAwareTitles: true,
  ordering: "Manual",
  hoverOpenDelay: 0,
  hoverCloseDelay: 150,
  pointerGraceRegion: 24,
  keepOpenOnTraversal: true,
  cacheEnabled: true,
  cacheMemoryBudget: 32768,
  staleWhileRefresh: true,
  prewarmView: true,
  nearestFirstProjection: false,
  groupedClickAction: "OpenPreview",
};

/// Probe counter; not a `$state` because `previewSettings()` is read inside
/// `$derived` expressions and `state_unsafe_mutation` is triggered otherwise.
let _previewSettingsReads = 0;

export function previewSettings(): Required<RawPreviewSettings> {
  _previewSettingsReads += 1;
  const raw = (settingsState.value as any)?.preview as RawPreviewSettings | undefined;
  if (!raw) {
    return FALLBACK;
  }
  return { ...FALLBACK, ...raw };
}

export function previewSettingsReads(): number {
  return _previewSettingsReads;
}

// ------------------- layout engine -------------------

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function computePreviewLayout(
  count: number,
  monitor: { rect: { left: number; top: number; right: number; bottom: number }; scaleFactor: number },
): PreviewLayout {
  const s = previewSettings();
  const scale = monitor.scaleFactor || 1;

  const maxPopupW = Math.round(s.maxPopupWidth * scale);
  const maxPopupH = Math.round(s.maxPopupHeight * scale);
  const gap = Math.round(s.gap * scale);
  const padding = Math.round(s.padding * scale);
  const borderRadius = Math.round(s.borderRadius * scale);
  const titleStrip = s.showTitles ? Math.round((s.titleLines === "TwoLines" ? 2 : 1) * 18 * scale) : 0;

  const baseW = Math.round(s.cardWidth * scale);
  const ratio = s.aspectRatioMode === "Fixed" && s.aspectRatioDen > 0 ? s.aspectRatioNum / s.aspectRatioDen : 16 / 9;

  let cardWidth = Math.max(baseW, 64);
  let cardHeight = s.cardHeight > 0 ? Math.round(s.cardHeight * scale) : Math.round(cardWidth / ratio);

  const innerW = Math.max(maxPopupW - padding * 2, cardWidth);
  const innerH = Math.max(maxPopupH - padding * 2, cardHeight + titleStrip);

  let columns: number;
  if (!s.autoGrid || s.ordering !== "Manual") {
    columns = clamp(s.columns, s.minColumns, s.maxColumns);
  } else {
    columns = clamp(
      Math.floor((innerW + gap) / (cardWidth + gap)),
      s.minColumns,
      Math.min(s.maxColumns, Math.max(count, 1)),
    );
  }

  let rows = Math.ceil(count / columns);
  let scrollable = false;
  if (rows > s.maxRows) {
    scrollable = true;
    rows = s.maxRows;
  }

  while (rows * (cardHeight + titleStrip) + (rows - 1) * gap > innerH && cardHeight > 40) {
    cardHeight = Math.max(40, Math.round(cardHeight * 0.92));
  }

  const popupWidth = Math.min(
    maxPopupW,
    columns * cardWidth + (columns - 1) * gap + padding * 2,
  );
  const visibleRows = Math.max(1, Math.ceil(Math.min(count, 999) / columns));
  const contentH = visibleRows * (cardHeight + titleStrip) + Math.max(0, visibleRows - 1) * gap;
  const popupHeight = Math.min(
    maxPopupH,
    contentH + padding * 2 + (scrollable ? Math.round(8 * scale) : 0),
  );

  return {
    columns,
    rows,
    cardWidth,
    cardHeight,
    popupWidth,
    popupHeight,
    gap,
    padding,
    borderRadius,
    scrollable,
    scaleFactor: scale,
  };
}

// ------------------- ordering -------------------

const manualOrders = new Map<string, string[]>();

const SKIP_SEGMENTS = new Set([
  "visual studio code",
  "visual studio code - insiders",
  "microsoft visual studio code",
  "microsoft visual studio code - insiders",
  "insiders",
  "microsoft edge",
  "microsoft edge beta",
  "microsoft edge dev",
  "microsoft edge canary",
  "microsoft edge for business",
  "windows terminal",
  "terminal",
  "nushell",
  "command prompt",
  "powershell",
  "pwsh",
]);

const NUM_SEGMENT = /^\d+:?$/;
const PATH_SEGMENT = /^[a-zA-Z]:[\\/]/;

export function localIdentity(w: UserAppWindow): string {
  const title = w.title.trim();
  if (!title) {
    return w.hwnd.toString(16);
  }
  const segs = title.split(" - ").map((s) => s.trim()).filter(Boolean);
  for (const seg of segs) {
    if (SKIP_SEGMENTS.has(seg.toLowerCase())) continue;
    if (NUM_SEGMENT.test(seg)) continue;
    if (PATH_SEGMENT.test(seg)) continue;
    return seg;
  }
  return title;
}

export function appKeyOf(w: UserAppWindow): string {
  if (w.umid) return w.umid.toLowerCase();
  const exe = w.process?.path?.toLowerCase() ?? "";
  const stem = exe.split(/[\\/]/).pop() ?? w.appName?.toLowerCase() ?? "";
  const key = stem.replace(/\.exe$/, "");
  if (key.includes("insiders")) return "vscode-insiders";
  return key;
}

export function applyOrder(app: string, windows: UserAppWindow[]): UserAppWindow[] {
  const saved = manualOrders.get(app);
  if (!saved || saved.length === 0) {
    return windows;
  }
  const numbered = windows.map((w, i) => ({ w, pos: i }));
  for (const entry of numbered) {
    const local = localIdentity(entry.w).toLowerCase();
    const idx = saved.findIndex((id) => id.toLowerCase() === local);
    entry.pos = idx >= 0 ? idx : saved.length + entry.pos;
  }
  numbered.sort((a, b) => a.pos - b.pos);
  return numbered.map((n) => n.w);
}

export async function getOrder(app: string): Promise<string[]> {
  if (manualOrders.has(app)) {
    return manualOrders.get(app)!;
  }
  const order = await invoke(SeelenCommand.WegGetWindowOrder, { app });
  manualOrders.set(app, order);
  return order;
}

export async function setOrder(app: string, ids: string[]): Promise<void> {
  manualOrders.set(app, ids);
  await invoke(SeelenCommand.WegSetWindowOrder, { app, identities: ids });
}

// ------------------- hover timing (stage model) -------------------

interface StageModel {
  t0Date: number;
  init: number;
  metadata: number;
  layout: number;
  thumbnail: number;
}

let stages: StageModel | null = null;

/// t0Date comes from the dock webview `Date.now()` at pointer enter.
/// This module runs in the preview webview; cross-context deltas are based on
/// `performance.timeOrigin` (epoch ms) and intra-context on `performance.now()`.
export function startTiming(t0Date: number | undefined): void {
  if (!t0Date) {
    return;
  }
  stages = { t0Date, init: performance.now(), metadata: 0, layout: 0, thumbnail: 0 };
}

export function markMetadata(): void {
  if (stages && !stages.metadata) {
    stages.metadata = performance.now();
  }
}

export function markLayout(): void {
  if (stages && !stages.layout) {
    stages.layout = performance.now();
  }
}

export function markThumbnail(): void {
  if (stages && !stages.thumbnail) {
    stages.thumbnail = performance.now();
  }
}

function epochMs(pnow: number): number {
  return performance.timeOrigin + pnow;
}

export function reportFirstPaint(cacheHit: boolean): void {
  if (!stages) {
    return;
  }
  const t0 = stages.t0Date;
  const paint = performance.now();
  const initDate = epochMs(stages.init);
  const latency = {
    metadataReadyUs: Math.max(
      0,
      Math.round((initDate + (stages.metadata || stages.init) - initDate) * 1000 + (initDate - t0) * 1000),
    ),
    layoutReadyUs: Math.max(
      0,
      Math.round(
        ((stages.layout || stages.init) - stages.init) * 1000 + (initDate - t0) * 1000,
      ),
    ),
    thumbnailReadyUs: Math.max(
      0,
      Math.round(
        ((stages.thumbnail || stages.init) - stages.init) * 1000 + (initDate - t0) * 1000,
      ),
    ),
    firstPaintUs: Math.max(0, Math.round((epochMs(paint) - t0) * 1000)),
    cacheHit,
  };
  invoke(SeelenCommand.WegReportPreviewLatency, { latency }).catch(() => {});
  stages = null;
}
