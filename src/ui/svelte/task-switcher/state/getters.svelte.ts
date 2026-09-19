import { invoke, SeelenCommand, SeelenEvent, Settings, subscribe, Widget } from "@seelen-ui/lib";
import type { WindowEntry } from "@seelen-ui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";

export const widget = Widget.getCurrent();

export const settings = lazyRune(() => Settings.getAsync());
await Settings.onChange((s) => (settings.value = s));

// Raw window metadata. `lastForegroundAt` is treated as pure metadata here;
// the spatial order stays stable in the semantic core below.
export const windows = lazyRune(() => invoke(SeelenCommand.GetUserAppWindows));
subscribe(SeelenEvent.UserAppWindowsChanged, windows.setByPayload);

// The ONE semantic ordering/identity source shared by grouped preview,
// Task Switcher and automation APIs (native PersistentDesktopOrder).
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

export const previews = lazyRune(() => invoke(SeelenCommand.GetUserAppWindowsPreviews));
subscribe(SeelenEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

export const focusedWinId = lazyRune(async () => (await invoke(SeelenCommand.GetFocusedApp)).hwnd);
subscribe(SeelenEvent.GlobalFocusChanged, (e) => {
  focusedWinId.value = e.payload.hwnd;
});

export const monitors = lazyRune(() => invoke(SeelenCommand.SystemGetMonitors));
subscribe(SeelenEvent.SystemMonitorsChanged, monitors.setByPayload);

await Promise.all([
  settings.init(),
  windows.init(),
  entries.init(),
  previews.init(),
  focusedWinId.init(),
  monitors.init(),
]);
