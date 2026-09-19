<script lang="ts">
  import type { AppOrFileWegItem } from "../../types.ts";
  import { settingsState } from "../../state/settings.svelte.ts";
  import { interactables, getWindowsForItem } from "../../state/windows.svelte.ts";
  import { isHorizontalDockSide } from "../../state/geometry.ts";
  import UserApplicationItem from "./UserApplicationItem.svelte";

  interface Props {
    item: AppOrFileWegItem;
    isOverlay?: boolean;
  }

  let { item, isOverlay = false }: Props = $props();

  const windows = $derived(getWindowsForItem(item, interactables.value));
  const settings = $derived(settingsState.value as any);
  const showAsSeparatedItems = $derived(settings?.splitWindows);
  // Orientation is the single layout source: left/right docks flow the split
  // items along the vertical primary axis, top/bottom along the horizontal one.
  const horizontal = $derived(isHorizontalDockSide(settingsState.position));
</script>

{#if showAsSeparatedItems && windows.length > 1}
  <div
    class="weg-split-items"
    style="display: flex; flex-direction: {horizontal ? "row" : "column"}; align-items: center; gap: {settings?.spaceBetweenItems ?? 0}px;"
  >
    {#each windows as win (win.hwnd)}
      <UserApplicationItem {item} windows={[win]} />
    {/each}
  </div>
{:else}
  <UserApplicationItem {item} {windows} />
{/if}
