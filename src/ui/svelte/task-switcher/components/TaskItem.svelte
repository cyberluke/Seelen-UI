<script lang="ts">
  import type { WindowEntry } from "@seelen-ui/lib/types";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { globalState } from "../state/index.svelte.ts";
  import { windows } from "../state/getters.svelte.ts";
  import { FileIcon, Icon } from "libs/ui/svelte/components/Icon";
  import MissingIcon from "libs/ui/svelte/components/Icon/MissingIcon.svelte";

  interface Props {
    task: WindowEntry;
    index: number;
  }

  let { task, index }: Props = $props();

  let boxRef: HTMLDivElement | undefined = $state();
  const isSelected = $derived(task.hwnd === globalState.selectedWindow);
  const preview = $derived(globalState.previews[task.hwnd]);
  const label = $derived(task.alias || task.displayTitle || task.title);

  // The shared window icon comes from the raw metadata table
  const raw = $derived(windows.value.find((w) => w.hwnd === task.hwnd));

  // Focus button when selected
  $effect(() => {
    if (isSelected && boxRef) {
      boxRef.focus();
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    // Handle Enter key to activate the window
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      boxRef?.click();
      return;
    }

    // Handle navigation keys
    const isNavigationKey = e.key === "Tab" || e.key === "ArrowRight" || e.key === "ArrowLeft";

    if (isNavigationKey) {
      e.preventDefault();

      const direction =
        e.key === "ArrowLeft" || (e.key === "Tab" && e.shiftKey) ? "previous" : "next";

      navigateToItem(direction, index);
    }
  }

  function handleClick() {
    globalState.showing = false;
    // Task Switcher selection = direct native focus action (no dock toggling)
    invoke(SeelenCommand.WegFocusWindow, {
      identification: task.runtimeWindowId,
    });
  }

  function handleFocus() {
    globalState.selectedWindow = task.hwnd;
  }

  // Navigation helper functions
  function getNextIndex(currentIndex: number, totalItems: number): number {
    return (currentIndex + 1) % totalItems;
  }

  function getPreviousIndex(currentIndex: number, totalItems: number): number {
    return (currentIndex - 1 + totalItems) % totalItems;
  }

  function navigateToItem(direction: "next" | "previous", currentIndex: number): void {
    // walk the frozen session order exactly as displayed
    const windows = globalState.windows;
    const totalItems = windows.length;

    if (totalItems === 0) return;

    const nextIndex =
      direction === "next"
        ? getNextIndex(currentIndex, totalItems)
        : getPreviousIndex(currentIndex, totalItems);

    globalState.selectedWindow = windows[nextIndex]?.hwnd || null;
  }
</script>

<div
  bind:this={boxRef}
  class="task"
  role="button"
  tabindex="0"
  onkeydown={handleKeyDown}
  onclick={handleClick}
  onfocus={handleFocus}
>
  <div class="task-header">
    <FileIcon class="task-icon" path={raw?.relaunch?.icon || raw?.process.path} umid={raw?.umid ?? undefined} />
    <div class="task-title">{label}</div>
    <button
      data-skin="transparent"
      onclick={(e) => {
        e.stopPropagation();
        invoke(SeelenCommand.WegCloseApp, { hwnd: task.hwnd });
      }}
    >
      <Icon iconName="TbX" />
    </button>
  </div>
  <div class="task-preview-container">
    {#if preview && preview.titleAtCapture === task.title}
      <img class="task-preview" src={`data:image/webp;base64,${preview.data}`} alt="" />
    {:else}
      <!-- placeholder instead of a stale frame from another content generation -->
      <MissingIcon class="task-no-preview" />
    {/if}
  </div>
</div>
