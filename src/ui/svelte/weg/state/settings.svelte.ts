import { invoke, RuntimeStyleSheet, SeelenCommand, Widget } from "@seelen-ui/lib";
import { Alignment, HideMode, SeelenWegSide } from "@seelen-ui/lib/types";
import { isTouchPrimary } from "libs/ui/svelte/utils";
import { computeWegRects, isHorizontalDockSide } from "./geometry.ts";
import { locale } from "../i18n/index.ts";
import { declareDocumentAsLayeredHitbox } from "libs/ui/react/utils/layered.ts";
import { systemState } from "./system.svelte.ts";
import { settings as _settings } from "./getters.svelte.ts";
import { dateState } from "libs/ui/svelte/runes/date.svelte.ts";

let isWidgetReady = $state(false);
const settings = $derived(_settings.value.byWidget["@seelen/weg"]);

const widgetRect = $derived(
  computeWegRects({
    monitor: systemState.currentMonitor.rect,
    scaleFactor: systemState.currentMonitor.scaleFactor,
    isTouch: isTouchPrimary.value,
    toolbar: {
      enabled: _settings.value.byWidget["@seelen/fancy-toolbar"]?.enabled ?? true,
      enabledOnMonitor: (_settings.value.monitorsV3[systemState.currentMonitor.id] as any)
        ?.byWidget?.["@seelen/fancy-toolbar"]?.enabled ?? true,
      position: _settings.value.byWidget["@seelen/fancy-toolbar"].position,
      itemSize: _settings.value.byWidget["@seelen/fancy-toolbar"].itemSize,
      padding: _settings.value.byWidget["@seelen/fancy-toolbar"].padding,
      margin: _settings.value.byWidget["@seelen/fancy-toolbar"].margin,
    },
    dock: {
      position: settings.position,
      size: settings.size,
      padding: settings.padding,
      margin: settings.margin,
    },
  }),
);

$effect.root(() => {
  $effect(() => {
    locale.set(_settings.value.language);
    dateState.setLang(_settings.value.language);
    dateState.setFormat(_settings.value.dateFormat);
  });
});

export const fullSettings = {
  get value() {
    return _settings.value;
  },
};

class SettingsState {
  popupAlignX = $derived.by(() => {
    switch (settings.position) {
      case SeelenWegSide.Left:
        return Alignment.Start;
      case SeelenWegSide.Right:
        return Alignment.End;
      default:
        return Alignment.Center;
    }
  });

  popupAlignY = $derived.by(() => {
    switch (settings.position) {
      case SeelenWegSide.Bottom:
        return Alignment.End;
      case SeelenWegSide.Top:
        return Alignment.Start;
      default:
        return Alignment.Center;
    }
  });

  tooltipOrigin = $derived.by(() => {
    const origin: { x: number | null; y: number | null } = { x: null, y: null };
    const rect = settingsState.widgetRect.hitboxRect;
    switch (settingsState.position) {
      case SeelenWegSide.Left:
        origin.x = rect.right;
        break;
      case SeelenWegSide.Right:
        origin.x = rect.left;
        break;
      case SeelenWegSide.Top:
        origin.y = rect.bottom;
        break;
      case SeelenWegSide.Bottom:
        origin.y = rect.top;
        break;
    }
    return origin;
  });

  get isReady() {
    return isWidgetReady;
  }

  set isReady(v: boolean) {
    isWidgetReady = v;
  }

  get all() {
    return _settings.value;
  }

  get allByWidget() {
    return _settings.value.byWidget;
  }

  get value() {
    return settings;
  }

  get position(): SeelenWegSide {
    return settings.position;
  }

  get hideMode(): HideMode {
    return settings.hideMode;
  }

  get delayToHide(): number {
    return settings.delayToHide;
  }

  get delayToShow(): number {
    return settings.delayToShow;
  }

  get widgetRect() {
    return widgetRect;
  }
}

export const settingsState = new SettingsState();

export function isHorizontalDock(): boolean {
  return isHorizontalDockSide(settings.position);
}

async function updateWidgetPosition() {
  const { hitboxRect, webviewRect } = widgetRect;
  const isTouch = isTouchPrimary.value;
  const hideMode = settings.hideMode;
  const position = settings.position;
  const isReady = settingsState.isReady;

  await Widget.self.setPosition(webviewRect);

  if (!isReady) {
    return;
  }

  if (hideMode === HideMode.Never || isTouch) {
    await invoke(SeelenCommand.RegisterAppBar, {
      rect: hitboxRect,
      edge: position as any,
    });
  } else {
    await invoke(SeelenCommand.UnregisterAppBar);
  }
}

Widget.self.attachPosition();
await updateWidgetPosition();

$effect.root(() => {
  $effect(() => {
    const { size, padding, margin, spaceBetweenItems } = settings;
    const sheet = new RuntimeStyleSheet("@config/weg");
    sheet.addVariable("--config-margin", `${margin}px`);
    sheet.addVariable("--config-padding", `${padding}px`);
    sheet.addVariable("--config-item-size", `${size}px`);
    sheet.addVariable("--config-space-between-items", `${spaceBetweenItems}px`);
    sheet.applyToDocument();
  });

  $effect(() => {
    updateWidgetPosition();
  });

  $effect(() => {
    if (isTouchPrimary.value) return;

    let unlisten: (() => void) | null = null;
    declareDocumentAsLayeredHitbox({
      getPhysicalRect: () => {
        const r = widgetRect.webviewRect;
        return {
          x: r.left,
          y: r.top,
          width: r.right - r.left,
          height: r.bottom - r.top,
        };
      },
    }).then((unlistenFn) => {
      unlisten = unlistenFn;
    });

    return () => {
      unlisten?.();
    };
  });
});
