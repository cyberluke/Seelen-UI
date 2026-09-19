import { HideMode, SeelenWegMode, SeelenWegSide, WegMiddleClickAction } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { $is_touch_primary } from "libs/ui/react/utils/signals";
import { Button, InputNumber, message, Select, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";

import { OptionsFromEnum } from "../../../shared/utils/app.ts";
import { getWegConfig, importFromWindowsTaskbar, patchWegConfig } from "./application.ts";
import { getDevTools } from "../../../developer/application.ts";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../../../components/SettingsBox/index.tsx";
import Compact from "antd/es/space/Compact";

export const SeelenWegSettings = () => {
  const settings = getWegConfig();
  const isTouchPrimary = $is_touch_primary.value;
  const devTools = getDevTools();

  const { t } = useTranslation();

  const handleImportTaskbarItems = async () => {
    try {
      const count = await importFromWindowsTaskbar();
      if (count === 0) {
        message.info(t("weg.import_no_items"));
      } else {
        message.success(t("weg.import_success", { count }));
      }
    } catch (error) {
      message.error(t("weg.import_error"));
      console.error("Failed to import taskbar items:", error);
    }
  };

  return (
    <>
      <SettingsGroup>
        <SettingsSubGroup label={t("weg.label")}>
          <SettingsOption>
            <div>{t("weg.width")}</div>
            <Select
              style={{ width: "120px" }}
              value={settings.mode}
              options={OptionsFromEnum(t, SeelenWegMode, "weg.mode")}
              onChange={(value) => patchWegConfig({ mode: value })}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.dock_side")}</div>
            <Compact>
              {Object.values(SeelenWegSide).map((side) => (
                <Button
                  key={side}
                  type={side === settings.position ? "primary" : "default"}
                  onClick={() => patchWegConfig({ position: side })}
                >
                  <Icon iconName={`CgToolbar${side}`} size={18} />
                </Button>
              ))}
            </Compact>
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.margin")}</div>
            <InputNumber
              value={settings.margin}
              onChange={(value) => patchWegConfig({ margin: value || 0 })}
              min={0}
              max={40}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.padding")}</div>
            <InputNumber
              value={settings.padding}
              onChange={(value) => patchWegConfig({ padding: value || 0 })}
              min={0}
              max={40}
            />
          </SettingsOption>
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup
          label={
            <SettingsOption>
              <b>{t("weg.auto_hide")}</b>
              {/* disabled on touch devices: autohide requires hover/pointer events that touchscreens don't fire */}
              <Tooltip title={isTouchPrimary ? t("weg.auto_hide_touch_disabled") : undefined}>
                <Select
                  style={{ width: "120px" }}
                  value={settings.hideMode}
                  options={OptionsFromEnum(t, HideMode, "weg.hide_mode")}
                  onChange={(value) => patchWegConfig({ hideMode: value })}
                  disabled={isTouchPrimary}
                />
              </Tooltip>
            </SettingsOption>
          }
        >
          <SettingsOption>
            <span>{t("weg.delay_to_show")} (ms)</span>
            <InputNumber
              value={settings.delayToShow}
              min={0}
              max={10000}
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => patchWegConfig({ delayToShow: value || 0 })}
            />
          </SettingsOption>
          <SettingsOption>
            <span>{t("weg.delay_to_hide")} (ms)</span>
            <InputNumber
              value={settings.delayToHide}
              min={0}
              max={10000}
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => patchWegConfig({ delayToHide: value || 0 })}
            />
          </SettingsOption>
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("weg.filtering")}>
          <SettingsOption>
            <div>{t("weg.items.temporal_visibility.label")}</div>
            <Select
              style={{ width: "120px" }}
              value={settings.temporalItemsVisibility}
              options={[
                { value: "All", label: t("weg.items.temporal_visibility.all") },
                {
                  value: "OnMonitor",
                  label: t("weg.items.temporal_visibility.on_monitor"),
                },
              ]}
              onChange={(value) => patchWegConfig({ temporalItemsVisibility: value })}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.items.pinned_visibility.label")}</div>
            <Select
              style={{ width: "120px" }}
              value={settings.pinnedItemsVisibility}
              options={[
                {
                  value: "Always",
                  label: t("weg.items.pinned_visibility.always"),
                },
                {
                  value: "WhenPrimary",
                  label: t("weg.items.pinned_visibility.when_primary"),
                },
              ]}
              onChange={(value) => patchWegConfig({ pinnedItemsVisibility: value })}
            />
          </SettingsOption>
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("weg.items.label")}>
          <SettingsOption>
            <div>{t("weg.items.size")}</div>
            <InputNumber
              value={settings.size}
              onChange={(value) => patchWegConfig({ size: value || 0 })}
              min={16}
              max={128}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.items.gap")}</div>
            <InputNumber
              value={settings.spaceBetweenItems}
              onChange={(value) => patchWegConfig({ spaceBetweenItems: value || 0 })}
              min={0}
              max={40}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.items.show_window_title")}</div>
            <Switch
              checked={settings.showWindowTitle}
              onChange={(value) => patchWegConfig({ showWindowTitle: value })}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.items.show_instance_counter")}</div>
            <Switch
              checked={settings.showInstanceCounter}
              onChange={(value) => patchWegConfig({ showInstanceCounter: value })}
            />
          </SettingsOption>
          <SettingsOption>
            <div>{t("weg.items.split_windows")}</div>
            <Switch
              checked={settings.splitWindows}
              onChange={(value) => patchWegConfig({ splitWindows: value })}
            />
          </SettingsOption>
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption
          label={t("weg.items.middle_click_action.label")}
          action={
            <Select
              style={{ width: "160px" }}
              value={settings.middleClickAction}
              options={[
                {
                  value: WegMiddleClickAction.CloseApp,
                  label: t("weg.items.middle_click_action.close_app"),
                },
                {
                  value: WegMiddleClickAction.OpenNewInstance,
                  label: t("weg.items.middle_click_action.open_new_instance"),
                },
              ]}
              onChange={(value) => patchWegConfig({ middleClickAction: value })}
            />
          }
        />
      </SettingsGroup>

      {devTools && (
        <SettingsGroup>
          <SettingsOption>
            <b>{t("weg.show_end_task")}</b>
            <Switch
              checked={settings.showEndTask}
              onChange={(value) => patchWegConfig({ showEndTask: value })}
            />
          </SettingsOption>
        </SettingsGroup>
      )}

      <SettingsGroup>
        <SettingsOption>
          <div>{t("weg.import_from_taskbar.label")}</div>
          <Button onClick={handleImportTaskbarItems}>
            {t("weg.import_from_taskbar.button")}
          </Button>
        </SettingsOption>
      </SettingsGroup>

      <WindowPreviewSection settings={settings} />
      <AutomationSection settings={settings} />
    </>
  );
};

type AnySettings = Record<string, any>;

const WindowPreviewSection = ({ settings }: { settings: AnySettings }) => {
  const { t } = useTranslation();
  const preview: AnySettings = settings.preview ?? {};

  const patch = (value: Partial<AnySettings>) => patchWegConfig({ preview: { ...preview, ...value } } as any);

  return (
    <SettingsGroup>
      <SettingsSubGroup label={t("weg.preview.label")}>
        <SettingsOption>
          <div>{t("weg.preview.trigger")}</div>
          <Select
            style={{ width: "140px" }}
            value={preview.trigger ?? "Hover"}
            options={[
              { value: "Hover", label: t("weg.preview.trigger_hover") },
              { value: "Click", label: t("weg.preview.trigger_click") },
              { value: "HoverAndClick", label: t("weg.preview.trigger_hover_and_click") },
            ]}
            onChange={(value) => patch({ trigger: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.card_width")}</div>
          <InputNumber
            value={preview.cardWidth}
            min={0}
            max={1024}
            onChange={(value) => patch({ cardWidth: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.card_height")}</div>
          <InputNumber
            value={preview.cardHeight}
            min={0}
            max={1024}
            onChange={(value) => patch({ cardHeight: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.aspect_mode")}</div>
          <Select
            style={{ width: "140px" }}
            value={preview.aspectRatioMode ?? "SourceWindow"}
            options={[
              { value: "SourceWindow", label: t("weg.preview.aspect_source") },
              { value: "Fixed", label: t("weg.preview.aspect_fixed") },
            ]}
            onChange={(value) => patch({ aspectRatioMode: value })}
          />
        </SettingsOption>
        {preview.aspectRatioMode === "Fixed" && (
          <SettingsOption>
            <div>{t("weg.preview.aspect_ratio")}</div>
            <InputNumber
              value={preview.aspectRatioNum}
              min={1}
              max={64}
              onChange={(value) => patch({ aspectRatioNum: value || 16 })}
            />
          </SettingsOption>
        )}
        <SettingsOption>
          <div>{t("weg.preview.auto_grid")}</div>
          <Switch
            checked={preview.autoGrid ?? true}
            onChange={(value) => patch({ autoGrid: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.columns")}</div>
          <InputNumber
            value={preview.columns}
            min={1}
            max={12}
            disabled={preview.autoGrid ?? true}
            onChange={(value) => patch({ columns: value || 1 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.min_columns")}</div>
          <InputNumber
            value={preview.minColumns}
            min={1}
            max={12}
            onChange={(value) => patch({ minColumns: value || 1 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.max_columns")}</div>
          <InputNumber
            value={preview.maxColumns}
            min={1}
            max={12}
            onChange={(value) => patch({ maxColumns: value || 1 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.max_rows")}</div>
          <InputNumber
            value={preview.maxRows}
            min={1}
            max={8}
            onChange={(value) => patch({ maxRows: value || 1 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.max_width")}</div>
          <InputNumber
            value={preview.maxPopupWidth}
            min={160}
            max={2560}
            onChange={(value) => patch({ maxPopupWidth: value || 256 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.max_height")}</div>
          <InputNumber
            value={preview.maxPopupHeight}
            min={120}
            max={1440}
            onChange={(value) => patch({ maxPopupHeight: value || 256 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.gap")}</div>
          <InputNumber
            value={preview.gap}
            min={0}
            max={64}
            onChange={(value) => patch({ gap: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.padding")}</div>
          <InputNumber
            value={preview.padding}
            min={0}
            max={64}
            onChange={(value) => patch({ padding: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.radius")}</div>
          <InputNumber
            value={preview.borderRadius}
            min={0}
            max={48}
            onChange={(value) => patch({ borderRadius: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.animated")}</div>
          <Switch
            checked={preview.animated ?? true}
            onChange={(value) => patch({ animated: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.animation_duration")}</div>
          <InputNumber
            value={preview.animationDuration}
            min={0}
            max={1000}
            onChange={(value) => patch({ animationDuration: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.titles")}</div>
          <Select
            style={{ width: "140px" }}
            value={preview.showTitles ? (preview.titleLines ?? "OneLine") : "Hidden"}
            options={[
              { value: "Hidden", label: t("weg.preview.titles_hidden") },
              { value: "OneLine", label: t("weg.preview.titles_one_line") },
              { value: "TwoLines", label: t("weg.preview.titles_two_lines") },
            ]}
            onChange={(value) =>
              patch(
                value === "Hidden" ? { showTitles: false } : { showTitles: true, titleLines: value },
              )}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.compact_titles")}</div>
          <Switch
            checked={preview.compactTitles ?? true}
            disabled={!preview.showTitles}
            onChange={(value) => patch({ compactTitles: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.app_aware_titles")}</div>
          <Switch
            checked={preview.applicationAwareTitles ?? true}
            disabled={!preview.showTitles}
            onChange={(value) => patch({ applicationAwareTitles: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.ordering")}</div>
          <Select
            style={{ width: "160px" }}
            value={preview.ordering ?? "Manual"}
            options={[
              { value: "Manual", label: t("weg.preview.ordering_manual") },
              { value: "Stable", label: t("weg.preview.ordering_stable") },
              { value: "Mru", label: t("weg.preview.ordering_mru") },
              { value: "Alphabetical", label: t("weg.preview.ordering_alphabetical") },
              {
                value: "ApplicationDefined",
                label: t("weg.preview.ordering_application_defined"),
              },
              { value: "Hybrid", label: t("weg.preview.ordering_hybrid") },
            ]}
            onChange={(value) => patch({ ordering: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.grouped_click_label")}</div>
          <Select
            style={{ width: "160px" }}
            value={preview.groupedClickAction ?? "OpenPreview"}
            options={[
              { value: "OpenPreview", label: t("weg.preview.grouped_click_preview") },
              { value: "ActivateLastUsed", label: t("weg.preview.grouped_click_last_used") },
              {
                value: "MinimizeRestoreGroup",
                label: t("weg.preview.grouped_click_minimize_restore"),
              },
            ]}
            onChange={(value) => patch({ groupedClickAction: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.hover_open_delay")}</div>
          <InputNumber
            value={preview.hoverOpenDelay}
            min={0}
            max={5000}
            onChange={(value) => patch({ hoverOpenDelay: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.hover_close_delay")}</div>
          <InputNumber
            value={preview.hoverCloseDelay}
            min={0}
            max={5000}
            onChange={(value) => patch({ hoverCloseDelay: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.grace_region")}</div>
          <InputNumber
            value={preview.pointerGraceRegion}
            min={0}
            max={200}
            onChange={(value) => patch({ pointerGraceRegion: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.keep_open")}</div>
          <Switch
            checked={preview.keepOpenOnTraversal ?? true}
            onChange={(value) => patch({ keepOpenOnTraversal: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.cache_enabled")}</div>
          <Switch
            checked={preview.cacheEnabled ?? true}
            onChange={(value) => patch({ cacheEnabled: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.prepared_entries")}</div>
          <InputNumber
            value={preview.cacheMemoryBudget}
            min={2}
            max={512}
            onChange={(value) => patch({ cacheMemoryBudget: value || 2 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.stale_while_refresh")}</div>
          <Switch
            checked={preview.staleWhileRefresh ?? true}
            onChange={(value) => patch({ staleWhileRefresh: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.prewarm")}</div>
          <Switch
            checked={preview.prewarmView ?? true}
            onChange={(value) => patch({ prewarmView: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.preview.nearest_first")}</div>
          <Switch
            checked={preview.nearestFirstProjection ?? false}
            onChange={(value) => patch({ nearestFirstProjection: value })}
          />
        </SettingsOption>
      </SettingsSubGroup>
    </SettingsGroup>
  );
};

const AutomationSection = ({ settings }: { settings: AnySettings }) => {
  const { t } = useTranslation();
  const automation: AnySettings = settings.automation ?? {};

  const patch = (value: Partial<AnySettings>) => patchWegConfig({ automation: { ...automation, ...value } } as any);

  return (
    <SettingsGroup>
      <SettingsSubGroup label={t("weg.automation.label")}>
        <SettingsOption>
          <div>{t("weg.automation.enabled")}</div>
          <Switch
            checked={automation.enabled ?? true}
            onChange={(value) => patch({ enabled: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.automation.rest_enabled")}</div>
          <Switch
            checked={automation.rest_enabled ?? true}
            onChange={(value) => patch({ rest_enabled: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.automation.rest_port")}</div>
          <InputNumber
            value={automation.rest_port}
            min={0}
            max={65535}
            onChange={(value) => patch({ rest_port: value || 0 })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.automation.mcp_enabled")}</div>
          <Switch
            checked={automation.mcp_enabled ?? true}
            onChange={(value) => patch({ mcp_enabled: value })}
          />
        </SettingsOption>
        <SettingsOption>
          <div>{t("weg.automation.flight_recorder")}</div>
          <Switch
            checked={automation.flight_recorder ?? true}
            onChange={(value) => patch({ flight_recorder: value })}
          />
        </SettingsOption>
      </SettingsSubGroup>
    </SettingsGroup>
  );
};
