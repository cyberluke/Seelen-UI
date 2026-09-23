<script lang="ts">
  import { globalState } from "../../state/mod.svelte";
  import { StartView } from "../../constants";
  import PinnedView from "../PinnedView.svelte";
  import AllAppsView from "../AllAppsView.svelte";
  import NaiViews from "../NaiViews.svelte";
</script>

<div class="apps-menu-body">
  {#if globalState.view === StartView.Favorites}
    <PinnedView />
  {:else if globalState.view === StartView.All}
    <AllAppsView />
  {:else}
    <NaiViews />
  {/if}
</div>
