use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use seelen_core::{
    state::SeelenWegSettings,
    system_state::{
        AutomationMetrics, LatencyStats, PreviewLatency, RuntimeWindowState, TraceFrame,
        UserAppWindow, WindowActionResult, WindowEntry,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    error::{Result, ResultLogExt},
    modules::{
        apps::application::USER_APPS_MANAGER,
        weg_core::identity::{self, LogicalWindowIdentity, identity_for},
    },
    state::application::FULL_STATE,
    utils::constants::SEELEN_COMMON,
    windows_api::{WindowsApi, window::Window},
};

/// Bounded flight-recorder ring buffer capacity.
const FLIGHT_RECORDER_CAPACITY: usize = 200;
/// Bounded latency sample window.
const LATENCY_SAMPLE_WINDOW: usize = 400;

// ============================ persistent state ============================

/// One persisted browser window slot: a stable logical name plus the
/// geometry/monitor fingerprint used for deterministic reclamation.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SlotRecord {
    pub name: String,
    pub fingerprint: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WegPersistedState {
    /// manual drag order per application key: list of logical local ids
    pub orders: HashMap<String, Vec<String>>,
    /// short user aliases keyed by full logical identity `app:local`
    pub aliases: HashMap<String, String>,
    /// preferred monitor device name per logical identity
    pub monitor_affinity: HashMap<String, String>,
    /// persistent window slot table per application key (browser identity)
    pub slots: HashMap<String, Vec<SlotRecord>>,
    /// global persistent spatial order used by the Task Switcher, keyed by
    /// full logical identity; destroyed ids stay as tombstones so recreated
    /// windows recover their old slot
    pub switcher_order: Vec<String>,
}

fn state_path() -> PathBuf {
    SEELEN_COMMON.app_data_dir().join("weg_state.json")
}

fn load_persisted() -> WegPersistedState {
    let path = state_path();
    if !path.exists() {
        return WegPersistedState::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

static PERSISTED: LazyLock<Mutex<WegPersistedState>> =
    LazyLock::new(|| Mutex::new(load_persisted()));

fn save_persisted(state: &WegPersistedState) {
    let path = state_path();
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            std::fs::write(&path, json).log_error();
        }
        Err(err) => log::error!("Failed to serialize weg state: {err}"),
    }
}

/// Append newly created slot records to the persisted slot table.
pub fn save_slots(new_records: Vec<(String, SlotRecord)>) {
    if new_records.is_empty() {
        return;
    }
    let mut state = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();
    for (app, record) in new_records {
        let list = state.slots.entry(app).or_default();
        if !list.iter().any(|r| r.name == record.name) {
            list.push(record);
        }
    }
    save_persisted(&state);
}

/// Resolve the alias for a full logical identity, if defined.
#[allow(dead_code)]
pub fn alias_of(identity: &str) -> Option<String> {
    PERSISTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .aliases
        .get(identity)
        .cloned()
}

// ============================ latency metrics ============================

#[derive(Debug, Default)]
struct Ring {
    samples: VecDeque<u64>,
}

impl Ring {
    fn push(&mut self, value_us: u64) {
        if self.samples.len() == LATENCY_SAMPLE_WINDOW {
            self.samples.pop_front();
        }
        self.samples.push_back(value_us);
    }

    fn stats(&self) -> LatencyStats {
        if self.samples.is_empty() {
            return LatencyStats::default();
        }
        let mut v: Vec<u64> = self.samples.iter().copied().collect();
        v.sort_unstable();
        let n = v.len();
        let at = |q: f64| -> f64 {
            let idx = ((n as f64 - 1.0) * q).ceil() as usize;
            v[idx.min(n - 1)] as f64
        };
        LatencyStats {
            samples: n,
            p50: at(0.50),
            p95: at(0.95),
            p99: at(0.99),
            max: *v.last().unwrap() as f64,
        }
    }
}

#[derive(Debug, Default)]
struct Metrics {
    metadata: Ring,
    layout: Ring,
    thumbnail: Ring,
    first_paint: Ring,
    cache_hits: usize,
    cache_misses: usize,
    action: Ring,
}

static METRICS: LazyLock<Mutex<Metrics>> = LazyLock::new(|| Mutex::new(Metrics::default()));

pub fn report_preview_latency(latency: &PreviewLatency) {
    let mut metrics = METRICS.lock().unwrap_or_else(|e| e.into_inner());
    metrics.metadata.push(latency.metadata_ready_us);
    metrics.layout.push(latency.layout_ready_us);
    metrics.thumbnail.push(latency.thumbnail_ready_us);
    metrics.first_paint.push(latency.first_paint_us);
    if latency.cache_hit {
        metrics.cache_hits += 1;
    } else {
        metrics.cache_misses += 1;
    }
}

pub fn automation_metrics() -> AutomationMetrics {
    let metrics = METRICS.lock().unwrap_or_else(|e| e.into_inner());
    AutomationMetrics {
        metadata_ready: metrics.metadata.stats(),
        layout_ready: metrics.layout.stats(),
        thumbnail_ready: metrics.thumbnail.stats(),
        first_paint: metrics.first_paint.stats(),
        cache_hits: metrics.cache_hits,
        cache_misses: metrics.cache_misses,
        action: metrics.action.stats(),
    }
}

fn record_action_latency(us: u64) {
    let mut metrics = METRICS.lock().unwrap_or_else(|e| e.into_inner());
    metrics.action.push(us);
}

// ============================ flight recorder ============================

static FLIGHT_RECORDER: LazyLock<Mutex<VecDeque<TraceFrame>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(FLIGHT_RECORDER_CAPACITY)));

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn record_trace(frame: TraceFrame) {
    if !FULL_STATE
        .load()
        .settings
        .by_widget
        .weg
        .automation
        .flight_recorder
    {
        return;
    }
    let mut rec = FLIGHT_RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    if rec.len() == FLIGHT_RECORDER_CAPACITY {
        rec.pop_front();
    }
    rec.push_back(frame);
}

pub fn get_trace() -> Vec<TraceFrame> {
    FLIGHT_RECORDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .cloned()
        .collect()
}

// ============================ window registry ============================

fn runtime_id(hwnd: isize) -> String {
    format!("{hwnd:x}")
}

fn state_of(win: &Window) -> RuntimeWindowState {
    if win.is_minimized() {
        RuntimeWindowState::Minimized
    } else if win.is_maximized() {
        RuntimeWindowState::Maximized
    } else if win.is_fullscreen() {
        RuntimeWindowState::Fullscreen
    } else {
        RuntimeWindowState::Normal
    }
}

fn make_entry(win: &UserAppWindow, window: &Window, ident: &LogicalWindowIdentity) -> WindowEntry {
    WindowEntry {
        runtime_window_id: runtime_id(win.hwnd),
        hwnd: win.hwnd,
        title: win.title.clone(),
        display_title: ident.display_title.clone(),
        application: ident.application.clone(),
        logical_identity: ident.logical_identity(),
        alias: ident.alias.clone(),
        state: state_of(window),
        monitor: win.monitor.clone(),
        is_focused: window.is_focused(),
        last_foreground_at: win.last_foreground_at,
    }
}

/// Full snapshot of the managed windows with stable identities, ordered inside
/// each application group by the configured ordering strategy.
pub fn window_entries() -> Vec<WindowEntry> {
    let settings = FULL_STATE.load();
    let weg: &SeelenWegSettings = &settings.settings.by_widget.weg;
    let entries: Vec<UserAppWindow> = USER_APPS_MANAGER.interactable_windows.to_vec();

    let mut persisted = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();

    let mut ordered_keys: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<WindowEntry>> = HashMap::new();
    let mut new_slots: Vec<(String, SlotRecord)> = Vec::new();

    for win in entries {
        let window = Window::from(win.hwnd);
        if identity::provider_for(&win) == identity::IdentityProvider::Edge {
            // Materialize the browser slot once per snapshot against a working
            // table so simultaneous new windows never reuse a number.
            let fingerprint = identity::slot_fingerprint(&win);
            let app = identity::application_key_for(&win);
            let table = persisted.slots.entry(app.clone()).or_default();
            let matched = table.iter().find(|r| r.fingerprint == fingerprint).cloned();
            let record = match matched {
                Some(record) => record,
                None => {
                    let mut used: Vec<u32> = table
                        .iter()
                        .filter_map(|r| r.name.strip_prefix("window-")?.parse::<u32>().ok())
                        .collect();
                    used.sort_unstable();
                    let mut next = 1u32;
                    for n in &used {
                        if *n == next {
                            next += 1;
                        }
                    }
                    let record = SlotRecord {
                        name: format!("window-{next}"),
                        fingerprint,
                    };
                    table.push(record.clone());
                    new_slots.push((app.clone(), record.clone()));
                    record
                }
            };
            let _ = &record; // slot is now present in the working table
        }
        let ident: LogicalWindowIdentity = identity_for(&win, &persisted);
        let entry = make_entry(&win, &window, &ident);
        if !groups.contains_key(&ident.application) {
            ordered_keys.push(ident.application.clone());
        }
        groups
            .entry(ident.application.clone())
            .or_default()
            .push(entry);
    }

    let strategy = &weg.preview.ordering;
    for key in &ordered_keys {
        let Some(group) = groups.get_mut(key) else {
            continue;
        };
        match strategy {
            seelen_core::state::PreviewOrderingStrategy::Manual
            | seelen_core::state::PreviewOrderingStrategy::Hybrid => {
                apply_manual_order(key, group, &persisted);
            }
            seelen_core::state::PreviewOrderingStrategy::Stable => {}
            seelen_core::state::PreviewOrderingStrategy::Mru => {
                group.sort_by_key(|a| std::cmp::Reverse(a.last_foreground_at));
            }
            seelen_core::state::PreviewOrderingStrategy::Alphabetical => {
                group.sort_by(|a, b| a.display_title.cmp(&b.display_title));
            }
            seelen_core::state::PreviewOrderingStrategy::ApplicationDefined => {
                group.sort_by_key(|e| e.hwnd);
            }
        }
    }

    let mut result = Vec::new();
    let mut emitted = HashSet::new();
    for key in ordered_keys {
        for entry in groups.remove(&key).unwrap_or_default() {
            if emitted.insert(entry.hwnd) {
                result.push(entry);
            }
        }
    }
    save_slots(new_slots);
    result
}

/// Legacy order-storage keys produced by older frontend app-key logic, so
/// previously persisted manual orders keep resolving after normalization.
fn legacy_orders_lookup<'a>(key: &str, state: &'a WegPersistedState) -> Option<&'a Vec<String>> {
    if let Some(saved) = state.orders.get(key) {
        return Some(saved);
    }
    const LEGACY: &[(&str, &[&str])] = &[
        ("code", &["microsoft.visualstudiocode", "code"]),
        (
            "vscode-insiders",
            &["microsoft.visualstudiocode.insiders", "code - insiders"],
        ),
        (
            "edge-stable",
            &["msedge", "microsoft.edge", "microsoft.edge.stable"],
        ),
        ("edge-beta", &["msedge-beta", "microsoft.edge.beta"]),
        ("edge-dev", &["msedge-dev", "microsoft.edge.dev"]),
        ("edge-canary", &["msedge-canary", "microsoft.edge.canary"]),
        ("wt", &["microsoft.windowsterminal"]),
        ("wt-canary", &["microsoft.windowsterminal.canary"]),
    ];
    let candidates = LEGACY.iter().find(|(k, _)| *k == key)?.1;
    candidates
        .iter()
        .find_map(|legacy| state.orders.get(*legacy))
}

fn apply_manual_order(key: &str, group: &mut Vec<WindowEntry>, state: &WegPersistedState) {
    let Some(saved) = legacy_orders_lookup(key, state) else {
        return;
    };
    let mut indexed: Vec<(usize, WindowEntry)> = Vec::with_capacity(group.len());
    for e in group.drain(..) {
        let local = e
            .logical_identity
            .rsplit_once(':')
            .map(|(_, l)| l.to_string())
            .unwrap_or_default();
        // aliased identities are stored by alias too
        let pos = saved
            .iter()
            .position(|id| *id == local || e.alias.as_deref() == Some(id.as_str()))
            .unwrap_or(saved.len() + indexed.len());
        indexed.push((pos, e));
    }
    indexed.sort_by_key(|(pos, _)| *pos);
    group.extend(indexed.into_iter().map(|(_, e)| e));
}

/// Tombstones older than this are dropped when rebuilding the global order.
const SWITCHER_TOMBSTONE_CAP: usize = 200;

/// Globally persistent Task Switcher order.
///
/// `window_entries()` is grouped per application; the switcher needs one flat
/// spatial order across all apps. The persisted list fixes the cross-group
/// interleaving (stored rank of each application's first known identity;
/// unknown groups append, destroyed ids stay as tombstones so recreated
/// windows recover their slot). Inside one group the freshest per-group
/// strategy order (manual/strategy from `window_entries`) wins, so recent
/// drags are never stale. The rebuilt list is persisted.
pub fn task_switcher_entries() -> Vec<WindowEntry> {
    let entries = window_entries();

    let mut state = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();

    // first appearance rank of every application key in the persisted order
    let mut group_rank: HashMap<&str, usize> = HashMap::with_capacity(state.switcher_order.len());
    let mut next_rank = 0usize;
    for id in &state.switcher_order {
        let app = id.split(':').next().unwrap_or(id.as_str());
        if !group_rank.contains_key(app) {
            group_rank.insert(app, next_rank);
            next_rank += 1;
        }
    }

    // (group rank, fresh index) keys; the fresh index keeps per-group order
    let mut keyed: Vec<(usize, usize, WindowEntry)> = Vec::with_capacity(entries.len());
    for (fresh, entry) in entries.into_iter().enumerate() {
        let app = entry
            .logical_identity
            .split(':')
            .next()
            .unwrap_or(entry.logical_identity.as_str());
        let group = group_rank.get(app).copied().unwrap_or_else(|| {
            let r = next_rank;
            next_rank += 1;
            r
        });
        keyed.push((group, fresh, entry));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let result: Vec<WindowEntry> = keyed.into_iter().map(|(_, _, e)| e).collect();

    // Rebuild: live ids in display order first, then retained tombstones.
    let mut rebuilt: Vec<String> = Vec::with_capacity(result.len());
    let mut live: HashSet<&str> = HashSet::with_capacity(result.len());
    for entry in &result {
        live.insert(entry.logical_identity.as_str());
        rebuilt.push(entry.logical_identity.clone());
    }
    for old in &state.switcher_order {
        if !live.contains(old.as_str()) {
            rebuilt.push(old.clone());
            if rebuilt.len() >= SWITCHER_TOMBSTONE_CAP {
                break;
            }
        }
    }

    if rebuilt != state.switcher_order {
        state.switcher_order = rebuilt;
        save_persisted(&state);
    }
    result
}

// ============================ resolution ============================

/// Resolve one identification string: runtime id, full logical identity,
/// alias or (case-insensitive) display/raw title.
pub fn resolve_window(identification: &str) -> Option<WindowEntry> {
    let id = identification.trim();
    if id.is_empty() {
        return None;
    }
    window_entries().into_iter().find(|e| {
        e.runtime_window_id == id
            || e.logical_identity == id
            || e.alias.as_deref() == Some(id)
            || e.display_title.eq_ignore_ascii_case(id)
            || e.title.eq_ignore_ascii_case(id)
    })
}

pub fn find_windows(query: &str) -> Vec<WindowEntry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return window_entries();
    }
    window_entries()
        .into_iter()
        .filter(|e| {
            e.logical_identity.to_lowercase().contains(&q)
                || e.display_title.to_lowercase().contains(&q)
                || e.title.to_lowercase().contains(&q)
                || e.alias
                    .as_deref()
                    .is_some_and(|a| a.to_lowercase().contains(&q))
        })
        .collect()
}

pub fn get_window(identification: &str) -> Option<WindowEntry> {
    resolve_window(identification)
}

pub fn get_recent_windows(limit: u32) -> Vec<WindowEntry> {
    let mut entries = window_entries();
    entries.sort_by_key(|a| std::cmp::Reverse(a.last_foreground_at));
    entries.truncate(limit as usize);
    entries
}

pub fn get_window_order(app: &str) -> Vec<String> {
    PERSISTED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .orders
        .get(app)
        .cloned()
        .unwrap_or_default()
}

pub fn set_window_order(app: &str, identities: &[String]) {
    let mut state = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();
    state.orders.insert(app.to_string(), identities.to_vec());
    save_persisted(&state);
}

pub fn set_window_alias(alias: &str, identity: &str) {
    let mut state = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();
    state
        .aliases
        .insert(identity.to_string(), alias.to_string());
    save_persisted(&state);
}

// ============================ native actions ============================

fn make_action(
    entry: &WindowEntry,
    previous: RuntimeWindowState,
    window: &Window,
    success: bool,
    start: Instant,
) -> WindowActionResult {
    let resulting = if success { state_of(window) } else { previous };
    let latency_us = start.elapsed().as_micros() as u64;
    record_action_latency(latency_us);
    WindowActionResult {
        window_id: entry.runtime_window_id.clone(),
        hwnd: entry.hwnd,
        application: entry.application.clone(),
        identity: entry.logical_identity.clone(),
        previous_state: previous,
        resulting_state: resulting,
        success,
        latency_us,
    }
}

fn trace_frame(
    source: &str,
    command: &str,
    entry: &WindowEntry,
    success: bool,
    start: Instant,
) -> TraceFrame {
    TraceFrame {
        at_ms: now_millis(),
        source: source.to_string(),
        command: command.to_string(),
        logical_identity: Some(entry.logical_identity.clone()),
        runtime_window_id: Some(entry.runtime_window_id.clone()),
        hwnd: Some(entry.hwnd),
        success,
        latency_us: start.elapsed().as_micros() as u64,
    }
}

pub fn focus_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);
    let mut success = true;
    if window.is_minimized() {
        success &= window.unminimize().is_ok();
    }
    success &= window.focus().is_ok();
    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(source, "focus", &entry, success, start));
    Ok(result)
}

pub fn maximize_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    use windows::Win32::UI::WindowsAndMessaging::{SW_MAXIMIZE, SW_RESTORE};
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);
    let mut success = true;
    if window.is_minimized() {
        success &= window.unminimize().is_ok();
    } else if window.is_maximized() {
        success &= window.show_window_async(SW_RESTORE).is_ok();
    } else {
        success &= window.show_window_async(SW_MAXIMIZE).is_ok();
    }
    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(source, "maximize", &entry, success, start));
    Ok(result)
}

pub fn restore_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    use windows::Win32::UI::WindowsAndMessaging::SW_RESTORE;
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);
    let success = window.show_window_async(SW_RESTORE).is_ok();
    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(source, "restore", &entry, success, start));
    Ok(result)
}

pub fn minimize_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    use windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);
    let success = window.show_window_async(SW_MINIMIZE).is_ok();
    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(source, "minimize", &entry, success, start));
    Ok(result)
}

pub fn close_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);
    let success = WindowsApi::post_message(window.hwnd(), WM_CLOSE, 0, 0).is_ok();
    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(source, "close", &entry, success, start));
    Ok(result)
}

/// Atomic focus + maximize: resolve the HWND once and run the correct Win32
/// sequence for the observed state.
pub fn focus_and_maximize_window(identification: &str, source: &str) -> Result<WindowActionResult> {
    use windows::Win32::UI::WindowsAndMessaging::SW_MAXIMIZE;
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);

    let mut success = true;
    if previous == RuntimeWindowState::Minimized {
        success &= window.unminimize().is_ok();
    }
    success &= window.focus().is_ok();
    success &= window.show_window_async(SW_MAXIMIZE).is_ok();

    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(
        source,
        "focus_and_maximize",
        &entry,
        success,
        start,
    ));
    Ok(result)
}

pub fn move_window_to_monitor(
    identification: &str,
    monitor_index: u32,
    source: &str,
) -> Result<WindowActionResult> {
    use crate::windows_api::MonitorEnumerator;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::SWP_NOZORDER;
    let start = Instant::now();
    let entry = resolve_window(identification).ok_or("Window not found")?;
    let previous = entry.state;
    let window = Window::from(entry.hwnd);

    let monitors = MonitorEnumerator::enumerate_win32()?;
    let Some(monitor) = monitors.into_iter().nth(monitor_index as usize) else {
        return Err("Invalid monitor index".into());
    };
    let rect = monitor.rect()?;
    let win_rect = RECT {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    let success = WindowsApi::set_position(window.hwnd(), None, &win_rect, SWP_NOZORDER).is_ok();

    if let Ok(id) = monitor.stable_id() {
        let mut state = PERSISTED.lock().unwrap_or_else(|e| e.into_inner()).clone();
        state
            .monitor_affinity
            .insert(entry.logical_identity.clone(), id.0);
        save_persisted(&state);
    }

    let result = make_action(&entry, previous, &window, success, start);
    record_trace(trace_frame(
        source,
        "move_to_monitor",
        &entry,
        success,
        start,
    ));
    Ok(result)
}

// ============================ taskbar items ============================

pub fn get_taskbar_order() -> Vec<String> {
    let items = crate::state::application::WEG_ITEMS_MANAGER.get();
    items
        .left
        .iter()
        .chain(items.center.iter())
        .chain(items.right.iter())
        .map(|item| item.id().to_string())
        .collect()
}
