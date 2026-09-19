import { invoke, SeelenCommand, SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";
import { SeelenWegSide, type UserAppWindow, type WindowEntry } from "@seelen-ui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import {
  markLayout,
  markMetadata,
  markThumbnail,
  previewSettings,
  reportFirstPaint,
  startTiming,
} from "../weg/state/preview.svelte.ts";

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

let hwnds = $state<number[]>([]);
let position = $state<SeelenWegSide>(SeelenWegSide.Bottom);
let previewSessionId = $state<string>("");
let t0Date = $state<number | undefined>(undefined);
let animated = $state(true);
let animationDuration = $state(150);

Widget.self.onTrigger(({ customArgs }) => {
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
  return _ordered.map((entry) => {
    const raw = rawByHwnd.get(entry.hwnd);
    const preview = previewOf(entry.hwnd);
    const stale = !!preview && !!preview.titleAtCapture && preview.titleAtCapture !== entry.title;
    return {
      entry,
      iconPath: raw?.relaunch?.icon || raw?.process?.path || null,
      umid: raw?.umid ?? null,
      preview,
      stale,
    };
  });
});

function titlesFor(entry: WindowEntry): { label: string; tooltip: string | null } {
  const s = previewSettings();
  const label = s.compactTitles ? localPart(entry) : entry.title;
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
