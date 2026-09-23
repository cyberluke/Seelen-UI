<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { settingsState } from "../state/settings.svelte.ts";

  // Path is baked at build time (debug or release, whichever built last)
  // and resolved by the background via `get_runtime_exe_path`.
  async function launchSeelenUi(): Promise<void> {
    const path = await invoke(SeelenCommand.GetRuntimeExePath);
    await invoke(SeelenCommand.OpenFile, { path });
  }
</script>

<div
  class="ft-bar-tray-overflow"
  id="@seelen/tb-seelen-launcher"
  role="button"
  tabindex="0"
  data-plugin-id="@seelen/tb-seelen-launcher"
  data-tooltip="NAI OS"
  data-tooltip-align-x="Center"
  data-tooltip-align-y={settingsState.tooltipAlignY}
  data-tooltip-origin-y={settingsState.tooltipOriginY}
  onclick={launchSeelenUi}
  onkeypress={launchSeelenUi}
>
  <svg class="ft-bar-tray-overflow-icon" viewBox="0 0 256 256" xmlns="http://www.w3.org/2000/svg">
    <rect width="100%" height="100%" rx="20%" ry="20%" fill="#0f0f0f" />
    <linearGradient id="seelen-launcher-screen" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#f75c46" />
      <stop offset="50%" stop-color="#9d57f4" />
      <stop offset="100%" stop-color="#0054b6" />
    </linearGradient>
    <rect x="5%" y="5%" width="90%" height="90%" rx="16%" ry="16%" fill="url(#seelen-launcher-screen)" />
    <line
      stroke-width="2.5%"
      x1="49%"
      y1="50%"
      x2="95%"
      y2="50%"
      stroke="#ffffff"
      stroke-dasharray="6.5% 4%"
    />
    <line
      stroke-width="2.5%"
      x1="50%"
      y1="5%"
      x2="50%"
      y2="95%"
      stroke="#ffffff"
      stroke-dasharray="6.5% 3.95%"
    />
  </svg>
</div>

<style>
  svg {
    width: calc(var(--config-item-size) * 0.8);
    height: calc(var(--config-item-size) * 0.8);
  }
</style>
