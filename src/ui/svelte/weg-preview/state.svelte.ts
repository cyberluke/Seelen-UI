import { invoke, SeelenCommand, SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";
import { SeelenWegSide, type UserAppWindow } from "@seelen-ui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import {
  appKeyOf,
  applyOrder,
  getOrder,
  localIdentity,
  markLayout,
  markMetadata,
  markThumbnail,
  previewSettings,
  reportFirstPaint,
  setOrder,
  startTiming,
} from "../weg/state/preview.svelte.ts";

// Layer A: window metadata (shared, kept hot via events - no polling).
export const interactables = lazyRune<UserAppWindow[]>(() => invoke(SeelenCommand.GetUserAppWindows));
subscribe(SeelenEvent.UserAppWindowsChanged, interactables.setByPayload);

// Layer B: thumbnail cache (kept hot via capture events).
export const previews = lazyRune<Record<number, { data: string; hash: string }>>(
  () => invoke(SeelenCommand.GetUserAppWindowsPreviews),
);
subscribe(SeelenEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

await Promise.all([interactables.init(), previews.init()]);

let hwnds = $state<number[]>([]);
let position = $state<SeelenWegSide>(SeelenWegSide.Bottom);
let t0Date = $state<number | undefined>(undefined);
let animated = $state(true);
let animationDuration = $state(150);
let orderVersion = $state(0);

Widget.self.onTrigger(({ customArgs }) => {
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

const _filtered = $derived(interactables.value.filter((w) => hwnds.includes(w.hwnd)));

const _appKeys = $derived.by(() => {
  const keys = new Set<string>();
  for (const w of _filtered) {
    keys.add(appKeyOf(w));
  }
  return [...keys];
});

// load persisted manual order per app (event-driven, one fetch per app)
$effect.root(() => {
  $effect(() => {
    for (const key of _appKeys) {
      getOrder(key)
        .then(() => orderVersion++)
        .catch(() => {});
    }
  });
});

/// ordered + nearest-pointer projection (visual only)
const _ordered = $derived.by(() => {
  void orderVersion;
  const s = previewSettings();
  const results: UserAppWindow[] = [];
  for (const key of _appKeys) {
    const group = _filtered.filter((w) => appKeyOf(w) === key);
    let list = applyOrder(key, group);
    if (s.nearestFirstProjection && list.length > 1) {
      // visual-only projection: most recently active first, others keep relative order
      list = [...list].sort((a, b) => b.lastForegroundAt - a.lastForegroundAt);
    }
    results.push(...list);
  }
  return results;
});

function titlesFor(w: UserAppWindow): { label: string; tooltip: string | null } {
  const s = previewSettings();
  const label = s.compactTitles ? localIdentity(w) : w.title;
  return {
    label,
    tooltip: s.showTitles && s.titleTooltip && label !== w.title ? w.title : null,
  };
}

let firstPaintReported = 0;

class PreviewState {
  get currentInteractables() {
    return _ordered;
  }

  get position() {
    return position;
  }

  get animated() {
    return animated;
  }

  get animationDuration() {
    return animationDuration;
  }

  titleInfo(w: UserAppWindow) {
    return titlesFor(w);
  }

  thumbnailOf(hwnd: number) {
    const preview = previews.value[hwnd];
    if (preview) {
      markThumbnail();
    }
    return preview;
  }

  reportPaint(cacheHit: boolean) {
    const stamp = Date.now();
    if (firstPaintReported === stamp) return;
    firstPaintReported = stamp;
    reportFirstPaint(cacheHit);
  }

  async persistOrder(list: UserAppWindow[]): Promise<void> {
    if (!list.length) {
      return;
    }
    const app = appKeyOf(list[0]!);
    const ids = list.map((w) => localIdentity(w));
    await setOrder(app, ids);
  }

  reorder(from: number, to: number): UserAppWindow[] {
    const list = [..._ordered];
    const [moved] = list.splice(from, 1);
    if (moved !== undefined) {
      list.splice(to, 0, moved);
    }
    return list;
  }
}

export const previewState = new PreviewState();
