<script lang="ts">
  import { t } from "../i18n";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { StartView } from "../constants";
  import { globalState } from "../state/mod.svelte";
  import { naiState } from "../state/nai.svelte";

  // ── shared helpers ────────────────────────────────────────────────────────
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
    (naiState.catalog as { entries?: CatalogEntry[] } | null)?.entries ?? [],
  );

  interface CatalogEntry {
    id: string;
    name: string;
    description: string;
    distributionType?: string;
    nonAutomated?: boolean;
  }

  const statuses = $state<Record<string, string>>({});

  $effect(() => {
    const list = (naiState.catalog as { entries?: { id: string }[] } | null)?.entries ?? [];
    if (!list.length || Object.keys(statuses).length) return;
    (async () => {
      for (const entry of list) {
        const res = (await invoke(SeelenCommand.NaiAppStatus, { id: entry.id })) as unknown as {
          state: string;
        };
        statuses[entry.id] = res.state;
      }
    })();
  });

  // ── media / shorts ────────────────────────────────────────────────────────
  let shortsQuery = $state("");
  const shorts = $derived(naiState.shortsQueue as MediaSource | null);
  const pip = $derived(naiState.gateway as MediaSource | null);

  interface MediaSource {
    ok: boolean;
    value: unknown;
    error: string | null;
  }

  const queueItems = $derived(
    ((shorts?.value as { items?: { videoId: string; reason: string }[] } | null)?.items) ?? [],
  );

  async function searchShorts() {
    if (!shortsQuery.trim()) return;
    await invoke(SeelenCommand.NaiShortsSearch, { query: shortsQuery, limit: 5 });
  }

  async function nextShort() {
    const res = (await invoke(SeelenCommand.NaiShortsNext)) as unknown as {
      next: { videoId: string; reason: string } | null;
    };
    if (res.next) {
      await invoke(SeelenCommand.OpenFile, {
        path: `https://www.youtube.com/watch?v=${res.next.videoId}`,
      });
    }
    await naiState.refreshShorts();
  }

  // ── social capsule ────────────────────────────────────────────────────────
  let composeText = $state("");
  const social = $derived(naiState.socialHome as MediaSource | null);
  const mentions = $derived(naiState.socialMentions as MediaSource | null);
  const feed = $derived((social?.value as { id: string; title: string; account: string }[] | null) ?? []);
  const mentionList = $derived(
    (mentions?.value as { id: string; type: string; from: string }[] | null) ?? [],
  );

  async function compose() {
    if (!composeText.trim()) return;
    await invoke(SeelenCommand.NaiSocialCompose, { status: composeText });
    composeText = "";
    await naiState.refreshSocial();
  }

  // ── agents ────────────────────────────────────────────────────────────────
  const caps = $derived(naiState.capabilities as MediaSource | null);
  const capabilityList = $derived(
    (caps?.value as { id: string; risk: string; latencyClass: string }[] | null) ?? [],
  );
  const gatewayInfo = $derived(naiState.gateway as MediaSource | null);
  const gatewayModels = $derived(
    ((gatewayInfo?.value as { models?: { id: string; backend: string; capabilities: string[] }[] } | null)
      ?.models) ?? [],
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
            <span class="nai-hero-state" data-state={app.runningPids?.length ? "active" : "idle"}>
              {app.path ? (app.runningPids?.length ? "●" : "○") : "—"}
            </span>
          </button>
        </li>
      {:else}
        <li class="nai-hero-desc">{$t("no_matching_items")}</li>
      {/each}
    </ul>
  </div>

  <div class="nai-social">
    <h3 class="nai-section-title">club.v271.cz</h3>
    {#if social && !social.ok}
      <p class="nai-error">{social.error}</p>
    {:else if feed.length}
      <ul class="nai-feed">
        {#each feed as post (post.id)}
          <li class="nai-feed-item">
            <span class="nai-feed-account">{post.account}</span>
            <span class="nai-feed-body">{post.title}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="nai-hero-desc">{$t("no_matching_items")}</p>
    {/if}
    {#if mentionList.length}
      <h4 class="nai-sub-title">{$t("nav.agents")}</h4>
      <ul class="nai-feed">
        {#each mentionList as mention (mention.id)}
          <li class="nai-feed-item">
            <span class="nai-feed-account">{mention.from}</span>
            <span class="nai-feed-body">{mention.type}</span>
          </li>
        {/each}
      </ul>
    {/if}
    <div class="nai-inline">
      <input
        data-skin="default"
        bind:value={composeText}
                placeholder={$t("store.install")}
      />
      <button data-skin="solid" onclick={compose}>↵</button>
    </div>
  </div>
{:else if globalState.view === StartView.AiApps}
  <div class="nai-store">
    {#each entries as entry (entry.id)}
      <div class="nai-store-row">
        <div class="nai-store-meta">
          <span class="nai-store-name">{entry.name}</span>
          <span class="nai-store-desc">{entry.description}</span>
          <span class="nai-store-state" data-state={statuses[entry.id] ?? "Unknown"}>
            {statuses[entry.id] ?? "Unknown"}
          </span>
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
    {:else}
      <p class="nai-hero-desc">{$t("no_matching_items")}</p>
    {/each}
  </div>
{:else if globalState.view === StartView.Activities}
  <div class="nai-activities">
    {#each (naiState.activities as any[] | null) ?? [] as activity (activity.id)}
      <div class="nai-activity">
        <span class="nai-activity-name">{activity.name}</span>
        <span class="nai-activity-count">{activity.objects.length}</span>
      </div>
    {:else}
      <p class="nai-hero-desc">{$t("no_matching_items")}</p>
    {/each}
    {#each (naiState.capsules as any[] | null) ?? [] as capsule (capsule.id)}
      <div class="nai-capsule">
        <span class="nai-capsule-name">{capsule.name}</span>
        <span class="nai-capsule-activity">{capsule.activity}</span>
      </div>
    {/each}
  </div>
{:else if globalState.view === StartView.Agents}
  <div class="nai-columns">
    <section>
      <h3 class="nai-section-title">{$t("nav.agents")}</h3>
      {#each ((naiState.apps as any[] | null) ?? []).filter((a) => a.runningPids?.length) as agent (agent.id)}
        <div class="nai-activity">
          <span class="nai-activity-name">{agent.name}</span>
          <span class="nai-activity-count">{agent.runningPids.join(", ")}</span>
        </div>
      {:else}
        <p class="nai-hero-desc">{$t("no_matching_items")}</p>
      {/each}
    </section>
    <section>
      <h3 class="nai-section-title">{$t("nai.gateway")}</h3>
      {#if gatewayInfo && !gatewayInfo.ok}
        <p class="nai-error">{gatewayInfo.error}</p>
      {:else}
        {#each gatewayModels as model (model.id)}
          <div class="nai-activity">
            <span class="nai-activity-name">{model.id}</span>
            <span class="nai-activity-count">{model.backend} · {model.capabilities.join(", ")}</span>
          </div>
        {/each}
      {/if}
      <h3 class="nai-section-title">{$t("nai.capabilities")}</h3>
      {#if caps && !caps.ok}
        <p class="nai-error">{caps.error}</p>
      {:else}
        {#each capabilityList as cap (cap.id)}
          <div class="nai-activity">
            <span class="nai-activity-name">{cap.id}</span>
            <span class="nai-activity-count">{cap.risk} · {cap.latencyClass}</span>
          </div>
        {/each}
      {/if}
    </section>
  </div>
{:else if globalState.view === StartView.Media}
  <div class="nai-columns">
    <section>
      <h3 class="nai-section-title">{$t("nav.media")}</h3>
      {#each (naiState.media as any[] | null) ?? [] as session (session.id)}
        <div class="nai-media-item">
          <span>{session.title}</span>
          <span class="nai-hero-desc">{session.author}</span>
        </div>
      {:else}
        <p class="nai-hero-desc">{$t("no_matching_items")}</p>
      {/each}
    </section>
    <section>
      <h3 class="nai-section-title">Shorts</h3>
      <div class="nai-inline">
        <input
          data-skin="default"
          bind:value={shortsQuery}
          placeholder={$t("query.web")}
          onkeydown={(e) => e.key === "Enter" && searchShorts()}
        />
        <button data-skin="solid" onclick={searchShorts}>↵</button>
      </div>
      {#if shorts && !shorts.ok}
        <p class="nai-error">{shorts.error}</p>
      {:else if queueItems.length}
        <ol class="nai-feed">
          {#each queueItems as item, i (item.videoId + i)}
            <li class="nai-feed-item">
              <span class="nai-feed-account">{item.videoId}</span>
              <span class="nai-feed-body">{item.reason}</span>
            </li>
          {/each}
        </ol>
        <button data-skin="solid" onclick={nextShort}>{$t("store.launch")}</button>
      {:else}
        <p class="nai-hero-desc">{$t("no_matching_items")}</p>
      {/if}
    </section>
  </div>
{/if}

<style>
  .nai-hero-title,
  .nai-section-title,
  .nai-sub-title {
    font-size: 0.9em;
    color: var(--nai-lavender-core, #8d7cff);
    padding: 4px 0;
  }
  .nai-sub-title {
    font-size: 0.85em;
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
  .nai-feed {
    display: grid;
    gap: 4px;
  }
  .nai-activity,
  .nai-capsule,
  .nai-media-item,
  .nai-feed-item {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
  .nai-columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 12px;
  }
  .nai-inline {
    display: flex;
    gap: 4px;
  }
  .nai-error {
    color: var(--nai-hot-coral, #ff5d8f);
    font-size: 0.8em;
  }
  @media (prefers-reduced-motion: reduce) {
    * {
      animation: none;
      transition: none;
    }
  }
</style>
