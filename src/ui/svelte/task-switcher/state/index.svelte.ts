import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { WindowEntry } from "@seelen-ui/lib/types";
import { debounce } from "lodash";
import z from "zod";
import { entries, focusedWinId, monitors, previews, settings, widget, windows } from "./getters.svelte.ts";

export { entries, focusedWinId, monitors, previews, settings, widget, windows };

const WidgetConfigSchema = z.object({
  onlyOnActiveMonitor: z.boolean(),
  ordering: z.enum(["SemanticPersistent", "MRU", "Alphabetical"]).default("SemanticPersistent"),
});

const widgetConfig = $derived.by(
  () =>
    WidgetConfigSchema.safeParse(settings.value.getCurrentWidgetConfig()).data ??
      (widget.getDefaultConfig() as unknown as z.infer<typeof WidgetConfigSchema>),
);

// +++++++++++++++++++++++ Reactive State +++++++++++++++++++++++

let showing = $state(false);
let autoConfirm = $state(false);

let desiredPosition = $state<{ x: number; y: number } | null>(null);

// Frozen session snapshot: the runtime ids in persistent spatial order, fixed
// when the switcher opens. Focus changes only patch/move selection, they never
// reshuffle this list while the session is open.
let sessionOrder = $state<string[]>([]);

let selectedWindow = $state<number | null>(focusedWinId.value ?? null);

function byRuntimeId(): Map<string, WindowEntry> {
  return new Map(entries.value.map((e) => [e.runtimeWindowId, e] as const));
}

// `lastForegroundAt` is metadata; default order is the persistent semantic
// order produced by the native core. MRU / Alphabetical only apply when the
// user explicitly chooses them in the widget settings.
let orderedWindows = $derived.by((): WindowEntry[] => {
  switch (taskSwitcherValue()) {
    case "MRU":
      return [...entries.value].sort((a, b) => b.lastForegroundAt - a.lastForegroundAt);
    case "Alphabetical":
      return [...entries.value].sort((a, b) => (a.alias || a.displayTitle).localeCompare(b.alias || b.displayTitle));
    default:
      return entries.value;
  }
});

let baseWindows = $derived.by((): WindowEntry[] => applyMonitorFilter(orderedWindows));

// Session-frozen display order, or the persistent order when hidden.
// New windows append; metadata patches in place; destroyed ids drop out.
let filteredWindows = $derived.by((): WindowEntry[] => {
  if (!showing) {
    return baseWindows;
  }

  // session mode: walk the frozen list, patch metadata in place,
  // remove destroyed ids, append newly created windows without reshuffling.
  const byId = byRuntimeId();
  const frozen: WindowEntry[] = [];
  const seen = new Set<string>();
  for (const id of sessionOrder) {
    const entry = byId.get(id);
    if (entry) {
      frozen.push(entry);
      seen.add(id);
    }
  }
  for (const entry of orderedWindows) {
    if (!seen.has(entry.runtimeWindowId)) {
      frozen.push(entry);
    }
  }
  return applyMonitorFilter(frozen);
});

// Persist newly created windows into the frozen session order (append-only).
$effect.root(() => {
  $effect(() => {
    if (!showing) return;
    const ids = new Set(sessionOrder);
    const additions = orderedWindows
      .filter((e) => !ids.has(e.runtimeWindowId))
      .map((e) => e.runtimeWindowId);
    if (additions.length > 0) {
      sessionOrder = [...sessionOrder, ...additions];
    }
  });
});

function applyMonitorFilter(list: WindowEntry[]): WindowEntry[] {
  const monitor = activeMonitor;
  if (!widgetConfig.onlyOnActiveMonitor || !monitor) {
    return list;
  }
  return list.filter((w) => String(w.monitor) === String(monitor.id));
}

function taskSwitcherValue(): "SemanticPersistent" | "MRU" | "Alphabetical" {
  const value = widgetConfig.ordering;
  return (value as "SemanticPersistent" | "MRU" | "Alphabetical") ?? "SemanticPersistent";
}

// Sync selectedWindow with focused window when the switcher is not visible
$effect.root(() => {
  $effect(() => {
    if (!showing) {
      const win = filteredWindows.find((w) => w.hwnd === focusedWinId.value);
      selectedWindow = win?.hwnd ?? null;
    }
  });
});

let desktopRect = $derived.by(() => {
  let rect = { top: 0, left: 0, right: 0, bottom: 0 };
  for (const monitor of monitors.value) {
    rect.left = Math.min(rect.left, monitor.rect.left);
    rect.top = Math.min(rect.top, monitor.rect.top);
    rect.right = Math.max(rect.right, monitor.rect.right);
    rect.bottom = Math.max(rect.bottom, monitor.rect.bottom);
  }
  return rect;
});

// Monitor under the cursor position that triggered the switcher, falling back to primary
let activeMonitor = $derived.by(() => {
  const pos = desiredPosition;
  const found = pos &&
    monitors.value.find(
      (m) =>
        m.rect.left <= pos.x &&
        pos.x < m.rect.right &&
        m.rect.top <= pos.y &&
        pos.y < m.rect.bottom,
    );
  return found || monitors.value.find((m) => m.isPrimary) || monitors.value[0];
});

let relativeActiveMonitor = $derived.by(() => {
  const monitor = activeMonitor;
  if (!monitor) {
    return null;
  }
  return {
    ...monitor,
    rect: {
      top: monitor.rect.top - desktopRect.top,
      left: monitor.rect.left - desktopRect.left,
      right: monitor.rect.right - desktopRect.left,
      bottom: monitor.rect.bottom - desktopRect.top,
    },
  };
});

$effect.root(() => {
  widget.attachPosition();
  $effect(() => {
    widget.setPosition(desktopRect);
  });
});

// +++++++++++++++++++++++ State Class +++++++++++++++++++++++

class State {
  get showing() {
    return showing;
  }

  set showing(value: boolean) {
    showing = value;
    if (!value) {
      // leave the session: clear the frozen snapshot
      sessionOrder = [];
    }
  }

  get windows() {
    return filteredWindows;
  }

  get previews() {
    return previews.value;
  }

  get selectedWindow() {
    return selectedWindow;
  }

  set selectedWindow(value: number | null) {
    selectedWindow = value;
  }

  get activeMonitor() {
    return relativeActiveMonitor;
  }
}

export const globalState = new State();

// +++++++++++++++++++++++ Visibility +++++++++++++++++++++++

$effect.root(() => {
  $effect(() => {
    let cancelled = false;

    if (showing) {
      widget.show().then(async () => {
        if (!cancelled) {
          await widget.focus();
        }
      });
    } else {
      widget.hide();
    }

    return () => {
      cancelled = true;
    };
  });

  const hideIfNotFocused = debounce(() => {
    if (focusedWinId.value !== widget.windowId) {
      showing = false;
    }
  }, 100);
  // Hide when focus leaves the widget
  $effect(() => {
    focusedWinId.value; // subscribed
    hideIfNotFocused(); // debounced to avoid inmediate hidden if focused changed while opening the widget
  });

  // Poll the hardware Alt key state instead of relying on keyup events,
  // since the widget-focus trick fakes an Alt keydown that never reaches window.onkeyup.
  $effect(() => {
    if (!showing) {
      return;
    }

    let cancelled = false;
    let wasAltDown = true;

    const poll = async () => {
      while (!cancelled) {
        const isAltDown = await invoke(SeelenCommand.GetKeyState, { key: "Alt" });
        if (wasAltDown && !isAltDown) {
          onAltKeyUp();
        }
        wasAltDown = isAltDown;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
    };
    poll();

    return () => {
      cancelled = true;
    };
  });
});

// +++++++++++++++++++++++ Triggering +++++++++++++++++++++++

function onAltKeyUp() {
  if (showing && selectedWindow && autoConfirm) {
    showing = false;
    focusByHwnd(selectedWindow);
  }
}

function focusByHwnd(hwnd: number): void {
  invoke(SeelenCommand.WegFocusWindow, { identification: hwnd.toString(16) });
}

widget.onTrigger((payload) => {
  const direction: string = (payload.customArgs?.direction as string) || "next";
  const autoConfirmValue: boolean = (payload.customArgs?.autoConfirm as boolean) || false;

  // Only capture autoConfirm and the trigger monitor on the first trigger (when switcher was hidden),
  // and do it before filtering windows so the monitor filter reflects the new cursor position.
  if (!showing) {
    autoConfirm = autoConfirmValue;
    if (payload.desiredPosition) {
      desiredPosition = payload.desiredPosition;
    }
    // First press of a new session: freeze the persistent semantic order.
    sessionOrder = filteredWindows.map((w) => w.runtimeWindowId);
  }

  const targetWindows = filteredWindows;
  if (targetWindows.length === 0) {
    return;
  }

  // Use the currently selected window when already showing, otherwise start from focused
  const currentHwnd = showing ? selectedWindow : focusedWinId.value;

  let index = targetWindows.findIndex((w) => w.hwnd === currentHwnd);
  if (direction === "next") {
    if (index === -1) index = targetWindows.length - 1;
    selectedWindow = targetWindows[(index + 1) % targetWindows.length]?.hwnd ?? null;
  } else if (direction === "previous") {
    if (index === -1) index = 0;
    selectedWindow = targetWindows[(index - 1 + targetWindows.length) % targetWindows.length]?.hwnd ?? null;
  }

  console.debug(
    "[task-switcher]",
    JSON.stringify({
      event: showing ? "navigate" : "open",
      orderingStrategy: taskSwitcherValue(),
      sessionId: sessionOrder.length ? sessionOrder.join(",") : null,
      ordered: targetWindows.map((w) => w.logicalIdentity),
      selected: targetWindows.find((w) => w.hwnd === selectedWindow)?.logicalIdentity ?? null,
    }),
  );

  showing = true;
});

window.onkeydown = (e) => {
  if (e.key === "Escape") {
    showing = false;
  }
};
