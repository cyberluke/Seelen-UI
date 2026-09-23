<script lang="ts">
  import { t } from "../i18n";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { StartView } from "../constants";
  import { globalState } from "../state/mod.svelte";
  import { naiState } from "../state/nai.svelte";

  async function launch(id: string) {
    await invoke(SeelenCommand.NaiLaunch, { id });
  }

  async function storeAction(
    command: (typeof SeelenCommand)[keyof typeof SeelenCommand],
    id: string,
  ) {
    await invoke(command, { id });
    await naiState.refreshCatalog();
  }

  const entries = $derived(
    (naiState.catalog as { entries?: { id: string; name: string; description: string }[] } | null)
      ?.entries ?? [],
  );
</script>

{#if globalState.view === StartView.Home}
  <div class="nai-hero">
    <h2 class="nai-hero-title">{$t("nav.home")}</h2>
    <ul class="nai-hero-list">
      {#each (naiState.apps as any[] | null) ?? [] as app (app.id)}
        <li class="nai-hero-item">
          <button data-skin="default" onclick={() => launch(app.id)}>
            <span class="nai-hero-name">{app.name}</span>
            <span class="nai-hero-desc">{app.description}</span>
            {#if app.runningPids?.length}
              <span class="nai-hero-state" data-state="active">•</span>
            {:else}
              <span class="nai-hero-state">○</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{:else if globalState.view === StartView.AiApps}
  <div class="nai-store">
    {#each entries as entry (entry.id)}
      <div class="nai-store-row">
        <div class="nai-store-meta">
          <span class="nai-store-name">{entry.name}</span>
          <span class="nai-store-desc">{entry.description}</span>
        </div>
        <div class="nai-store-actions">
          <button data-skin="solid" onclick={() => storeAction(SeelenCommand.NaiInstall, entry.id)}>
            {$t("store.install")}
          </button>
          <button data-skin="default" onclick={() => storeAction(SeelenCommand.NaiUpdate, entry.id)}>
            {$t("store.update")}
          </button>
          <button
            data-skin="default"
            onclick={() => storeAction(SeelenCommand.NaiUninstall, entry.id)}>{$t("store.uninstall")}</button
          >
          <button
            data-skin="transparent"
            onclick={() => storeAction(SeelenCommand.NaiStoreLaunch, entry.id)}>{$t("store.launch")}</button
          >
        </div>
      </div>
    {/each}
  </div>
{:else if globalState.view === StartView.Activities}
  <div class="nai-activities">
    {#each (naiState.activities as any[] | null) ?? [] as activity (activity.id)}
      <div class="nai-activity">
        <span class="nai-activity-name">{activity.name}</span>
        <span class="nai-activity-count">{activity.objects.length}</span>
      </div>
    {/each}
    {#each (naiState.capsules as any[] | null) ?? [] as capsule (capsule.id)}
      <div class="nai-capsule">
        <span class="nai-capsule-name">{capsule.name}</span>
        <span class="nai-capsule-activity">{capsule.activity}</span>
      </div>
    {/each}
  </div>
{:else if globalState.view === StartView.Agents}
  <div class="nai-activities">
    {#each ((naiState.apps as any[] | null) ?? []).filter((a) => a.runningPids?.length) as agent (agent.id)}
      <div class="nai-activity">
        <span class="nai-activity-name">{agent.name}</span>
        <span class="nai-activity-count">{agent.runningPids.join(", ")}</span>
      </div>
    {/each}
  </div>
{:else if globalState.view === StartView.Media}
  <div class="nai-media">
    {#each (naiState.media as any[] | null) ?? [] as session (session.id)}
      <div class="nai-media-item">
        <span>{session.title}</span>
        <span>{session.author}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .nai-hero-title {
    font-size: 0.9em;
    color: var(--nai-lavender-core, #8d7cff);
    padding: 4px 0;
  }
  .nai-hero-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 6px;
  }
  .nai-hero-item button {
    display: grid;
    gap: 2px;
    text-align: left;
    padding: 8px;
    border-radius: 8px;
    background: var(--nai-evening-surface, #10101a);
  }
  .nai-hero-name {
    font-weight: 600;
  }
  .nai-hero-desc,
  .nai-store-desc,
  .nai-activity-count,
  .nai-capsule-activity {
    font-size: 0.8em;
    opacity: 0.7;
  }
  .nai-store-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
  }
  .nai-store-meta {
    display: grid;
    min-width: 0;
  }
  .nai-store-actions {
    display: flex;
    gap: 4px;
  }
  .nai-activities,
  .nai-media {
    display: grid;
    gap: 4px;
  }
  .nai-activity,
  .nai-capsule,
  .nai-media-item {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  @media (prefers-reduced-motion: reduce) {
    * {
      animation: none;
      transition: none;
    }
  }
</style>
