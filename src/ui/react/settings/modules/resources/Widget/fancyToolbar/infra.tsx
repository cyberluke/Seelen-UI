import { FancyToolbarSide, HideMode, TrayOfflineMode, TrayPinnedPosition } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { $is_touch_primary } from "libs/ui/react/utils/signals";
import { Button, InputNumber, Select, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";

import { OptionsFromEnum } from "../../../shared/utils/app.ts";
import {
  getToolbarConfig,
  getTrayConfig,
  setToolbarDelayToHide,
  setToolbarDelayToShow,
  setToolbarHideMode,
  setToolbarItemSize,
  setToolbarMargin,
  setToolbarPadding,
  setToolbarPosition,
  setTrayDragReorder,
  setTrayIconSize,
  setTrayIconSpacing,
  setTrayOfflineMode,
  setTrayOverflowArrow,
  setTrayPinnedPosition,
  setTrayShowTooltips,
} from "./application.ts";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../../../components/SettingsBox/index.tsx";
import Compact from "antd/es/space/Compact";

export function FancyToolbarSettings() {
  const settings = getToolbarConfig();
  const delayToShow = settings.delayToShow;
  const delayToHide = settings.delayToHide;
  const isTouchPrimary = $is_touch_primary.value;

  const { t } = useTranslation();

  return (
    <>
      <SettingsGroup>
        <SettingsSubGroup label={t("toolbar.label")}>
          <SettingsOption
            label={t("toolbar.item_size")}
            action={
              <InputNumber
                value={settings.itemSize}
                onChange={(value) => setToolbarItemSize(value || 0)}
                min={4}
                max={100}
              />
            }
          />

          <SettingsOption
            label={t("toolbar.padding")}
            action={
              <InputNumber
                value={settings.padding}
                onChange={(value) => setToolbarPadding(value || 0)}
                min={0}
                max={40}
              />
            }
          />

          <SettingsOption
            label={t("toolbar.margin")}
            action={
              <InputNumber
                value={settings.margin}
                onChange={(value) => setToolbarMargin(value || 0)}
                min={0}
                max={40}
              />
            }
          />

          <SettingsOption
            label={t("toolbar.dock_side")}
            action={
              <Compact>
                {Object.values(FancyToolbarSide).map((side) => (
                  <Button
                    key={side}
                    type={side === settings.position ? "primary" : "default"}
                    onClick={() => setToolbarPosition(side)}
                  >
                    <Icon iconName={`CgToolbar${side}`} size={18} />
                  </Button>
                ))}
              </Compact>
            }
          />
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup
          label={
            <SettingsOption>
              <b>{t("toolbar.auto_hide")}</b>
              {/* disabled on touch devices: autohide requires hover/pointer events that touchscreens don't fire */}
              <Tooltip title={isTouchPrimary ? t("toolbar.auto_hide_touch_disabled") : undefined}>
                <Select
                  style={{ width: "120px" }}
                  value={settings.hideMode}
                  options={OptionsFromEnum(t, HideMode, "toolbar.hide_mode")}
                  onChange={(value) => setToolbarHideMode(value)}
                  disabled={isTouchPrimary}
                />
              </Tooltip>
            </SettingsOption>
          }
        >
          <SettingsOption>
            <span>{t("toolbar.delay_to_show")} (ms)</span>
            <InputNumber
              value={delayToShow}
              min={0}
              max={10000}
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => setToolbarDelayToShow(value || 0)}
            />
          </SettingsOption>
          <SettingsOption>
            <span>{t("toolbar.delay_to_hide")} (ms)</span>
            <InputNumber
              value={delayToHide}
              min={0}
              max={10000}
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => setToolbarDelayToHide(value || 0)}
            />
          </SettingsOption>
        </SettingsSubGroup>
      </SettingsGroup>

      <TraySettingsSection />
    </>
  );
}

function TraySettingsSection() {
  const { t } = useTranslation();
  const tray = getTrayConfig();

  return (
    <SettingsGroup>
      <SettingsSubGroup label={t("system_tray.label")}>
        <SettingsOption
          label={t("system_tray.pinned_position")}
          action={
            <Select
              style={{ width: "160px" }}
              value={tray.pinnedPosition}
              options={[
                {
                  value: TrayPinnedPosition.BeforeOverflow,
                  label: t("system_tray.pinned_position.before"),
                },
                {
                  value: TrayPinnedPosition.AfterOverflow,
                  label: t("system_tray.pinned_position.after"),
                },
              ]}
              onChange={(value) => setTrayPinnedPosition(value as TrayPinnedPosition)}
            />
          }
        />
        <SettingsOption
          label={t("system_tray.offline_pinned")}
          action={
            <Select
              style={{ width: "160px" }}
              value={tray.offlinePinned}
              options={[
                {
                  value: TrayOfflineMode.Hide,
                  label: t("system_tray.offline_pinned.hide"),
                },
                {
                  value: TrayOfflineMode.Disabled,
                  label: t("system_tray.offline_pinned.disabled"),
                },
              ]}
              onChange={(value) => setTrayOfflineMode(value as TrayOfflineMode)}
            />
          }
        />
        <SettingsOption
          label={t("system_tray.icon_spacing")}
          action={
            <InputNumber
              value={tray.iconSpacing ?? undefined}
              min={0}
              max={40}
              placeholder="-"
              onChange={(value) => setTrayIconSpacing(value)}
            />
          }
        />
        <SettingsOption
          label={t("system_tray.icon_size")}
          action={
            <InputNumber
              value={tray.iconSize ?? undefined}
              min={0}
              max={100}
              placeholder="-"
              onChange={(value) => setTrayIconSize(value)}
            />
          }
        />
        <SettingsOption
          label={t("system_tray.show_tooltips")}
          action={<Switch checked={tray.showTooltips} onChange={setTrayShowTooltips} />}
        />
        <SettingsOption
          label={t("system_tray.drag_reorder")}
          action={<Switch checked={tray.dragReorderEnabled} onChange={setTrayDragReorder} />}
        />
        <SettingsOption
          label={t("system_tray.overflow_arrow")}
          action={<Switch checked={tray.overflowArrowEnabled} onChange={setTrayOverflowArrow} />}
        />
      </SettingsSubGroup>
    </SettingsGroup>
  );
}
