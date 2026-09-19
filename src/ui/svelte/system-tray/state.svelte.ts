import { invoke, SeelenCommand, SeelenEvent, Settings, subscribe } from "@seelen-ui/lib";
import { TrayOfflineMode } from "@seelen-ui/lib/types";
import type { FancyToolbarSettings, SysTrayIcon, SystrayIconAction, TrayPinState } from "@seelen-ui/lib/types";

export interface ResolvedTrayIcon {
  raw: SysTrayIcon;
  logicalId: string;
  pinnedOrder: number;
  tooltip: string;
}

interface TrayStore {
  icons: SysTrayIcon[];
  pinned: string[];
  settings: FancyToolbarSettings | null;
  ready: boolean;
}

const store = $state<TrayStore>({
  icons: [],
  pinned: [],
  settings: null,
  ready: false,
});

function applySettings(s: FancyToolbarSettings | undefined | null): void {
  store.settings = s ?? null;
  store.pinned = s?.tray?.pinned ? [...s.tray.pinned] : [];
}

async function seed(): Promise<void> {
  const [icons, settings] = await Promise.all([
    invoke(SeelenCommand.GetSystemTrayIcons),
    Settings.getAsync(),
  ]);
  store.icons = icons;
  applySettings(settings.byWidget["@seelen/fancy-toolbar"]);
  store.ready = true;
}

await seed();

subscribe(SeelenEvent.SystemTrayChanged, (e) => {
  store.icons = e.payload;
});
Settings.onChange((s) => applySettings(s.byWidget?.["@seelen/fancy-toolbar"]));

// ── Derived helpers ─────────────────────────────────────────────────────────

function _resolveOne(icon: SysTrayIcon, order: number): ResolvedTrayIcon {
  return {
    raw: icon,
    logicalId: icon.logicalId,
    pinnedOrder: order,
    tooltip: icon.tooltip?.trim() ||
      icon.applicationDisplayName ||
      icon.processName ||
      "",
  };
}

const _resolvedPinned = $derived.by((): ResolvedTrayIcon[] => {
  const byLogical = new Map<string, SysTrayIcon>();
  for (const icon of store.icons) {
    if (icon.isVisible) byLogical.set(icon.logicalId, icon);
  }
  const offlineMode = store.settings?.tray?.offlinePinned;
  const result: ResolvedTrayIcon[] = [];
  for (let i = 0; i < store.pinned.length; i++) {
    const key = store.pinned[i]!;
    const hit = byLogical.get(key);
    if (hit) {
      result.push(_resolveOne(hit, i));
      continue;
    }
    if (offlineMode === TrayOfflineMode.Disabled) {
      const now = Date.now();
      result.push({
        raw: {
          stableId: { HandleUid: [0, now] },
          logicalId: key,
          uid: null,
          windowHandle: null,
          guid: null,
          tooltip: key,
          iconHandle: null,
          iconPath: null,
          iconImageHash: null,
          callbackMessage: null,
          version: null,
          isVisible: false,
          processId: null,
          processPath: null,
          processName: null,
          appUserModelId: null,
          applicationDisplayName: null,
        } as unknown as SysTrayIcon,
        logicalId: key,
        pinnedOrder: i,
        tooltip: key,
      });
    }
  }
  return result;
});

const _overflowIcons = $derived.by((): SysTrayIcon[] => store.icons.filter((i) => i.isVisible));

const _pinnedSet = $derived.by(() => new Set(store.pinned));

/// Function-return exports are stable for module-level use with Svelte runes.
export function resolvedPinned(): ResolvedTrayIcon[] {
  return _resolvedPinned;
}

export function overflowIcons(): SysTrayIcon[] {
  return _overflowIcons;
}

export function pinnedSet(): Set<string> {
  return _pinnedSet;
}

// ── Actions ────────────────────────────────────────────────────────────────

export async function togglePin(logicalId: string): Promise<void> {
  if (!logicalId) return;
  if (store.pinned.includes(logicalId)) {
    await invoke(SeelenCommand.UnpinTrayIcon, { logicalId });
  } else {
    await invoke(SeelenCommand.PinTrayIcon, { logicalId });
  }
}

export async function setPinnedOrder(order: string[]): Promise<void> {
  await invoke(SeelenCommand.SetTrayPinOrder, { order });
}

export async function sendAction(
  logicalId: string,
  action: SystrayIconAction,
): Promise<void> {
  await invoke(SeelenCommand.SendTrayAction, { logicalId, action });
}

export { store as trayStore };
export type { TrayPinState };
