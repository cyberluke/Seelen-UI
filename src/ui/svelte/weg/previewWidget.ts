import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { Alignment, SeelenWegSide, type UserAppWindow, type WidgetId } from "@seelen-ui/lib/types";
import { settingsState } from "./state/settings.svelte.ts";
import { systemState } from "./state/system.svelte.ts";
import { previewSettings } from "./state/preview.svelte.ts";

export function triggerPreviewWidget(itemEl: HTMLElement, windows: UserAppWindow[], t0Date?: number) {
  const dockSide = settingsState.position;
  const preview = previewSettings();

  const elRect = itemEl.getBoundingClientRect();
  const viewRect = settingsState.widgetRect.hitboxRect;
  const scaleFactor = systemState.currentMonitor.scaleFactor || globalThis.devicePixelRatio || 1;

  const toPhysical = (n: number) => Math.round(n * scaleFactor);

  let x: number;
  let y: number;
  let alignX: Alignment;
  let alignY: Alignment;

  switch (dockSide) {
    case SeelenWegSide.Bottom:
      x = viewRect.left + toPhysical(elRect.left + elRect.width / 2);
      y = viewRect.top;
      alignX = Alignment.Center;
      alignY = Alignment.End;
      break;
    case SeelenWegSide.Top:
      x = viewRect.left + toPhysical(elRect.left + elRect.width / 2);
      y = viewRect.bottom;
      alignX = Alignment.Center;
      alignY = Alignment.Start;
      break;
    case SeelenWegSide.Left:
      x = viewRect.right;
      y = viewRect.top + toPhysical(elRect.top + elRect.height / 2);
      alignX = Alignment.Start;
      alignY = Alignment.Center;
      break;
    default:
      x = viewRect.left;
      y = viewRect.top + toPhysical(elRect.top + elRect.height / 2);
      alignX = Alignment.End;
      alignY = Alignment.Center;
      break;
  }

  invoke(SeelenCommand.TriggerWidget, {
    payload: {
      id: "@seelen/weg-preview" as WidgetId,
      desiredPosition: { x, y },
      alignX,
      alignY,
      customArgs: {
        hwnds: windows.map((w) => w.hwnd),
        position: dockSide,
        t0Date,
        animated: preview.animated,
        animationDuration: preview.animationDuration,
      },
    },
  });
}
