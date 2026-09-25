import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { lazyRune } from "libs/ui/svelte/utils";

/** Explicit, non-silent result shape for network-backed runic sources. */
interface Runic<T> {
  ok: boolean;
  value: T | null;
  error: string | null;
}

async function runic<T>(command: Parameters<typeof invoke>[0], arg?: unknown): Promise<Runic<T>> {
  try {
    const value = (await invoke(command as never, arg as never)) as T;
    return { ok: true, value: value ?? null, error: null };
  } catch (error) {
    return { ok: false, value: null, error: String(error) };
  }
}

const apps = lazyRune(() => invoke(SeelenCommand.NaiApps));
const catalog = lazyRune(() => invoke(SeelenCommand.NaiCatalog));
const activities = lazyRune(() => invoke(SeelenCommand.NaiActivities));
const capsules = lazyRune(() => invoke(SeelenCommand.NaiCapsules));
const media = lazyRune(() => invoke(SeelenCommand.GetMediaSessions));

const capabilities = lazyRune(() => runic(SeelenCommand.NaiCapabilities));
const gateway = lazyRune(() => runic(SeelenCommand.NaiGatewayModels));
const shortsQueue = lazyRune(() => runic(SeelenCommand.NaiShortsQueue));
const socialHome = lazyRune(() => runic(SeelenCommand.NaiSocialTimeline, { limit: 8 }));
const socialMentions = lazyRune(() => runic(SeelenCommand.NaiSocialNotifications, { limit: 8 }));

await Promise.all([
  apps.init(),
  catalog.init(),
  activities.init(),
  capsules.init(),
  media.init(),
  capabilities.init(),
  gateway.init(),
  shortsQueue.init(),
  socialHome.init(),
  socialMentions.init(),
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
  get capabilities() {
    return capabilities.value;
  }
  get gateway() {
    return gateway.value;
  }
  get shortsQueue() {
    return shortsQueue.value;
  }
  get socialHome() {
    return socialHome.value;
  }
  get socialMentions() {
    return socialMentions.value;
  }

  async refreshCatalog() {
    catalog.value = await invoke(SeelenCommand.NaiCatalog);
  }
  async refreshShorts() {
    shortsQueue.value = await runic(SeelenCommand.NaiShortsQueue);
  }
  async refreshSocial() {
    socialHome.value = await runic(SeelenCommand.NaiSocialTimeline, { limit: 8 });
    socialMentions.value = await runic(SeelenCommand.NaiSocialNotifications, { limit: 8 });
  }
}

export const naiState = new NaiState();
