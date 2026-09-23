export enum StartDisplayMode {
  Normal = "normal",
  Fullscreen = "fullscreen",
}

export enum StartView {
  All = "all",
  Favorites = "favorites",
  Home = "home",
  AiApps = "aiapps",
  Activities = "activities",
  Agents = "agents",
  Media = "media",
}

/** Left navigation of the NAI launcher (ADR/14 §4). The Home hero and the
 *  AI Apps store are distinct surfaces; Settings stays the existing widget. */
export const NAV_SECTIONS = [
  { id: StartView.Home, key: "nav.home" },
  { id: StartView.AiApps, key: "nav.ai_apps" },
  { id: StartView.Activities, key: "nav.activities" },
  { id: StartView.Agents, key: "nav.agents" },
  { id: StartView.Media, key: "nav.media" },
] as const;
