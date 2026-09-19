<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { WegMiddleClickAction, type UserAppWindow } from "@seelen-ui/lib/types";
  import { FileIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import { t } from "../../i18n/index.ts";
  import type { AppOrFileWegItem } from "../../types.ts";
  import { settingsState } from "../../state/settings.svelte.ts";
  import { windowsState, focused } from "../../state/windows.svelte.ts";
  import { mousePos, notifications } from "../../state/getters.svelte.ts";
  import { previewSettings } from "../../state/preview.svelte.ts";
  import { Widget } from "@seelen-ui/lib";
  import { getUserApplicationContextMenu, launchItem } from "../../appMenu.ts";
  import { triggerPreviewWidget } from "../../previewWidget.ts";
  import { CssHandled } from "libs/ui/svelte/utils/animations.ts";

  interface Props {
    item: AppOrFileWegItem;
    windows: UserAppWindow[];
  }

  let { item, windows }: Props = $props();

  const settings = $derived(settingsState.value as any);
  const notificationsCount = $derived(
    notifications.value.filter((n: any) => n.appUmid === item.umid).length,
  );
  const itemLabel = $derived(
    settings?.showWindowTitle && windows.length ? windows[0]!.title : null,
  );
  const isFocused = $derived(windows.some((w) => w.hwnd === focused.value?.hwnd));

  let itemEl: HTMLDivElement | null = $state(null);

  let openTimer: ReturnType<typeof setTimeout> | null = null;
  let closeTimer: ReturnType<typeof setTimeout> | null = null;

  function triggerNow() {
    if (openTimer !== null) {
      clearTimeout(openTimer);
      openTimer = null;
    }
    if (closeTimer !== null) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
    if (windows.length > 1) {
      const action = previewSettings().groupedClickAction;
      switch (action) {
        case "ActivateLastUsed": {
          const lastUsed = [...windows].sort(
            (a, b) => b.lastForegroundAt - a.lastForegroundAt,
          )[0];
          if (lastUsed) {
            invoke(SeelenCommand.WegToggleWindowState, {
              hwnd: lastUsed.hwnd,
              wasFocused: windowsState.delayedFocused?.hwnd === lastUsed.hwnd,
            });
          }
          return;
        }
        case "MinimizeRestoreGroup": {
          const win = windows[0];
          if (win) {
            invoke(SeelenCommand.WegToggleWindowState, {
              hwnd: win.hwnd,
              wasFocused: windowsState.delayedFocused?.hwnd === win.hwnd,
            });
          }
          return;
        }
        default:
          triggerPreviewWidget(itemEl!, windows, Date.now());
      }
      return;
    }
    if (!windows[0]) {
      launchItem(item, false);
    } else {
      invoke(SeelenCommand.WegToggleWindowState, {
        hwnd: windows[0].hwnd,
        wasFocused: windowsState.delayedFocused?.hwnd === windows[0].hwnd,
      });
    }
  }

  function scheduleHide() {
    const { hoverCloseDelay, keepOpenOnTraversal } = previewSettings();
    if (closeTimer !== null) {
      clearTimeout(closeTimer);
    }
    closeTimer = setTimeout(() => {
      closeTimer = null;
      if (keepOpenOnTraversal && isPointerInsideGrace()) {
        return;
      }
      Widget.self.hide();
    }, hoverCloseDelay);
  }

  const graceRect = $derived.by(() => {
    const el = itemEl;
    if (!el) return null;
    const r = el.getBoundingClientRect();
    const grace = previewSettings().pointerGraceRegion * (globalThis.devicePixelRatio || 1);
    return {
      left: r.left - grace,
      right: r.right + grace,
      top: r.top - grace,
      bottom: r.bottom + grace,
    };
  });

  function insideRect(
    rect: { left: number; right: number; top: number; bottom: number } | null,
    x: number,
    y: number,
  ): boolean {
    if (!rect) return false;
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  }

  function isPointerInsideGrace(): boolean {
    const pos = mousePos.value;
    const scale = globalThis.devicePixelRatio || 1;
    return insideRect(graceRect, pos.x / scale, pos.y / scale);
  }

  function onPointerEnter() {
    const { trigger, hoverOpenDelay } = previewSettings();
    if (trigger === "Click") return;
    // Hover is meaningful only for grouped apps (>1 windows). Single-window
    // and ungrouped items behave like classic taskbar buttons and only react
    // to click. Grouped-only hover keeps the pointer model cheap and matches
    // the native notification-area preview behaviour.
    if (windows.length < 2) return;
    if (hoverOpenDelay <= 0) {
      triggerNow();
    } else {
      if (openTimer !== null) {
        clearTimeout(openTimer);
      }
      openTimer = setTimeout(() => {
        openTimer = null;
        triggerNow();
      }, hoverOpenDelay);
    }
  }

  function onPointerLeave() {
    if (previewSettings().trigger === "Click") return;
    if (windows.length < 2) return;
    scheduleHide();
  }

  function onPointerMove(e: PointerEvent) {
    if (closeTimer === null) return;
    if (windows.length < 2) return;
    if (insideRect(graceRect, e.clientX, e.clientY)) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
  }

  function onClick() {
    if (closeTimer !== null) {
      clearTimeout(closeTimer);
      closeTimer = null;
    }
    if (windows.length > 1) {
      triggerNow();
      return;
    }
    if (!windows[0]) {
      launchItem(item, false);
    } else {
      invoke(SeelenCommand.WegToggleWindowState, {
        hwnd: windows[0].hwnd,
        wasFocused: windowsState.delayedFocused?.hwnd === windows[0].hwnd,
      });
    }
  }

  function onAuxClick(e: MouseEvent) {
    if (e.button !== 1) return;
    if (settings?.middleClickAction === WegMiddleClickAction.OpenNewInstance) {
      launchItem(item, false);
    } else {
      const win = windows[0];
      if (win) invoke(SeelenCommand.WegCloseApp, { hwnd: win.hwnd });
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.stopPropagation();
    const alignX = settingsState.popupAlignX;
    const alignY = settingsState.popupAlignY;
    invoke(SeelenCommand.TriggerContextMenu, {
      menu: { ...getUserApplicationContextMenu($t, item, windows), alignX, alignY },
      forwardTo: null,
    });
  }
</script>

<div
  bind:this={itemEl}
  role="menu"
  tabindex="0"
  class="weg-item-overlay"
>
  <div
    role="menuitem"
    tabindex="0"
    class="weg-item"
    data-tooltip={item.displayName}
    data-tooltip-origin-y={settingsState.tooltipOrigin.y}
    data-tooltip-origin-x={settingsState.tooltipOrigin.x}
    data-tooltip-align-x={settingsState.popupAlignX}
    data-tooltip-align-y={settingsState.popupAlignY}
    onclick={onClick}
    onauxclick={onAuxClick}
    oncontextmenu={onContextMenu}
    onkeypress={() => {}}
    onpointerenter={onPointerEnter}
    onpointerleave={onPointerLeave}
    onpointermove={onPointerMove}
  >
    <FileIcon class="weg-item-icon" path={item.relaunch?.icon || item.path} umid={item.umid} />
    {#if itemLabel}
      <div class="weg-item-title">{itemLabel}</div>
    {/if}
  </div>

  {#if notificationsCount > 0}
    <div class="weg-item-notification-badge" transition:CssHandled>
      {notificationsCount}
    </div>
  {/if}

  {#if settings?.showInstanceCounter && windows.length > 1}
    <div class="weg-item-instance-counter-badge" transition:CssHandled>
      {windows.length}
    </div>
  {/if}

  {#if !settings?.showWindowTitle}
    <div
      class="weg-item-open-sign"
      class:weg-item-open-sign-active={windows.length > 0}
      class:weg-item-open-sign-focused={isFocused}
    ></div>
  {/if}
</div>
