import { invoke, SeelenCommand, SeelenEvent, subscribe, Widget } from "@seelen-ui/lib";
import type { Alignment, UserAppWindow, WidgetId } from "@seelen-ui/lib/types";
import { selfWinId, widgetStatuses } from "./state/getters.svelte.ts";
import { settingsState } from "./state/settings.svelte.ts";
import { systemState } from "./state/system.svelte.ts";
import { appKeyOf, previewSettings } from "./state/preview.svelte.ts";
import { computePreviewAnchor, isLargePreviewCard } from "./state/geometry.ts";

// ── Cross-webview preview lifecycle controller ──────────────────────────────
// ONE semantic model for the grouped preview: a small state machine with a
// session token per open. All events are matched against the active session id
// so stale enter/leave/close events from older sessions are ignored. The main
// `@seelen/weg` webview never hides itself for preview lifecycle; only the
// dedicated `WegHidePreview` command touches the preview webview.

type PreviewPhase = "Closed" | "Opening" | "Open" | "Closing";

interface PreviewSession {
  id: string;
  groupKey: string;
  signature: string;
  /** owner windows of this preview; they keep the popup alive on focus */
  hwnds: number[];
}

const session: {
  current: PreviewSession | null;
  phase: PreviewPhase;
  hideTimer: ReturnType<typeof setTimeout> | null;
  pointerInsidePreview: boolean;
  lastAnchorKey: string;
  graceCheck: (() => boolean) | null;
  onExpire: (() => void) | null;
} = {
  current: null,
  phase: "Closed",
  hideTimer: null,
  pointerInsidePreview: false,
  lastAnchorKey: "",
  graceCheck: null,
  onExpire: null,
};

function debug(event: string, extra?: unknown): void {
  console.debug(
    `[weg-preview] ${event}`,
    JSON.stringify({
      sessionId: session.current?.id ?? null,
      groupKey: session.current?.groupKey ?? null,
      windowSignature: session.current?.signature ?? null,
      phase: session.phase,
      mainWegVisible: true,
      ...((extra as object) || {}),
    }),
  );
}

function signatureOf(windows: UserAppWindow[]): string {
  return windows
    .map((w) => w.hwnd)
    .sort((a, b) => a - b)
    .join(",");
}

// ── events from the preview webview (matched by session id) ─────────────────

let listenersReady = false;
function ensurePreviewListeners(): void {
  if (listenersReady) return;
  listenersReady = true;

  Widget.self.webview
    .listen<{ sessionId: string }>("weg-preview:pointer-enter", ({ payload }) => {
      if (!session.current || payload.sessionId !== session.current.id) return; // stale
      session.pointerInsidePreview = true;
      cancelHide();
      debug("preview-enter", { previewVisible: true });
    })
    .catch(() => {});

  Widget.self.webview
    .listen<{ sessionId: string }>("weg-preview:pointer-leave", ({ payload }) => {
      if (!session.current || payload.sessionId !== session.current.id) return; // stale
      session.pointerInsidePreview = false;
      debug("preview-leave", { previewVisible: true });
      armHide();
    })
    .catch(() => {});

  Widget.self.webview
    .listen<{ sessionId: string }>("weg-preview:closed", ({ payload }) => {
      if (!session.current || payload.sessionId !== session.current.id) return; // stale
      session.phase = "Closed";
      session.pointerInsidePreview = false;
      debug("preview-hidden");
    })
    .catch(() => {});

  // Focus-driven dismissal: pointer lifecycle alone is insufficient, a click
  // outside the webviews never produces `pointerleave` here, so the popup
  // would stay forever. Any global focus change that belongs to neither the
  // dock, the preview nor one of the owner windows dismisses the preview.
  subscribe(SeelenEvent.GlobalFocusChanged, ({ payload }) => {
    if (session.phase === "Closed" || !session.current) return;
    const hwnd = payload?.hwnd;
    if (hwnd === undefined) return;
    const previewWindowId = widgetStatuses.value.find(
      (status) => status.widgetId === "@seelen/weg-preview",
    )?.webviewWindowId;
    const ownedBySession = session.current.hwnds.includes(hwnd);
    if (hwnd === selfWinId.value || hwnd === previewWindowId || ownedBySession) {
      return;
    }
    cancelHide();
    hidePreview();
    debug("preview-dismissed-by-focus", { focused: hwnd });
  });
}

// ── hide timer ───────────────────────────────────────────────────────────────

function cancelHide(): void {
  if (session.hideTimer !== null) {
    clearTimeout(session.hideTimer);
    session.hideTimer = null;
    debug("close-cancelled");
  }
}

function armHide(): void {
  cancelHide();
  const { hoverCloseDelay } = previewSettings();
  session.phase = "Closing";
  session.hideTimer = setTimeout(() => {
    session.hideTimer = null;
    const { keepOpenOnTraversal } = previewSettings();
    const inside = session.pointerInsidePreview || (session.graceCheck ? session.graceCheck() : false);
    if (keepOpenOnTraversal && inside) {
      session.phase = "Open";
      return;
    }
    hidePreview();
  }, hoverCloseDelay);
  debug("close-armed");
}

function hidePreview(): void {
  session.phase = "Closed";
  // Targeted command: hides ONLY `@seelen/weg-preview`, keeps it warm.
  // `@seelen/weg` itself is never hidden by the preview lifecycle.
  invoke(SeelenCommand.WegHidePreview).catch(() => {});
  debug("preview-hidden");
}

// ── public API ───────────────────────────────────────────────────────────────

export interface PreviewAnchor {
  x: number;
  y: number;
  alignX: Alignment;
  alignY: Alignment;
}

function computeAnchor(itemEl: HTMLElement): PreviewAnchor {
  const scaleFactor = systemState.currentMonitor.scaleFactor || globalThis.devicePixelRatio || 1;
  const elRect = itemEl.getBoundingClientRect();

  return computePreviewAnchor({
    large: isLargePreviewCard(previewSettings().cardWidth, scaleFactor),
    monitor: systemState.currentMonitor.rect,
    hitbox: settingsState.widgetRect.hitboxRect,
    itemCenterX: elRect.left + elRect.width / 2,
    itemCenterY: elRect.top + elRect.height / 2,
    dockSide: settingsState.position,
    scaleFactor,
  });
}

/**
 * Open/toggle the grouped preview for `windows`.
 *
 * Deduplicating: triggers for the same group/signature while the preview is
 * Opening/Open are no-ops unless the anchor position changed (then only the
 * position is refreshed). A new session id is issued on every real open so
 * stale webview events are ignored.
 */
export function triggerPreviewWidget(
  itemEl: HTMLElement,
  windows: UserAppWindow[],
  t0Date?: number,
): void {
  ensurePreviewListeners();

  const preview = previewSettings();
  const groupKey = windows[0] ? appKeyOf(windows[0]) : "";
  const signature = signatureOf(windows);
  const anchor = computeAnchor(itemEl);
  const anchorKey = `${anchor.x},${anchor.y}`;

  cancelHide();

  const current = session.current;
  const sameSession = current !== null &&
    current.groupKey === groupKey &&
    current.signature === signature &&
    (session.phase === "Open" || session.phase === "Opening");

  if (sameSession && current) {
    if (session.lastAnchorKey === anchorKey) {
      debug("preview-deduplicated");
      return;
    }
    // only the anchor moved: refresh position without rebuilding the session
    sendTrigger(groupKey, signature, current.id, windows, preview, anchor, t0Date);
    session.lastAnchorKey = anchorKey;
    debug("preview-request", { reason: "anchor-moved" });
    return;
  }

  const previewSessionId = crypto.randomUUID();
  session.current = { id: previewSessionId, groupKey, signature, hwnds: windows.map((w) => w.hwnd) };
  session.lastAnchorKey = anchorKey;
  session.phase = "Opening";
  session.pointerInsidePreview = false;
  sendTrigger(groupKey, signature, previewSessionId, windows, preview, anchor, t0Date);
  session.phase = "Open";
  debug("preview-request", { reason: "new-session" });
}

function sendTrigger(
  _groupKey: string,
  _signature: string,
  previewSessionId: string,
  windows: UserAppWindow[],
  preview: ReturnType<typeof previewSettings>,
  anchor: PreviewAnchor,
  t0Date?: number,
): void {
  invoke(SeelenCommand.TriggerWidget, {
    payload: {
      id: "@seelen/weg-preview" as WidgetId,
      desiredPosition: { x: anchor.x, y: anchor.y },
      alignX: anchor.alignX,
      alignY: anchor.alignY,
      customArgs: {
        previewSessionId,
        hwnds: windows.map((w) => w.hwnd),
        position: settingsState.position,
        t0Date,
        animated: preview.animated,
        animationDuration: preview.animationDuration,
      },
    },
  });
}

/** Start the single close timer for the preview (called on icon leave). */
export function schedulePreviewHide(graceCheck?: () => boolean): void {
  ensurePreviewListeners();
  session.graceCheck = graceCheck ?? null;
  armHide();
}

/** Clear all timers/listeners state for a fresh interaction. */
export function clearPreviewHideTimer(): void {
  cancelHide();
}

/** Whether the pointer is currently inside the live preview (grace #2). */
export function isPointerInsidePreview(): boolean {
  return session.pointerInsidePreview;
}
