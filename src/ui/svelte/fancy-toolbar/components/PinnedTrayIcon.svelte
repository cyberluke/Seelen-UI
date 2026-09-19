<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { SystrayIconAction } from "@seelen-ui/lib/types";
  import { MissingIcon } from "libs/ui/svelte/components/Icon";
  import { settingsState } from "../state/settings.svelte.ts";
  import { sendAction, setPinnedOrder } from "../../system-tray/state.svelte.ts";
  import type { ResolvedTrayIcon } from "../../system-tray/state.svelte.ts";

  // Single implementation of pinned tray icon pointer semantics, shared by
  // both render branches (pinned-before-overflow / pinned-after-overflow) so
  // they can never diverge again.
  //
  // Gesture ownership:
  //   left click        -> SystrayIconAction.LeftClick   (tray app owns it)
  //   double click      -> SystrayIconAction.LeftDoubleClick
  //   middle click      -> SystrayIconAction.MiddleClick
  //   right click       -> SystrayIconAction.RightClick via `contextmenu`,
  //                        consumed here with preventDefault + stopPropagation
  //                        so the generic Fancy Toolbar menu never receives it.

  interface Props {
    entry: ResolvedTrayIcon;
    index: number;
    list: ResolvedTrayIcon[];
    sizeStyle: string;
    iconMinStyle: string;
    showTips: boolean;
    draggable: boolean;
  }

  let { entry, index, list, sizeStyle, iconMinStyle, showTips, draggable }: Props = $props();

  let dragFrom: number | null = $state(null);

  const iconUrl = $derived.by(() => {
    if (!entry.raw.iconPath) return null;
    return convertFileSrc(entry.raw.iconPath) + `?hash=${entry.raw.iconImageHash || "null"}`;
  });

  function onClick(e: MouseEvent): void {
    // second press of a double click is handled by `ondblclick`
    if (e.detail === 2) return;
    console.debug(
      "[tray]",
      JSON.stringify({
        logicalId: entry.logicalId,
        domEvent: "click",
        trayAction: "left-click",
        propagationStopped: false,
      }),
    );
    void sendAction(entry.logicalId, SystrayIconAction.LeftClick);
  }

  function onDblClick(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    console.debug(
      "[tray]",
      JSON.stringify({
        logicalId: entry.logicalId,
        domEvent: "dblclick",
        trayAction: "left-double-click",
        propagationStopped: true,
      }),
    );
    void sendAction(entry.logicalId, SystrayIconAction.LeftDoubleClick);
  }

  function onAuxClick(e: MouseEvent): void {
    if (e.button !== 1) return;
    console.debug(
      "[tray]",
      JSON.stringify({
        logicalId: entry.logicalId,
        domEvent: "auxclick",
        trayAction: "middle-click",
        propagationStopped: true,
      }),
    );
    e.stopPropagation();
    void sendAction(entry.logicalId, SystrayIconAction.MiddleClick);
  }

  function onTrayContextMenu(e: MouseEvent): void {
    // The tray application owns this gesture: consume the DOM event so the
    // root Fancy Toolbar context menu handler never sees it.
    e.preventDefault();
    e.stopPropagation();
    console.debug(
      "[tray]",
      JSON.stringify({
        logicalId: entry.logicalId,
        domEvent: "contextmenu",
        trayAction: "right-click",
        propagationStopped: true,
      }),
    );
    void sendAction(entry.logicalId, SystrayIconAction.RightClick);
  }

  function onEnter(): void {
    if (entry.raw.isVisible) {
      invoke(SeelenCommand.SendTrayAction, {
        logicalId: entry.logicalId,
        action: SystrayIconAction.HoverEnter,
      });
    }
  }

  function onLeave(): void {
    if (entry.raw.isVisible) {
      invoke(SeelenCommand.SendTrayAction, {
        logicalId: entry.logicalId,
        action: SystrayIconAction.HoverLeave,
      });
    }
  }

  function handleDragStart(e: DragEvent): void {
    if (!draggable) return;
    dragFrom = index;
    e.dataTransfer?.setData("text/plain", String(index));
  }

  function handleDrop(target: number): void {
    if (!draggable || dragFrom === null) return;
    const order = list.map((i) => i.logicalId);
    if (order.length === 0) return;
    const [moved] = order.splice(dragFrom, 1);
    const insertAt = target > dragFrom ? target - 1 : target;
    if (moved !== undefined) order.splice(insertAt, 0, moved);
    // Preserve any pinned logical ids currently offline so reorder does not
    // silently drop them.
    const allPinned = settingsState.traySettings?.pinned ?? [];
    for (const k of allPinned) {
      if (!order.includes(k)) order.push(k);
    }
    void setPinnedOrder(order);
    dragFrom = null;
  }
</script>

<div
  class="ft-bar-tray-pinned"
  role="button"
  tabindex="0"
  draggable={draggable}
  data-context-owner="tray"
  data-offline={!entry.raw.isVisible}
  data-tooltip={showTips ? entry.tooltip : null}
  data-tooltip-align-x="Center"
  data-tooltip-align-y={settingsState.tooltipAlignY}
  data-tooltip-origin-y={settingsState.tooltipOriginY}
  style:width={sizeStyle}
  style:min-height={iconMinStyle}
  onclick={onClick}
  onkeypress={(e) => {
    if (e.key === "Enter") {
      void sendAction(entry.logicalId, SystrayIconAction.LeftClick);
    }
  }}
  ondblclick={onDblClick}
  onauxclick={onAuxClick}
  oncontextmenu={onTrayContextMenu}
  ondragstart={handleDragStart}
  ondragover={(e) => e.preventDefault()}
  ondrop={(e) => {
    e.preventDefault();
    handleDrop(index);
  }}
  onmouseenter={onEnter}
  onmouseleave={onLeave}
>
  {#if iconUrl}
    <img class="ft-bar-tray-pinned-icon" src={iconUrl} alt="" />
  {:else}
    <MissingIcon class="ft-bar-tray-pinned-icon" />
  {/if}
</div>
