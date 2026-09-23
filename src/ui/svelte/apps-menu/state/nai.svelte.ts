import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { lazyRune } from "libs/ui/svelte/utils";

const apps = lazyRune(() => invoke(SeelenCommand.NaiApps));
const catalog = lazyRune(() => invoke(SeelenCommand.NaiCatalog));
const activities = lazyRune(() => invoke(SeelenCommand.NaiActivities));
const capsules = lazyRune(() => invoke(SeelenCommand.NaiCapsules));
const media = lazyRune(() => invoke(SeelenCommand.GetMediaSessions));

await Promise.all([
  apps.init(),
  catalog.init(),
  activities.init(),
  capsules.init(),
  media.init(),
]);

class NaiState {
  get apps() {
    return apps.value;
  }
  get catalog() {
    return catalog.value;
  }
  get activities() {
    return activities.value;
  }
  get capsules() {
    return capsules.value;
  }
  get media() {
    return media.value;
  }
  async refreshCatalog() {
    catalog.value = await invoke(SeelenCommand.NaiCatalog);
  }
}

export const naiState = new NaiState();
