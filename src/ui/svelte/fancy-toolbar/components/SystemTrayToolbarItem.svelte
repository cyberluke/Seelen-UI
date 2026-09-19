<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import type { WidgetId } from "@seelen-ui/lib/types";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import type { IconName } from "libs/ui/icons";
  import { settingsState } from "../state/settings.svelte.ts";
  import {
    overflowIcons as _overflow,
    pinnedSet as _pinSet,
    resolvedPinned as _pinList,
  } from "../../system-tray/state.svelte.ts";
  import PinnedTrayIcon from "./PinnedTrayIcon.svelte";

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

  const resolvedPinned = $derived(_pinList());
  const pinnedSet = $derived(_pinSet());
  const overflow = $derived(_overflow());

  function onArrowClick(): void {
    invoke(SeelenCommand.TriggerWidget, {
      payload: { id: "@seelen/system-tray" as WidgetId },
    });
  }
</script>

<div class="ft-bar-tray-cluster" style:gap={spacingStyle || undefined}>
  {#if pinnedFirst}
    {#each resolvedPinned as entry, i (entry.logicalId)}
      <PinnedTrayIcon
        {entry}
        index={i}
        list={resolvedPinned}
        {sizeStyle}
        {iconMinStyle}
        {showTips}
        {draggable}
      />
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
      <PinnedTrayIcon
        {entry}
        index={i}
        list={resolvedPinned}
        {sizeStyle}
        {iconMinStyle}
        {showTips}
        {draggable}
      />
    {/each}
  {/if}
  {#if !pinnedSet.size && !showArrow && overflow.length === 0}
    <span class="ft-bar-tray-empty"></span>
  {/if}
</div>
