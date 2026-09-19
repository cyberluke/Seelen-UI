import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { SeelenWegSide, type UserAppWindow } from "@seelen-ui/lib/types";
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
  side?: SeelenWegSide,
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

  // Orientation-first: the dock side is the layout constraint of record.
  // Left/Right docks have a vertical primary axis -> single column, grow rows,
  // scroll vertically. Top/Bottom docks keep the adaptive width-driven grid.
  const vertical = side === SeelenWegSide.Left || side === SeelenWegSide.Right;

  let columns: number;
  if (vertical) {
    columns = 1;
  } else if (!s.autoGrid || s.ordering !== "Manual") {
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

// ------------------- group identity (mirrors the native identity core) -------------------

/// Mirrors `identity.rs` application-key normalization so all surfaces use
/// ONE semantic identity model. Keep in sync with
/// `src/background/modules/weg_core/identity.rs`.
export function appKeyOf(w: UserAppWindow): string {
  const umid = w.umid?.toLowerCase().replace(/ /g, ".") ?? "";
  const stem = (w.process?.path?.toLowerCase().split(/[\\/]/).pop() ?? "")
    .replace(/\.exe$/, "");
  const path = w.process?.path?.toLowerCase() ?? "";
  const app = w.appName?.toLowerCase() ?? "";

  if (stem.startsWith("msedge") || stem === "edge" || umid.startsWith("microsoft.edge")) {
    if (stem.endsWith("_beta") || umid.endsWith(".beta") || path.includes("edge beta")) {
      return "edge-beta";
    }
    if (stem.endsWith("_dev") || umid.endsWith(".dev") || path.includes("edge dev")) {
      return "edge-dev";
    }
    if (stem.endsWith("_canary") || umid.endsWith(".canary") || path.includes("edge canary")) {
      return "edge-canary";
    }
    return "edge-stable";
  }
  if (stem === "code" || stem === "code - insiders" || umid.startsWith("microsoft.visualstudiocode")) {
    const insiders = stem === "code - insiders" || umid.includes(".insiders") || path.includes("insiders");
    return insiders ? "vscode-insiders" : "code";
  }
  if (stem === "wt" || umid.startsWith("microsoft.windowsterminal")) {
    if (umid.endsWith(".canary") || path.includes("canary")) return "wt-canary";
    return "wt";
  }
  return umid || stem || app;
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
