use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::MonitorId;

/// Resolved state of a managed window at snapshot time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum RuntimeWindowState {
    Normal,
    Maximized,
    Minimized,
    Fullscreen,
}

/// One entry of the native window registry, resolved by the command core.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WindowEntry {
    /// unique id for the current window lifetime (hex hwnd)
    pub runtime_window_id: String,
    /// raw win32 handle
    pub hwnd: isize,
    /// raw title as reported by the window
    pub title: String,
    /// normalized display title (application-aware)
    pub display_title: String,
    /// normalized application key (umid or exe basename)
    pub application: String,
    /// stable logical identity `application:logicalId` (alias-aware)
    pub logical_identity: String,
    /// user assigned short alias, when present
    pub alias: Option<String>,
    pub state: RuntimeWindowState,
    pub monitor: MonitorId,
    pub is_focused: bool,
    /// unix timestamp in ms of last foreground activation; 0 if never
    pub last_foreground_at: i64,
}

/// Grouped application summary produced by the command core.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct ApplicationEntry {
    pub key: String,
    pub window_count: usize,
}

/// Structured per-action result returned by control-plane commands.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct WindowActionResult {
    pub window_id: String,
    pub hwnd: isize,
    pub application: String,
    pub identity: String,
    pub previous_state: RuntimeWindowState,
    pub resulting_state: RuntimeWindowState,
    pub success: bool,
    pub latency_us: u64,
}

/// One frame of the flight-recorder ring buffer.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct TraceFrame {
    /// unix ms
    pub at_ms: u64,
    pub source: String,
    pub command: String,
    pub logical_identity: Option<String>,
    pub runtime_window_id: Option<String>,
    pub hwnd: Option<isize>,
    pub success: bool,
    pub latency_us: u64,
}

/// Aggregated latency percentiles (microseconds) for a measured quantity.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct LatencyStats {
    pub samples: usize,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub max: f64,
}

/// Aggregated metrics of the automation / preview control plane.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct AutomationMetrics {
    pub metadata_ready: LatencyStats,
    pub layout_ready: LatencyStats,
    pub thumbnail_ready: LatencyStats,
    pub first_paint: LatencyStats,
    /// per firstPaint of hover previews that found the model already warm
    pub cache_hits: usize,
    pub cache_misses: usize,
    /// per native action (focus / maximize / focus+maximize ...)
    pub action: LatencyStats,
}

/// Timing of the layered preview pipeline, measured in microseconds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
#[serde(rename_all = "camelCase")]
pub struct PreviewLatency {
    pub metadata_ready_us: u64,
    pub layout_ready_us: u64,
    pub thumbnail_ready_us: u64,
    pub first_paint_us: u64,
    pub cache_hit: bool,
}
