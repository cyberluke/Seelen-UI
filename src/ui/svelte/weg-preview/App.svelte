<script lang="ts">
  import { Widget } from "@seelen-ui/lib";
  import { DragDropProvider, DragOverlay } from "@dnd-kit/svelte";
  import { move } from "@dnd-kit/helpers";
  import { onDestroy, onMount } from "svelte";
  import { previewState } from "./state.svelte.ts";
  import CardItem from "./components/CardItem.svelte";
  import { computePreviewLayout, previewSettings } from "../weg/state/preview.svelte.ts";
  import { systemState } from "../weg/state/system.svelte.ts";
  import { createDragDropManager } from "libs/ui/dnd.ts";

  const manager = createDragDropManager();
  onDestroy(() => manager.destroy());

  $effect(() => {
    Widget.getCurrent().ready();
  });

  $effect(() => {
    if (previewState.currentInteractables.length === 0) {
      Widget.self.hide();
    }
  });

  const s = $derived(previewSettings());
  const cards = $derived(previewState.currentCards);
  const layout = $derived(
    computePreviewLayout(
      previewState.currentInteractables.length,
      systemState.currentMonitor,
      previewState.position,
    ),
  );

  const cacheHit = $derived(
    !s.cacheEnabled || cards.some((c) => c.preview),
  );

  let painted = false;
  onMount(() => {
    requestAnimationFrame(() => {
      if (!painted) {
        painted = true;
        previewState.reportPaint(cacheHit);
      }
    });
  });

  // ── cross-webview pointer bridge (geometry + state only) ──────────────────
  // The parent `@seelen/weg` controller owns the single close timer; these
  // events only report pointer presence with the active session id so stale
  // events from older sessions are ignored.
  function emitBridge(event: string): void {
    Widget.self.webview
      .emit(event, { sessionId: previewState.session })
      .catch(() => {});
  }

  function handleDragOver(event: any): void {
    const ids = previewState.currentInteractables.map((w) => w.hwnd);
    const newIds: number[] = move(ids, event);
    const ordered = newIds
      .map((id) => previewState.currentInteractables.find((w) => w.hwnd === id))
      .filter((w): w is (typeof previewState.currentInteractables)[number] => !!w);
    previewState.persistOrder(ordered);
  }
</script>

<div
  class="weg-item-preview-container slu-std-popover"
  role="presentation"
  style="
    padding: {layout.padding}px;
    border-radius: {layout.borderRadius}px;
    max-width: {layout.popupWidth}px;
    max-height: {layout.popupHeight}px;
    {previewState.animated
      ? `transition: opacity ${previewState.animationDuration}ms ease, transform ${previewState.animationDuration}ms ease;`
      : "transition: none;"}
  "
  onpointerenter={() => emitBridge("weg-preview:pointer-enter")}
  onpointerleave={() => emitBridge("weg-preview:pointer-leave")}
>
  <DragDropProvider {manager} onDragOver={handleDragOver}>
    <div
      class="weg-item-preview-list"
      style="
        display: grid;
        grid-template-columns: repeat({layout.columns}, {layout.cardWidth}px);
        gap: {layout.gap}px;
        {layout.scrollable ? `overflow-y: auto; max-height: ${layout.popupHeight - layout.padding * 2}px;` : ""}
      "
    >
      {#each cards as card, i (card.entry.hwnd)}
        <CardItem
          entry={card.entry}
          iconPath={card.iconPath}
          umid={card.umid}
          index={i}
          preview={card.preview}
          stale={card.stale}
          showTitles={s.showTitles}
          titleLines={s.titleLines}
          radius={layout.borderRadius}
          aspectFixed={s.aspectRatioMode === "Fixed"}
          ratio="{s.aspectRatioNum} / {s.aspectRatioDen}"
        />
      {/each}
    </div>

    <DragOverlay>
      {#snippet children(source)}
        {@const item = previewState.currentInteractables.find((w) => w.hwnd === source.id)}
        {#if item}
          {@const title = previewState.titleInfo(item)}
          <div class="weg-item-preview weg-overlay" data-title={title.label}>
            {item.title}
          </div>
        {/if}
      {/snippet}
    </DragOverlay>
  </DragDropProvider>
</div>
