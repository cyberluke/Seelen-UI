import { invoke, RuntimeStyleSheet, SeelenCommand, SeelenEvent, Settings, subscribe, Widget } from "@seelen-ui/lib";
import { declareDocumentAsLayeredHitbox } from "libs/ui/react/utils/layered";
import type { FocusedApp, TwmReservation, TwmRuntimeTree, WindowManagerSettings } from "@seelen-ui/lib/types";

import { computeWmCanvasRect } from "./state/geometry.ts";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";
import { isTouchPrimary } from "libs/ui/svelte/utils/signals.svelte.ts";

let layouts = lazyRune(() => invoke(SeelenCommand.WmGetRenderTree));
subscribe(SeelenEvent.WMTreeChanged, layouts.setByPayload);

let workspaces = lazyRune(() => invoke(SeelenCommand.StateGetVirtualDesktops));
subscribe(SeelenEvent.VirtualDesktopsChanged, workspaces.setByPayload);

let interactables = lazyRune(() => invoke(SeelenCommand.GetUserAppWindows));
subscribe(SeelenEvent.UserAppWindowsChanged, interactables.setByPayload);

let monitors = lazyRune(() => invoke(SeelenCommand.SystemGetMonitors));
subscribe(SeelenEvent.SystemMonitorsChanged, monitors.setByPayload);

let reservation = $state<TwmReservation | null>(null);
subscribe(SeelenEvent.WMSetReservation, (e) => {
  reservation = e.payload;
});

let forceRepositioning = $state(0);
subscribe(SeelenEvent.WMForceRetiling, () => {
  forceRepositioning++;
});

const [focusedAppInit, settingsInit] = await Promise.all([
  invoke(SeelenCommand.GetFocusedApp),
  Settings.getAsync(),
  layouts.init(),
  workspaces.init(),
  interactables.init(),
  monitors.init(),
]);

let focusedApp = $state<FocusedApp>(focusedAppInit);
subscribe(SeelenEvent.GlobalFocusChanged, (e) => {
  focusedApp = e.payload;
});

let fullSettings = $state(settingsInit);
let settings = $state<WindowManagerSettings>(
  settingsInit.byWidget["@seelen/window-manager"],
);
Settings.onChange((s) => {
  fullSettings = s;
  settings = s.byWidget["@seelen/window-manager"];
});

// =================================================
//                  CSS variables
// =================================================

$effect.root(() => {
  $effect(() => {
    const sheet = new RuntimeStyleSheet("@config/window-manager");
    sheet.addVariable("--config-padding", `${settings.workspacePadding}px`);
    sheet.addVariable("--config-containers-gap", `${settings.workspaceGap}px`);
    sheet.addVariable(
      "--config-margin-top",
      `${settings.workspaceMargin.top}px`,
    );
    sheet.addVariable(
      "--config-margin-left",
      `${settings.workspaceMargin.left}px`,
    );
    sheet.addVariable(
      "--config-margin-right",
      `${settings.workspaceMargin.right}px`,
    );
    sheet.addVariable(
      "--config-margin-bottom",
      `${settings.workspaceMargin.bottom}px`,
    );
    sheet.addVariable("--config-border-offset", `${settings.border.offset}px`);
    sheet.addVariable("--config-border-width", `${settings.border.width}px`);
    sheet.applyToDocument();
  });
});

// =================================================
//                   Positioning
// =================================================

const monitorId = Widget.getCurrent().decoded.monitorId!;

const widgetRect = $derived.by(() => {
  const monitor = monitors.value.find((m) => m.id === monitorId);
  if (!monitor) {
    throw new Error("Current monitor not found");
  }
  const tb = fullSettings.byWidget["@seelen/fancy-toolbar"];
  const weg = fullSettings.byWidget["@seelen/weg"];
  return computeWmCanvasRect({
    monitor: monitor.rect,
    scaleFactor: monitor.scaleFactor,
    isTouch: isTouchPrimary.value,
    toolbar: {
      enabled: tb.enabled,
      enabledOnMonitor: (fullSettings.monitorsV3[monitor.id] as any)?.byWidget?.[
        "@seelen/fancy-toolbar"
      ]?.enabled ?? true,
      hideMode: tb.hideMode,
      position: tb.position,
      itemSize: tb.itemSize,
      padding: tb.padding,
      margin: tb.margin,
    },
    weg: {
      enabled: weg.enabled,
      enabledOnMonitor: (fullSettings.monitorsV3[monitor.id] as any)?.byWidget?.["@seelen/weg"]
        ?.enabled ?? true,
      hideMode: weg.hideMode,
      position: weg.position,
      size: weg.size,
      padding: weg.padding,
      margin: weg.margin,
    },
  });
});

$effect.root(() => {
  Widget.self.attachPosition();
  $effect(() => {
    Widget.self.setPosition(widgetRect);
  });
});

await declareDocumentAsLayeredHitbox({
  getPhysicalRect: () => {
    const r = widgetRect;
    return {
      x: r.left,
      y: r.top,
      width: r.right - r.left,
      height: r.bottom - r.top,
    };
  },
  shouldAllowMouseEvent: (e) => e.getAttribute("data-allow-mouse-events") === "true",
});

// =================================================
//               Exported State Getters
// =================================================

export type State = _State;
class _State {
  readonly paused = $derived.by(
    () => layouts.value.paused || !!layouts.value.pausedByMonitor[monitorId],
  );

  getLayout(monitorId: string): TwmRuntimeTree | null {
    const activeWsId = workspaces.value?.monitors?.[monitorId]?.active_workspace;
    if (!activeWsId) return null;
    return layouts.value?.workspaces?.[activeWsId] ?? null;
  }
  get forceRepositioning() {
    return forceRepositioning;
  }
  get interactables() {
    return interactables.value;
  }
  get focusedApp() {
    return focusedApp;
  }
  get settings() {
    return settings;
  }
  get reservation() {
    return reservation;
  }
  get widgetRect() {
    return widgetRect;
  }
}

export const state = new _State();
