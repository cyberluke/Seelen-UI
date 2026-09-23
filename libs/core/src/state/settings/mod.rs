/* In this file we use #[serde_alias(SnakeCase)] as backward compatibility from versions below v1.9.8 */
pub mod by_monitor;
pub mod by_theme;
pub mod by_wallpaper;
pub mod by_widget;
pub mod settings_by_app;
pub mod shortcuts;

pub use settings_by_app::*;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_alias::serde_alias;

use crate::resource::WidgetId;
use crate::state::WallpaperCollection;
use crate::system_state::MonitorId;
use crate::{
    error::Result,
    rect::Rect,
    resource::{IconPackId, PluginId, ThemeId, WallpaperId},
    state::{
        by_monitor::MonitorConfiguration, by_theme::ThemeSettings,
        by_wallpaper::WallpaperInstanceSettings, by_widget::SettingsByWidget,
        shortcuts::SluShortcutsSettings,
    },
};

// ============== Fancy Toolbar Settings ==============

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct FancyToolbarSettings {
    /// enable or disable the fancy toolbar
    pub enabled: bool,
    /// Key overrides for widget-declared shortcuts (`shortcut_id -> keys`).
    #[serde(rename = "$shortcuts")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    /// item size in px
    pub item_size: u32,
    /// Toolbar margin in px
    pub margin: u32,
    /// Toolbar padding in px
    pub padding: u32,
    /// position of the toolbar
    pub position: FancyToolbarSide,
    /// hide mode
    pub hide_mode: HideMode,
    /// delay to show the toolbar on Mouse Hover in milliseconds
    pub delay_to_show: u32,
    /// delay to hide the toolbar on Mouse Leave in milliseconds
    pub delay_to_hide: u32,
    /// System tray cluster configuration (pinned icons + overflow arrow).
    #[serde(default)]
    pub tray: TrayToolbarSettings,
}

/// Where the pinned tray cluster sits *inside* the tray slot.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum TrayPinnedPosition {
    /// Pinned icons appear before the overflow arrow (default).
    #[default]
    BeforeOverflow,
    /// Pinned icons appear after the overflow arrow.
    AfterOverflow,
}

/// Behavior for a pinned icon whose application is not currently running.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum TrayOfflineMode {
    /// Hide the offline icon (default).
    #[default]
    Hide,
    /// Show a disabled placeholder.
    Disabled,
}

/// Typed configuration for the pinned + overflow tray cluster.
#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct TrayToolbarSettings {
    /// Where the pinned cluster sits relative to the overflow arrow.
    pub pinned_position: TrayPinnedPosition,
    /// Behavior for pinned icons whose application is offline.
    pub offline_pinned: TrayOfflineMode,
    /// Explicit spacing between cluster icons (px). `None` inherits the
    /// default from the parent toolbar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_spacing: Option<u32>,
    /// Explicit size for pinned icons (px). `None` inherits `itemSize`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_size: Option<u32>,
    /// Show the tooltip for each pinned icon.
    pub show_tooltips: bool,
    /// Enable drag reorder of pinned icons.
    pub drag_reorder_enabled: bool,
    /// Render the overflow arrow (opens `@seelen/system-tray`).
    pub overflow_arrow_enabled: bool,
    /// Ordered tray logical identity keys that are currently pinned.
    /// This is the single authoritative pin store.
    pub pinned: Vec<String>,
}

impl Default for TrayToolbarSettings {
    fn default() -> Self {
        Self {
            pinned_position: TrayPinnedPosition::BeforeOverflow,
            offline_pinned: TrayOfflineMode::Hide,
            icon_spacing: Some(6),
            icon_size: Some(32),
            show_tooltips: true,
            drag_reorder_enabled: true,
            overflow_arrow_enabled: true,
            pinned: Vec::new(),
        }
    }
}

impl Default for FancyToolbarSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: None,
            item_size: 16,
            padding: 8,
            margin: 0,
            position: FancyToolbarSide::Top,
            hide_mode: HideMode::Never,
            delay_to_show: 100,
            delay_to_hide: 800,
            tray: TrayToolbarSettings::default(),
        }
    }
}

impl FancyToolbarSettings {
    /// total height of the toolbar
    pub fn total_size(&self) -> u32 {
        self.item_size + (self.padding * 2) + (self.margin * 2)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum FancyToolbarSide {
    Top,
    Bottom,
}

// ============== SeelenWeg Settings ==============

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum SeelenWegMode {
    #[serde(alias = "Full-Width")]
    FullWidth,
    #[serde(alias = "Min-Content")]
    MinContent,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WegTemporalItemsVisibility {
    All,
    OnMonitor,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WegPinnedItemsVisibility {
    Always,
    WhenPrimary,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum HideMode {
    /// never hide
    Never,
    /// auto-hide always on
    Always,
    /// auto-hide only if is overlaped by the focused window
    #[serde(alias = "On-Overlap")]
    OnOverlap,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WegMiddleClickAction {
    /// Close the focused window of the app (default)
    CloseApp,
    /// Open a new instance of the app
    OpenNewInstance,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum SeelenWegSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct SeelenWegSettings {
    /// enable or disable the seelenweg
    pub enabled: bool,
    /// Key overrides for widget-declared shortcuts (`shortcut_id -> keys`).
    #[serde(rename = "$shortcuts")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    /// Dock/Taskbar mode
    pub mode: SeelenWegMode,
    /// When to hide the dock
    pub hide_mode: HideMode,
    /// Split windows into separated items instead of grouped.
    pub split_windows: bool,
    /// Which temporal items to show on the dock instance (this can be overridden per monitor)
    pub temporal_items_visibility: WegTemporalItemsVisibility,
    /// Determines is the pinned item should be shown or not (this can be overridden per monitor).
    pub pinned_items_visibility: WegPinnedItemsVisibility,
    /// Dock position
    pub position: SeelenWegSide,
    /// enable or disable the instance counter visibility on weg instance
    pub show_instance_counter: bool,
    /// enable or disable the window title visibility for opened apps
    pub show_window_title: bool,
    /// item size in px
    pub size: u32,
    /// Dock/Taskbar margin in px
    pub margin: u32,
    /// Dock/Taskbar padding in px
    pub padding: u32,
    /// space between items in px
    pub space_between_items: u32,
    /// delay to show the toolbar on Mouse Hover in milliseconds
    pub delay_to_show: u32,
    /// delay to hide the toolbar on Mouse Leave in milliseconds
    pub delay_to_hide: u32,
    /// show end task button on context menu (needs developer mode enabled)
    pub show_end_task: bool,
    /// Action to perform when middle-clicking a dock item
    pub middle_click_action: WegMiddleClickAction,
    /// window preview popup configuration
    pub preview: WindowPreviewSettings,
    /// programmatic control planes (MCP / REST / CLI)
    pub automation: AutomationSettings,
}

impl Default for SeelenWegSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: None,
            mode: SeelenWegMode::MinContent,
            hide_mode: HideMode::OnOverlap,
            position: SeelenWegSide::Bottom,
            show_instance_counter: true,
            show_window_title: false,
            temporal_items_visibility: WegTemporalItemsVisibility::All,
            pinned_items_visibility: WegPinnedItemsVisibility::Always,
            size: 40,
            margin: 8,
            padding: 8,
            space_between_items: 8,
            delay_to_show: 100,
            delay_to_hide: 800,
            show_end_task: false,
            split_windows: false,
            middle_click_action: WegMiddleClickAction::OpenNewInstance,
            preview: WindowPreviewSettings::default(),
            automation: AutomationSettings::default(),
        }
    }
}

impl SeelenWegSettings {
    /// total height or width of the dock, depending on the Position
    pub fn total_size(&self) -> u32 {
        self.size + (self.padding * 2) + (self.margin * 2)
    }
}

// ============== Window Preview Settings ==============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WindowPreviewTrigger {
    /// popup opens when the pointer enters the dock item
    Hover,
    /// popup opens only on click
    Click,
    /// popup opens on hover or click
    HoverAndClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum PreviewAspectRatioMode {
    /// cards use the real source window ratio
    SourceWindow,
    /// cards use the configured fixed ratio
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum PreviewTitleLines {
    OneLine,
    TwoLines,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum PreviewOrderingStrategy {
    /// user-defined manual order (drag), never reshuffled by focus/MRU
    Manual,
    /// stable creation order
    Stable,
    /// most-recently-used order
    Mru,
    /// alphabetical by display title
    Alphabetical,
    /// order provided/declared by the application itself (creation order fallback)
    ApplicationDefined,
    /// manual first, unknown windows appended by mru
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum PreviewGroupedClickAction {
    /// keep/toggle the preview popup
    OpenPreview,
    /// activate the last used window of the group
    ActivateLastUsed,
    /// minimize (if focused) / restore the group
    MinimizeRestoreGroup,
}

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct WindowPreviewSettings {
    /// which input opens the preview popup
    pub trigger: WindowPreviewTrigger,
    /// preferred card width in px (0 = derive from popup max width and columns)
    pub card_width: u32,
    /// forced card height in px; 0 keeps source ratio
    pub card_height: u32,
    /// how the card aspect ratio is resolved
    pub aspect_ratio_mode: PreviewAspectRatioMode,
    /// fixed ratio numerator (used when aspectRatioMode = Fixed)
    pub aspect_ratio_num: u32,
    /// fixed ratio denominator (used when aspectRatioMode = Fixed)
    pub aspect_ratio_den: u32,
    /// auto-compute columns from available space
    pub auto_grid: bool,
    /// preferred number of columns when auto grid computes within bounds
    #[serde(alias = "columns")]
    pub preferred_columns: u32,
    /// minimum columns clamp
    pub min_columns: u32,
    /// maximum columns clamp
    pub max_columns: u32,
    /// maximum rows before the list scrolls
    pub max_rows: u32,
    /// popup maximum width in px
    #[serde(alias = "maxPopupWidth")]
    pub max_width: u32,
    /// popup maximum height in px
    #[serde(alias = "maxPopupHeight")]
    pub max_height: u32,
    /// gap between cards in px
    pub gap: u32,
    /// popup padding in px
    pub padding: u32,
    /// card border radius in px
    pub border_radius: u32,
    /// animate popup open/close
    pub animated: bool,
    /// popup animation duration in ms
    pub animation_duration: u32,
    /// show window titles on cards
    pub show_titles: bool,
    /// how many lines a title may use
    pub title_lines: PreviewTitleLines,
    /// compact title mode (strip redundant app suffixes)
    pub compact_titles: bool,
    /// full title tooltip (requires showTitles)
    pub title_tooltip: bool,
    /// application-aware title normalization (vscode/edge/terminal parsers)
    pub application_aware_titles: bool,
    /// how cards are ordered inside a group popup
    pub ordering: PreviewOrderingStrategy,
    /// open delay after pointer enter, in ms
    pub hover_open_delay: u32,
    /// close delay after pointer leave, in ms (grace for traversal)
    pub hover_close_delay: u32,
    /// extra geometric grace beyond the popup rect, in px
    pub pointer_grace_region: u32,
    /// keep popup open while the pointer travels icon -> gap -> popup
    pub keep_open_on_traversal: bool,
    /// enable the layered preview cache
    pub cache_enabled: bool,
    /// max thumbnails kept per cache tier
    pub thumbnail_cache_size: u32,
    /// cache memory budget in KiB
    pub cache_memory_budget: u32,
    /// show stale thumbnails immediately and refresh asynchronously
    pub stale_while_refresh: bool,
    /// keep the grouped view/model prewarmed for running groups
    pub prewarm_view: bool,
    /// visual projection: nearest window to the anchor goes first visually
    pub nearest_first_projection: bool,
    /// grouped-icon click behavior
    pub grouped_click_action: PreviewGroupedClickAction,
}

impl Default for WindowPreviewSettings {
    fn default() -> Self {
        Self {
            trigger: WindowPreviewTrigger::Hover,
            card_width: 256,
            card_height: 0,
            aspect_ratio_mode: PreviewAspectRatioMode::SourceWindow,
            aspect_ratio_num: 16,
            aspect_ratio_den: 9,
            auto_grid: true,
            preferred_columns: 3,
            min_columns: 1,
            max_columns: 8,
            max_rows: 4,
            max_width: 800,
            max_height: 560,
            gap: 10,
            padding: 10,
            border_radius: 10,
            animated: true,
            animation_duration: 150,
            show_titles: true,
            title_lines: PreviewTitleLines::OneLine,
            compact_titles: true,
            title_tooltip: true,
            application_aware_titles: true,
            ordering: PreviewOrderingStrategy::Manual,
            hover_open_delay: 0,
            hover_close_delay: 150,
            pointer_grace_region: 24,
            keep_open_on_traversal: true,
            cache_enabled: true,
            thumbnail_cache_size: 64,
            cache_memory_budget: 32768,
            stale_while_refresh: true,
            prewarm_view: true,
            nearest_first_projection: false,
            grouped_click_action: PreviewGroupedClickAction::OpenPreview,
        }
    }
}

// ============== Automation (Control Plane) Settings ==============

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct AutomationSettings {
    /// master switch for the programmatic control planes
    #[serde(default = "auto_enabled")]
    pub enabled: bool,
    /// bind the REST adapter (always loopback); 0 disables it
    #[serde(default = "auto_enabled")]
    pub rest_enabled: bool,
    /// REST port; 0 lets the OS auto-assign on 127.0.0.1
    #[serde(default = "auto_port")]
    pub rest_port: u16,
    /// serve the MCP stdio adapter through the `slu` client process
    #[serde(default = "auto_enabled")]
    pub mcp_enabled: bool,
    /// emit a bounded trace ring-buffer for `slu trace`
    #[serde(default = "auto_enabled")]
    pub flight_recorder: bool,
}

fn auto_enabled() -> bool {
    true
}

fn auto_port() -> u16 {
    37_074
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            enabled: auto_enabled(),
            rest_enabled: auto_enabled(),
            rest_port: auto_port(),
            mcp_enabled: auto_enabled(),
            flight_recorder: auto_enabled(),
        }
    }
}

// ============== Window Manager Settings ==============

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct WindowManagerSettings {
    /// enable or disable the tiling window manager
    pub enabled: bool,
    /// Key overrides for widget-declared shortcuts (`shortcut_id -> keys`).
    #[serde(rename = "$shortcuts")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    /// enable or disable auto stacking by category
    pub auto_stacking_by_category: bool,
    /// window manager border
    pub border: Border,
    /// the resize size in % to be used when resizing via cli
    pub resize_delta: f32,
    /// default gap between containers
    pub workspace_gap: u32,
    /// default workspace padding
    pub workspace_padding: u32,
    /// default workspace margin
    pub workspace_margin: Rect,
    /// floating window settings
    pub floating: FloatingWindowSettings,
    /// default layout
    pub default_layout: PluginId,
    /// window manager animations
    pub animations: WmAnimations,
    /// window manager drag behavior
    pub drag_behavior: WmDragBehavior,
    /// when to show the stack bar (tabs) on stacked containers
    pub stack_bar_visibility: WmStackBarVisibility,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WmDragBehavior {
    /// While dragging the windows on the layout will be sorted.
    Sort,
    /// On drag end the dragged and the overlaped will be swapped.
    Swap,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum WmStackBarVisibility {
    /// Always show the stack bar, even if the stack only has one window.
    Always,
    /// Only show the stack bar when the stack has 2 or more windows.
    AsNeeded,
}

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct Border {
    pub enabled: bool,
    pub width: f64,
    pub offset: f64,
}

#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct FloatingWindowSettings {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct WmAnimations {
    pub enabled: bool,
    pub duration_ms: u64,
    pub ease_function: String,
}

impl Default for WmAnimations {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_ms: 200,
            ease_function: "EaseOut".into(),
        }
    }
}

impl Default for Border {
    fn default() -> Self {
        Self {
            enabled: true,
            offset: 0.0,
            width: 3.0,
        }
    }
}

impl Default for FloatingWindowSettings {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 500.0,
        }
    }
}

impl Default for WindowManagerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            shortcuts: None,
            auto_stacking_by_category: true,
            border: Border::default(),
            resize_delta: 10.0,
            workspace_gap: 10,
            workspace_padding: 10,
            workspace_margin: Rect::default(),
            floating: FloatingWindowSettings::default(),
            default_layout: "@default/wm-bspwm".into(),
            animations: WmAnimations::default(),
            drag_behavior: WmDragBehavior::Sort,
            stack_bar_visibility: WmStackBarVisibility::AsNeeded,
        }
    }
}

// ================= Seelen Wall ================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum MultimonitorBehaviour {
    /// Each monitor has its own wallpaper
    PerMonitor,
    /// Single wallpaper extended across all monitors
    Extend,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct SeelenWallSettings {
    pub enabled: bool,
    /// Key overrides for widget-declared shortcuts (`shortcut_id -> keys`).
    #[serde(rename = "$shortcuts")]
    pub shortcuts: Option<std::collections::HashMap<String, Vec<String>>>,
    /// update interval in seconds
    pub interval: u32,
    /// randomize order
    pub randomize: bool,
    /// collection id, if none default wallpaper will be used
    pub default_collection: Option<uuid::Uuid>,
    /// multimonitor behaviour
    pub multimonitor_behaviour: MultimonitorBehaviour,
    /// whether to extract and apply accent color from wallpaper
    pub use_accent_color: bool,
    /// fraction of monitor area that must be covered by windows before pausing wallpaper (0.5–1.0)
    pub coverage_pause_threshold: f64,
    /// deprecated, this field will be removed on v3
    #[serde(alias = "backgroundsV2")]
    pub deprecated_bgs: Option<Vec<WallpaperId>>,
}

impl Default for SeelenWallSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: None,
            interval: 300, // 5min
            randomize: false,
            default_collection: None,
            multimonitor_behaviour: MultimonitorBehaviour::PerMonitor,
            use_accent_color: false,
            coverage_pause_threshold: 0.8,
            deprecated_bgs: None,
        }
    }
}

// ========================== Seelen Updates ==============================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum UpdateChannel {
    Release,
    Beta,
    Nightly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct UpdaterSettings {
    pub channel: UpdateChannel,
}

impl Default for UpdaterSettings {
    fn default() -> Self {
        Self {
            channel: UpdateChannel::Release,
        }
    }
}

// ========================== Start of Week ==============================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
#[derive(Default)]
pub enum StartOfWeek {
    #[default]
    Monday,
    Sunday,
    Saturday,
}

// ======================== Final Settings Struct ===============================
#[serde_alias(SnakeCase)]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct Settings {
    pub by_app: AppsConfigurationList,
    /// list of monitors and their configurations
    pub monitors_v3: HashMap<MonitorId, MonitorConfiguration>,
    /// app shortcuts settings
    pub shortcuts: SluShortcutsSettings,
    /// list of selected themes as filename as backguard compatibility for versions before v2.3.8, will be removed in v3
    #[serde(alias = "selectedThemes")]
    pub old_active_themes: Vec<String>,
    /// list of selected themes
    pub active_themes: Vec<ThemeId>,
    /// list of selected icon packs
    pub active_icon_packs: Vec<IconPackId>,
    /// enable or disable dev tools tab in settings
    pub dev_tools: bool,
    /// discord rich presence
    pub drpc: bool,
    /// language to use, if null the system locale is used
    pub language: String,
    /// MomentJS date format
    pub date_format: String,
    /// Start of week for calendar
    pub start_of_week: StartOfWeek,
    /// Updater Settings
    pub updater: UpdaterSettings,
    /// Custom settings for widgets
    pub by_widget: SettingsByWidget,
    /// Custom variables for themes by theme id
    /// ### example
    /// ```json
    /// {
    ///     "@username/themeName": {
    ///         "--css-variable-name": "123px",
    ///         "--css-variable-name2": "#aabbccaa",
    ///     }
    /// }
    /// ```
    pub by_theme: HashMap<ThemeId, ThemeSettings>,
    /// settings for each background
    pub by_wallpaper: HashMap<WallpaperId, WallpaperInstanceSettings>,
    /// list of wallpaper collections
    pub wallpaper_collections: Vec<WallpaperCollection>,
    /// Performance options
    pub performance_mode: PerformanceModeSettings,
    /// enable or disable hardware acceleration
    pub hardware_acceleration: bool,
    /// Enable unstable chromium optimizations (e.g. process-per-site). May improve RAM usage but can cause crashes.
    pub unstable_optimizations: bool,
    /// interval to poll for system resources like cpu, memory, network usage, etc, in seconds.
    pub polling_interval: u64,
    /// Streaming mode: replaces sensitive information (e.g. emails) with placeholders
    /// to avoid exposing personal data in recordings or screenshots.
    pub streaming_mode: bool,
    /// Enable or disable automatic cloud backup sync.
    pub backup_sync_enabled: bool,
    /// Suspend all webviews when Windows GameMode is active to free resources for the game.
    pub suspend_on_game_mode: bool,
    /// Allow editing read-only shortcuts (e.g. system overrides). Only effective when dev_tools is enabled.
    pub unlock_shortcuts: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            by_app: AppsConfigurationList::default(),
            performance_mode: PerformanceModeSettings::default(),
            shortcuts: SluShortcutsSettings::default(),
            drpc: false,
            old_active_themes: Vec::new(),
            active_themes: vec!["@default/theme".into()],
            active_icon_packs: vec!["@system/icon-pack".into()],
            monitors_v3: HashMap::new(),
            dev_tools: false,
            language: Self::get_app_language(),
            date_format: "ddd D MMM, hh:mm A".to_owned(),
            start_of_week: StartOfWeek::default(),
            updater: UpdaterSettings::default(),
            by_widget: SettingsByWidget::default(),
            by_theme: HashMap::new(),
            by_wallpaper: HashMap::new(),
            wallpaper_collections: Vec::new(),
            hardware_acceleration: true,
            unstable_optimizations: false,
            polling_interval: 3,
            streaming_mode: false,
            backup_sync_enabled: true,
            suspend_on_game_mode: false,
            unlock_shortcuts: false,
        }
    }
}

impl Settings {
    pub fn get_system_locale() -> Option<String> {
        sys_locale::get_locale()
    }

    pub fn get_app_language() -> String {
        use crate::constants::SUPPORTED_LANGUAGES;

        let Some(sys_locale) = sys_locale::get_locale() else {
            return "en".to_string();
        };

        if SUPPORTED_LANGUAGES.iter().any(|l| l.value == sys_locale) {
            return sys_locale;
        }

        let Some(base) = sys_locale.split('-').next() else {
            return "en".to_string();
        };
        if SUPPORTED_LANGUAGES.iter().any(|l| l.value == base) {
            return base.to_string();
        }

        "en".to_string()
    }

    pub fn migrate(&mut self) -> Result<()> {
        // Migrate step for v2.5
        if let Some(backgrounds) = self.by_widget.wall.deprecated_bgs.take()
            && !backgrounds.is_empty()
        {
            let collection = WallpaperCollection {
                id: uuid::Uuid::new_v4(),
                name: "Migrated".to_string(),
                wallpapers: backgrounds,
                hidden: false,
            };

            // Set as default collection if no default is set
            if self.by_widget.wall.default_collection.is_none() {
                self.by_widget.wall.default_collection = Some(collection.id);
            }

            self.wallpaper_collections.push(collection);
        }

        Ok(())
    }

    pub fn dedup_themes(&mut self) {
        let mut seen = HashSet::new();
        self.active_themes.retain(|x| seen.insert(x.clone())); // dedup
    }

    pub fn dedup_icon_packs(&mut self) {
        let mut seen = HashSet::new();
        self.active_icon_packs.retain(|x| seen.insert(x.clone())); // dedup
    }

    pub fn sanitize(&mut self) -> Result<()> {
        if self.language.is_empty() {
            self.language = Self::get_app_language();
        }

        // ensure base is always selected
        self.active_themes.insert(0, "@default/theme".into());
        self.dedup_themes();
        // ensure base is always selected
        self.active_icon_packs.insert(0, "@system/icon-pack".into());
        self.dedup_icon_packs();

        self.by_app.prepare();
        self.by_widget.sanitize();

        self.polling_interval = self.polling_interval.max(1);
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parent = path.parent().unwrap_or(path);

        let shortcuts_path = parent.join(format!("{stem}_shortcuts.json"));
        let by_app_path = parent.join(format!("{stem}_by_app.yml"));

        let (main, shortcuts, by_app) = std::thread::scope(|s| {
            let main = s.spawn(|| -> Result<Self> {
                let file = File::open(path)?;
                file.lock_shared()?;
                Ok(serde_json::from_reader(&file)?)
            });

            let shortcuts = s.spawn(|| -> Result<Option<SluShortcutsSettings>> {
                if !shortcuts_path.exists() {
                    return Ok(None);
                }
                let file = File::open(&shortcuts_path)?;
                file.lock_shared()?;
                Ok(Some(serde_json::from_reader(&file)?))
            });

            let by_app = s.spawn(|| -> Result<Option<AppsConfigurationList>> {
                if !by_app_path.exists() {
                    return Ok(None);
                }
                let file = File::open(&by_app_path)?;
                file.lock_shared()?;
                Ok(Some(serde_yaml::from_reader(&file)?))
            });

            (main.join(), shortcuts.join(), by_app.join())
        });

        let mut settings = main.map_err(|_| "settings thread panicked")??;
        if let Some(s) = shortcuts.map_err(|_| "shortcuts thread panicked")?? {
            settings.shortcuts = s;
        }
        if let Some(b) = by_app.map_err(|_| "by_app thread panicked")?? {
            settings.by_app = b;
        }

        settings.migrate()?;
        settings.sanitize()?;
        Ok(settings)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        {
            // Create a copy without splitted fields
            let mut settings_copy = serde_json::to_value(self)?;
            let obj = settings_copy.as_object_mut().unwrap();
            obj.remove("shortcuts");
            obj.remove("byApp");

            let mut file = File::create(path)?;
            file.lock()?;
            serde_json::to_writer_pretty(&file, &settings_copy)?;
            file.flush()?;
        }

        // Save shortcuts to sibling file
        if let (Some(parent), Some(stem)) = (path.parent(), path.file_stem()) {
            let shortcuts_path = parent.join(format!("{}_shortcuts.json", stem.to_string_lossy()));
            let mut shortcuts_file = File::create(&shortcuts_path)?;
            shortcuts_file.lock()?;
            serde_json::to_writer_pretty(&shortcuts_file, &self.shortcuts)?;
            shortcuts_file.flush()?;

            let by_app_path = parent.join(format!("{}_by_app.yml", stem.to_string_lossy()));
            let mut by_app_file = File::create(&by_app_path)?;
            by_app_file.lock()?;
            serde_yaml::to_writer(&by_app_file, &self.by_app)?;
            by_app_file.flush()?;
        }

        Ok(())
    }

    /// This indicates if the widget is enabled on general, doesn't take in care multi-instances
    pub fn is_widget_enabled(&self, widget_id: &WidgetId) -> bool {
        self.by_widget.is_enabled(widget_id)
    }

    pub fn set_widget_enabled(&mut self, widget_id: &WidgetId, enabled: bool) {
        self.by_widget.set_enabled(widget_id, enabled);
    }

    pub fn is_widget_enabled_on_monitor(
        &self,
        widget_id: &WidgetId,
        monitor_id: &MonitorId,
    ) -> bool {
        if !self.is_widget_enabled(widget_id) {
            return false;
        }
        // default to true as new connected monitors should be enabled
        self.monitors_v3
            .get(monitor_id)
            .is_none_or(|monitor_config| monitor_config.by_widget.is_widget_enabled(widget_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(default, rename_all = "camelCase")]
pub struct PerformanceModeSettings {
    pub default: PerformanceMode,
    pub on_battery: PerformanceMode,
    pub on_energy_saver: PerformanceMode,
}

impl Default for PerformanceModeSettings {
    fn default() -> Self {
        Self {
            default: PerformanceMode::Disabled,
            on_battery: PerformanceMode::Minimal,
            on_energy_saver: PerformanceMode::Extreme,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum PerformanceMode {
    /// Does nothing, all animations are enabled.
    Disabled,
    /// Disables windows animations and other heavy effects.
    Minimal,
    /// Disables all the animations.
    Extreme,
}

impl From<u8> for PerformanceMode {
    fn from(value: u8) -> Self {
        match value {
            0 => PerformanceMode::Disabled,
            1 => PerformanceMode::Minimal,
            2 => PerformanceMode::Extreme,
            _ => PerformanceMode::Disabled,
        }
    }
}

impl From<PerformanceMode> for u8 {
    fn from(value: PerformanceMode) -> Self {
        match value {
            PerformanceMode::Disabled => 0,
            PerformanceMode::Minimal => 1,
            PerformanceMode::Extreme => 2,
        }
    }
}
