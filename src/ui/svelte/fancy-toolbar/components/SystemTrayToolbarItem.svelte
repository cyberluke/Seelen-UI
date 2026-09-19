<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { SystrayIconAction } from "@seelen-ui/lib/types";
  import type { SysTrayIcon, WidgetId } from "@seelen-ui/lib/types";
  import { MissingIcon, Icon } from "libs/ui/svelte/components/Icon";
  import type { IconName } from "libs/ui/icons";
  import { settingsState } from "../state/settings.svelte.ts";
  import {
    overflowIcons as _overflow,
    pinnedSet as _pinSet,
    resolvedPinned as _pinList,
    sendAction,
    setPinnedOrder,
  } from "../../system-tray/state.svelte.ts";

  const ARROW_ICON: IconName = "IoIosArrowDropdown";

  const traySettings = $derived(settingsState.traySettings);
  const pinnedFirst = $derived(
    !traySettings || traySettings.pinnedPosition !== "AfterOverflow",
  );
  const showArrow = $derived(traySettings ? traySettings.overflowArrowEnabled : true);
  const showTips = $derived(traySettings ? traySettings.showTooltips : true);
  const draggable = $derived(traySettings ? traySettings.dragReorderEnabled : true);

  const spacingStyle = $derived(traySettings?.iconSpacing ? `${traySettings.iconSpacing}px` : "");
  const sizeStyle = $derived(
    traySettings?.iconSize ? `${traySettings.iconSize}px` : "var(--config-item-size)",
  );
  const iconMinStyle = $derived(sizeStyle);

  interface LocalResolved {
    raw: SysTrayIcon;
    logicalId: string;
    pinnedOrder: number;
    tooltip: string;
  }

  const resolvedPinned = $derived(_pinList());
  const pinnedSet = $derived(_pinSet());
  const overflow = $derived(_overflow());

  let dragFrom: number | null = null;

  function iconUrl(item: LocalResolved): string | null {
    if (!item.raw.iconPath) return null;
    return convertFileSrc(item.raw.iconPath) + `?hash=${item.raw.iconImageHash || "null"}`;
  }

  function forward(e: MouseEvent, logicalId: string, dbl = false): void {
    if (dbl) {
      e.preventDefault();
      e.stopPropagation();
      void sendAction(logicalId, SystrayIconAction.LeftDoubleClick);
      return;
    }
    if (e.detail === 2) return;
    let action: SystrayIconAction = SystrayIconAction.LeftClick;
    if (e.button === 1) action = SystrayIconAction.MiddleClick;
    else if (e.button === 2) action = SystrayIconAction.RightClick;
    void sendAction(logicalId, action);
  }

  function onArrowClick(): void {
    invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/system-tray" as WidgetId },
    });
  }

  function handleDragStart(index: number, e: DragEvent): void {
    if (!draggable) return;
    dragFrom = index;
    e.dataTransfer?.setData("text/plain", String(index));
  }

  function handleDrop(target: number, list: LocalResolved[]): void {
    if (!draggable || dragFrom === null) return;
    const order = list.map((i) => i.logicalId);
    if (order.length === 0) return;
    const [moved] = order.splice(dragFrom, 1);
    const insertAt = target > dragFrom ? target - 1 : target;
    if (moved !== undefined) order.splice(insertAt, 0, moved);
    // Preserve any pinned logical ids currently offline so reorder does not
    // silently drop them.
    const allPinned = traySettings?.pinned ?? [];
    for (const k of allPinned) {
      if (!order.includes(k)) order.push(k);
    }
    void setPinnedOrder(order);
    dragFrom = null;
  }
</script>

<div class="ft-bar-tray-cluster" style:gap={spacingStyle || undefined}>
  {#if pinnedFirst}
    {#each resolvedPinned as entry, i (entry.logicalId)}
      <div
        class="ft-bar-tray-pinned"
        role="button"
        tabindex="0"
        draggable={draggable}
        data-offline={!entry.raw.isVisible}
        data-tooltip={showTips ? entry.tooltip : null}
        data-tooltip-align-x="Center"
        data-tooltip-align-y={settingsState.tooltipAlignY}
        data-tooltip-origin-y={settingsState.tooltipOriginY}
        style:width={sizeStyle}
        style:min-height={iconMinStyle}
        onpointerdown={(e) => forward(e as MouseEvent, entry.logicalId)}
        ondblclick={(e) => forward(e, entry.logicalId, true)}
        ondragstart={(e) => handleDragStart(i, e)}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => {
          e.preventDefault();
          handleDrop(i, resolvedPinned);
        }}
        onmouseenter={() => {
          if (entry.raw.isVisible) {
            invoke(SeelenCommand.SendTrayAction, {
              logicalId: entry.logicalId,
              action: SystrayIconAction.HoverEnter,
            });
          }
        }}
        onmouseleave={() => {
          if (entry.raw.isVisible) {
            invoke(SeelenCommand.SendTrayAction, {
              logicalId: entry.logicalId,
              action: SystrayIconAction.HoverLeave,
            });
          }
        }}
      >
        {#if iconUrl(entry)}
          <img class="ft-bar-tray-pinned-icon" src={iconUrl(entry)} alt="" />
        {:else}
          <MissingIcon class="ft-bar-tray-pinned-icon" />
        {/if}
      </div>
    {/each}
    {#if showArrow}
      <div
        class="ft-bar-tray-overflow"
        id="@seelen/tb-system-tray"
        role="button"
        tabindex="0"
        data-plugin-id="@seelen/tb-system-tray"
        data-tooltip="System Tray"
        data-tooltip-align-x="Center"
        data-tooltip-align-y={settingsState.tooltipAlignY}
        data-tooltip-origin-y={settingsState.tooltipOriginY}
        style:width={sizeStyle}
        style:min-height={iconMinStyle}
        onclick={onArrowClick}
        onkeypress={onArrowClick}
      >
        <Icon class="ft-bar-tray-overflow-icon" name={ARROW_ICON} />
      </div>
    {/if}
  {:else}
    {#if showArrow}
      <div
        class="ft-bar-tray-overflow"
        id="@seelen/tb-system-tray"
        role="button"
        tabindex="0"
        data-plugin-id="@seelen/tb-system-tray"
        data-tooltip="System Tray"
        data-tooltip-align-x="Center"
        data-tooltip-align-y={settingsState.tooltipAlignY}
        data-tooltip-origin-y={settingsState.tooltipOriginY}
        style:width={sizeStyle}
        style:min-height={iconMinStyle}
        onclick={onArrowClick}
        onkeypress={onArrowClick}
      >
        <Icon class="ft-bar-tray-overflow-icon" name={ARROW_ICON} />
      </div>
    {/if}
    {#each resolvedPinned as entry, i (entry.logicalId)}
      <div
        class="ft-bar-tray-pinned"
        role="button"
        tabindex="0"
        draggable={draggable}
        data-offline={!entry.raw.isVisible}
        data-tooltip={showTips ? entry.tooltip : null}
        data-tooltip-align-x="Center"
        data-tooltip-align-y={settingsState.tooltipAlignY}
        data-tooltip-origin-y={settingsState.tooltipOriginY}
        style:width={sizeStyle}
        style:min-height={iconMinStyle}
        onpointerdown={(e) => forward(e as MouseEvent, entry.logicalId)}
        ondblclick={(e) => forward(e, entry.logicalId, true)}
        ondragstart={(e) => handleDragStart(i, e)}
        ondragover={(e) => e.preventDefault()}
        ondrop={(e) => {
          e.preventDefault();
          handleDrop(i, resolvedPinned);
        }}
      >
        {#if iconUrl(entry)}
          <img class="ft-bar-tray-pinned-icon" src={iconUrl(entry)} alt="" />
        {:else}
          <MissingIcon class="ft-bar-tray-pinned-icon" />
        {/if}
      </div>
    {/each}
  {/if}
  {#if !pinnedSet.size && !showArrow && overflow.length === 0}
    <span class="ft-bar-tray-empty"></span>
  {/if}
</div>
