<script lang="ts">
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import { createSortable } from "@dnd-kit/svelte/sortable";
  import { RestrictToHorizontalAxis, RestrictToVerticalAxis } from "@dnd-kit/abstract/modifiers";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import type { WindowEntry } from "@seelen-ui/lib/types";
  import { isHorizontalDock } from "../../weg/state/settings.svelte.ts";
  import { previewState, type PreviewCard } from "../state.svelte.ts";

  interface Props {
    entry: WindowEntry;
    iconPath: string | null;
    umid: string | null;
    index: number;
    preview: PreviewCard["preview"];
    stale: boolean;
    showTitles: boolean;
    titleLines: string;
    radius: number;
    aspectFixed: boolean;
    ratio: string;
  }

  let {
    entry,
    iconPath,
    umid,
    index,
    preview,
    stale,
    showTitles,
    titleLines,
    radius,
    aspectFixed,
    ratio,
  }: Props = $props();

  const titleInfo = $derived(previewState.titleInfo(entry));

  let interactionFrozen = false;
  const sortable = createSortable({
    get id() {
      return entry.hwnd;
    },
    get index() {
      return index;
    },
    get modifiers() {
      return isHorizontalDock() ? [RestrictToHorizontalAxis] : [RestrictToVerticalAxis];
    },
  });

  // Preview click lifecycle:
  //   pointerdown inside preview -> freeze the close timer
  //   click -> resolve exact HWND once -> native focus/restore once
  //   -> hide only the preview webview, keeping `@seelen/weg` visible
  function onPointerDown(): void {
    interactionFrozen = true;
  }

  function onClick() {
    invoke(SeelenCommand.WegFocusWindow, {
      identification: entry.hwnd.toString(16),
    });
    interactionFrozen = false;
    // Only the preview webview closes (warm keep, no reload/destroy).
    Widget.self.hide();
  }

  function onClose(e: MouseEvent) {
    e.stopPropagation();
    invoke(SeelenCommand.WegCloseApp, { hwnd: entry.hwnd });
  }

  function onAuxClick(e: MouseEvent) {
    if (e.button === 1) {
      invoke(SeelenCommand.WegCloseApp, { hwnd: entry.hwnd });
    }
  }
</script>

<div
  {@attach sortable.attach}
  role="button"
  tabindex="0"
  class="weg-item-preview"
  style="border-radius: {radius}px; {sortable.isDragging ? 'opacity: 0.3;' : ''}"
  data-title-lines={titleLines}
  data-title={titleInfo.label}
  onpointerdown={onPointerDown}
  onclick={onClick}
  onauxclick={onAuxClick}
  onkeypress={() => {}}
>
  {#if showTitles}
    <div class="weg-item-preview-topbar">
      <div class="weg-item-preview-title" title={titleInfo.tooltip ?? undefined}>
        {titleInfo.label}
      </div>
      <button data-skin="transparent" onclick={onClose}>
        <Icon iconName="IoClose" />
      </button>
    </div>
  {/if}
  <div
    class="weg-item-preview-image-container"
    style="
      border-radius: {radius}px;
      {aspectFixed ? `aspect-ratio: ${ratio};` : ""}
    "
  >
    {#if preview && !stale}
      <img
        class="weg-item-preview-image"
        src="data:image/webp;base64,{preview.data}"
        alt={titleInfo.label}
      />
    {:else}
      <!-- generation mismatch or missing frame: placeholder instead of a
           confidently wrong bitmap for the current content -->
      <MissingIcon class="weg-item-no-preview" />
    {/if}
  </div>
</div>
