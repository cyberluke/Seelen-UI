import { Icon } from "libs/ui/react/components/Icon";
import type React from "react";

export enum RoutePath {
  Home = "/",
  Store = "/store",
  Agent = "/agent",
  Platform = "/platform",
  General = "/general",
  Resource = "/resources",
  Shortcuts = "/shortcuts",
  SettingsByMonitor = "/monitors",
  SettingsByApplication = "/specific_apps",
  DevTools = "/developer",
  Extras = "/extras",
}

export const RouteIcons: { [key in RoutePath]?: React.ReactNode } = {
  [RoutePath.Home]: <Icon iconName="TbHome" />,
  [RoutePath.Store]: <Icon iconName="PiShoppingBag" />,
  [RoutePath.Agent]: <Icon iconName="PiCpuBold" />,
  [RoutePath.Platform]: <Icon iconName="PiCodeBold" />,
  [RoutePath.General]: <Icon iconName="RiSettings3Fill" />,
  [RoutePath.Resource]: <Icon iconName="IoColorPalette" />,
  [RoutePath.SettingsByMonitor]: <Icon iconName="PiMonitorBold" />,
  [RoutePath.SettingsByApplication]: <Icon iconName="IoIosApps" />,
  [RoutePath.Shortcuts]: <Icon iconName="MdLaunch" />,
  [RoutePath.Extras]: <Icon iconName="PiInfoFill" />,
  [RoutePath.DevTools]: <Icon iconName="PiCodeBold" />,
};
