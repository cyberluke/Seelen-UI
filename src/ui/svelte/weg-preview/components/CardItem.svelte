<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { createSortable } from "@dnd-kit/svelte/sortable";
  import { RestrictToHorizontalAxis, RestrictToVerticalAxis } from "@dnd-kit/abstract/modifiers";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import type { UserAppWindow } from "@seelen-ui/lib/types";
  import { isHorizontalDock } from "../../weg/state/settings.svelte.ts";

  interface Props {
    win: UserAppWindow;
    index: number;
    label: string;
    tooltip: string | null;
    preview?: { data: string } | undefined;
    showTitles: boolean;
    titleLines: string;
    radius: number;
    aspectFixed: boolean;
    ratio: string;
  }

  let {
    win,
    index,
    label,
    tooltip,
    preview,
    showTitles,
    titleLines,
    radius,
    aspectFixed,
    ratio,
  }: Props = $props();

  const sortable = createSortable({
    get id() {
      return win.hwnd;
    },
    get index() {
      return index;
    },
    get modifiers() {
      return isHorizontalDock() ? [RestrictToHorizontalAxis] : [RestrictToVerticalAxis];
    },
  });

  function onClick() {
    invoke(SeelenCommand.WegToggleWindowState, {
      hwnd: win.hwnd,
      wasFocused: false,
    });
  }

  function onClose(e: MouseEvent) {
    e.stopPropagation();
    invoke(SeelenCommand.WegCloseApp, { hwnd: win.hwnd });
  }

  function onAuxClick(e: MouseEvent) {
    if (e.button === 1) {
      invoke(SeelenCommand.WegCloseApp, { hwnd: win.hwnd });
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
  data-title={label}
  onclick={onClick}
  onauxclick={onAuxClick}
  onkeypress={() => {}}
>
  {#if showTitles}
    <div class="weg-item-preview-topbar">
      <div class="weg-item-preview-title" title={tooltip ?? undefined}>{label}</div>
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
    {#if preview}
      <img
        class="weg-item-preview-image"
        src="data:image/webp;base64,{preview.data}"
        alt={label}
      />
    {:else}
      <MissingIcon class="weg-item-no-preview" />
    {/if}
  </div>
</div>
