import { settings } from "../../../../state/mod";
import type { FancyToolbarSettings } from "@seelen-ui/lib/types";
import type { FancyToolbarSide, HideMode, TrayOfflineMode, TrayPinnedPosition } from "@seelen-ui/lib/types";

/**
 * Patches the FancyToolbar configuration with partial updates.
 * This helper function simplifies updating the toolbar settings by handling
 * the nested structure automatically.
 *
 * @example
 * patchToolbarConfig({ enabled: true, height: 40 });
 */
export function patchToolbarConfig(patch: Partial<FancyToolbarSettings>) {
  settings.value = {
    ...settings.value,
    byWidget: {
      ...settings.value.byWidget,
      "@seelen/fancy-toolbar": {
        ...settings.value.byWidget["@seelen/fancy-toolbar"],
        ...patch,
      },
    },
  };
}

/**
 * Gets the current FancyToolbar configuration
 */
export function getToolbarConfig(): FancyToolbarSettings {
  return settings.value.byWidget["@seelen/fancy-toolbar"];
}

/**
 * Sets the toolbar item size
 */
export function setToolbarItemSize(itemSize: number) {
  patchToolbarConfig({ itemSize });
}

/**
 * Sets the toolbar margin
 */
export function setToolbarMargin(margin: number) {
  patchToolbarConfig({ margin });
}

/**
 * Sets the toolbar padding
 */
export function setToolbarPadding(padding: number) {
  patchToolbarConfig({ padding });
}

/**
 * Sets the toolbar position
 */
export function setToolbarPosition(position: FancyToolbarSide) {
  patchToolbarConfig({ position });
}

/**
 * Sets the hide mode
 */
export function setToolbarHideMode(hideMode: HideMode) {
  patchToolbarConfig({ hideMode });
}

/**
 * Sets the delay to show
 */
export function setToolbarDelayToShow(delayToShow: number) {
  patchToolbarConfig({ delayToShow });
}

/**
 * Sets the delay to hide
 */
export function setToolbarDelayToHide(delayToHide: number) {
  patchToolbarConfig({ delayToHide });
}

/**
 * System-tray sub-config helpers
 */
export function getTrayConfig() {
  return getToolbarConfig().tray;
}

export function patchTrayConfig(
  patch: Partial<NonNullable<ReturnType<typeof getTrayConfig>>>,
) {
  const current = getToolbarConfig();
  patchToolbarConfig({
    tray: { ...(current.tray ?? {}), ...patch },
  });
}

export function setTrayPinnedPosition(pos: TrayPinnedPosition): void {
  patchTrayConfig({ pinnedPosition: pos });
}

export function setTrayOfflineMode(mode: TrayOfflineMode): void {
  patchTrayConfig({ offlinePinned: mode });
}

export function setTrayIconSpacing(value: number | null) {
  patchTrayConfig({ iconSpacing: value ?? undefined });
}

export function setTrayIconSize(value: number | null) {
  patchTrayConfig({ iconSize: value ?? undefined });
}

export function setTrayShowTooltips(enabled: boolean) {
  patchTrayConfig({ showTooltips: enabled });
}

export function setTrayDragReorder(enabled: boolean) {
  patchTrayConfig({ dragReorderEnabled: enabled });
}

export function setTrayOverflowArrow(enabled: boolean) {
  patchTrayConfig({ overflowArrowEnabled: enabled });
}
