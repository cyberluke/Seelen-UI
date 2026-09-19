<script lang="ts">
  import { SystrayIconAction } from "@seelen-ui/lib/types";
  import {
    overflowIcons,
    pinnedSet,
    togglePin,
  } from "./state.svelte.ts";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon";

  $effect(() => {
    Widget.getCurrent().ready();
  });

  const GUIDS_TO_IGNORE = [
    "7820ae73-23e3-4229-82c1-e41cb67d5b9c", // speaker volume icon
    "7820ae74-23e3-4229-82c1-e41cb67d5b9c", // network icon
    "7820ae75-23e3-4229-82c1-e41cb67d5b9c", // battery icon
  ];

  function onClick(event: MouseEvent, logicalId: string): void {
    if (event.detail === 2) return;
    let action: SystrayIconAction = SystrayIconAction.LeftClick;
    if (event.button === 1) action = SystrayIconAction.MiddleClick;
    else if (event.button === 2) action = SystrayIconAction.RightClick;
    invoke(SeelenCommand.SendTrayAction, { logicalId, action });
  }

  function onDoubleClick(e: MouseEvent, logicalId: string): void {
    e.preventDefault();
    e.stopPropagation();
    invoke(SeelenCommand.SendTrayAction, {
      logicalId,
      action: SystrayIconAction.LeftDoubleClick,
    });
  }

  function onPin(e: MouseEvent, logicalId: string): void {
    e.stopPropagation();
    void togglePin(logicalId);
  }
</script>

<div class={["slu-std-popover", "system-tray"]}>
  {#each overflowIcons() as item}
    {#if item.guid && GUIDS_TO_IGNORE.includes(item.guid)}
      <!-- skip well-known synthetic icons -->
    {:else}
      {@const logicalId = item.logicalId}
      <div
        class="system-tray-item"
        role="button"
        tabindex="0"
        onpointerdown={(e) => onClick(e, logicalId)}
        ondblclick={(e) => onDoubleClick(e, logicalId)}
        onmouseenter={() => {
          invoke(SeelenCommand.SendTrayAction, {
            logicalId,
            action: SystrayIconAction.HoverEnter,
          });
        }}
        onmouseleave={() => {
          invoke(SeelenCommand.SendTrayAction, {
            logicalId,
            action: SystrayIconAction.HoverLeave,
          });
        }}
      >
        <div class="system-tray-item-icon-box">
          {#if !!item.iconPath}
            <img
              class="system-tray-item-icon"
              src={convertFileSrc(item.iconPath) +
                `?hash=${item.iconImageHash || "null"}`}
              alt=""
            />
          {:else}
            <MissingIcon class="system-tray-item-icon" />
          {/if}
        </div>
        <span class="system-tray-item-label">
          {item.tooltip ||
            item.guid ||
            `${item.windowHandle?.toString(16)}::${item.uid}`}
        </span>
        <button
          class="system-tray-pin"
          data-skin="transparent"
          data-state={pinnedSet().has(logicalId) ? "pinned" : "unpinned"}
          onclick={(e) => onPin(e, logicalId)}
        >
          <Icon
            name={pinnedSet().has(logicalId) ? "VscPinned" : "VscPin"}
            class="system-tray-pin-icon"
          />
        </button>
      </div>
    {/if}
  {/each}
</div>
