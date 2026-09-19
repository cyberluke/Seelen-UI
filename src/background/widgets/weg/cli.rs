pub use slu_ipc::commands::WegCli;
use slu_ipc::commands::WegCommand;

use seelen_core::{
    state::{WegItem, WegItemData},
    system_state::{MonitorId, UserAppWindow, WindowEntry},
};
use windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;

use crate::{
    error::Result,
    modules::{
        apps::application::USER_APPS_MANAGER,
        monitors::infrastructure::get_connected_monitors,
        weg_core::{application as core, infrastructure},
    },
    state::application::WEG_ITEMS_MANAGER,
    widgets::window_manager::state_v2::WM_STATE,
    windows_api::{WindowsApi, window::Window},
};

/// Mirrors `getWindowsForItem` from the frontend (`windows.ts`).
///
/// Grouping rules:
///   1. Window has a umid  → matched only by exact umid equality. Path is not used.
///      If no item has that umid, a new item will be created for it.
///   2. Window has no umid → matched by exact path (item.relaunch.command or item.path).
///
/// note: on update of this function check src\ui\svelte\weg\state\windows.svelte.ts
fn get_windows_for_item<'a>(
    item: &WegItemData,
    interactables: &'a [UserAppWindow],
) -> Vec<&'a UserAppWindow> {
    let item_command = item.relaunch.as_ref().map(|r| r.command.to_lowercase());
    let item_path = item.path.to_string_lossy().to_lowercase();

    interactables
        .iter()
        .filter(|w| {
            if w.umid.is_some() {
                return item.umid == w.umid;
            }

            let win_path = w
                .process
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if win_path.is_empty() {
                return false;
            }

            item_command.as_deref() == Some(win_path.as_str()) || item_path == win_path
        })
        .collect()
}

fn json<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_string(value).ok()
}

pub fn process(cmd: WegCli) -> Result<Option<String>> {
    #[allow(irrefutable_let_patterns)]
    if let WegCommand::ForegroundOrRunApp { index } = cmd.subcommand {
        let weg_items = WEG_ITEMS_MANAGER.get();

        let all_items: Vec<&WegItem> = weg_items
            .left
            .iter()
            .chain(weg_items.center.iter())
            .chain(weg_items.right.iter())
            .filter(|item| matches!(item, WegItem::AppOrFile(_)))
            .collect();

        if all_items.len() <= index {
            return Ok(None);
        }

        let WegItem::AppOrFile(inner_data) = all_items[index] else {
            return Ok(None);
        };

        let interactables = USER_APPS_MANAGER.interactable_windows.to_vec();
        let windows = get_windows_for_item(inner_data, &interactables);

        if windows.is_empty() {
            let command = inner_data
                .relaunch
                .as_ref()
                .map(|r| r.command.clone())
                .unwrap_or_else(|| inner_data.path.to_string_lossy().to_string());
            let args = inner_data
                .relaunch
                .as_ref()
                .and_then(|r| r.args.as_ref())
                .map(|a| a.to_string());
            let working_dir = inner_data
                .relaunch
                .as_ref()
                .and_then(|r| r.working_dir.clone());
            WindowsApi::execute(command, args, working_dir, false)?;
        } else {
            let focused = windows.iter().find(|w| Window::from(w.hwnd).is_focused());
            if let Some(w) = focused {
                Window::from(w.hwnd).show_window_async(SW_MINIMIZE)?;
            } else if let Some(w) = windows.first() {
                let window = Window::from(w.hwnd);
                if window.is_window() {
                    window.unminimize()?;
                    window.focus()?;
                }
            }
        }
        return Ok(None);
    }

    let payload = match cmd.subcommand {
        WegCommand::Windows => json(&core::window_entries()),
        WegCommand::Apps => {
            use std::collections::HashMap;
            let mut counts: HashMap<String, usize> = HashMap::new();
            for e in core::window_entries() {
                *counts.entry(e.application).or_default() += 1;
            }
            let apps: Vec<seelen_core::system_state::ApplicationEntry> = counts
                .into_iter()
                .map(
                    |(key, window_count)| seelen_core::system_state::ApplicationEntry {
                        key,
                        window_count,
                    },
                )
                .collect();
            json(&apps)
        }
        WegCommand::Get { identification } => json(&core::get_window(&identification)),
        WegCommand::Find { query } => json(&core::find_windows(&query)),
        WegCommand::Focus { identification } => {
            json(&infrastructure::weg_focus_window(identification)?)
        }
        WegCommand::FocusMaximize { identification } => json(
            &infrastructure::weg_focus_and_maximize_window(identification)?,
        ),
        WegCommand::Maximize { identification } => {
            json(&infrastructure::weg_maximize_window(identification)?)
        }
        WegCommand::Restore { identification } => {
            json(&infrastructure::weg_restore_window(identification)?)
        }
        WegCommand::Minimize { identification } => {
            json(&infrastructure::weg_minimize_window(identification)?)
        }
        WegCommand::Close { identification } => json(&core::close_window(&identification, "cli")?),
        WegCommand::MoveToMonitor {
            identification,
            monitor,
        } => json(&core::move_window_to_monitor(
            &identification,
            monitor,
            "cli",
        )?),
        WegCommand::OrderList { app } => json(&core::get_window_order(&app)),
        WegCommand::OrderMove { app, identity, to } => {
            let mut order = core::get_window_order(&app);
            if order.is_empty() {
                order = core::window_entries()
                    .into_iter()
                    .filter(|e| e.application == app)
                    .map(|e| {
                        e.logical_identity
                            .rsplit_once(':')
                            .map(|(_, l)| l.to_string())
                            .unwrap_or(e.logical_identity)
                    })
                    .collect();
            }
            let idx = order
                .iter()
                .position(|id| id.eq_ignore_ascii_case(&identity))
                .ok_or("identity not in order")?;
            let item = order.remove(idx);
            let target = to.min(order.len());
            order.insert(target, item);
            core::set_window_order(&app, &order);
            json(&order)
        }
        WegCommand::Recent { limit } => json(&core::get_recent_windows(limit.unwrap_or(10))),
        WegCommand::State => json(&weg_state_json()),
        WegCommand::Metrics => json(&core::automation_metrics()),
        WegCommand::Trace => json(&core::get_trace()),
        WegCommand::ForegroundOrRunApp { .. } => None,
    };
    Ok(payload)
}

/// Resolved identity info for one window (logical identity + user alias).
#[derive(Debug, Clone)]
struct WindowIdentity {
    logical: String,
    alias: Option<String>,
}

/// One-shot map from hwnd to resolved identity, from the command-core snapshot.
fn identity_map(entries: &[WindowEntry]) -> std::collections::HashMap<isize, WindowIdentity> {
    entries
        .iter()
        .map(|entry| {
            let identity = WindowIdentity {
                logical: entry.logical_identity.clone(),
                alias: entry.alias.clone(),
            };
            (entry.hwnd, identity)
        })
        .collect()
}

/// camelCase window view of a runtime `UserAppWindow`.
fn window_json(
    win: &UserAppWindow,
    identity: Option<&WindowIdentity>,
    leader: bool,
    foreground: isize,
) -> serde_json::Value {
    let focused = win.hwnd == foreground;
    let geometry = win.rect.as_ref().map(|rect| {
        serde_json::json!({
            "left": rect.left,
            "top": rect.top,
            "width": rect.width(),
            "height": rect.height(),
        })
    });
    serde_json::json!({
        "id": format!("{:x}", win.hwnd),
        "title": win.title.clone(),
        "alias": identity.and_then(|i| i.alias.clone()),
        "logicalIdentity": identity.map(|i| i.logical.clone()).unwrap_or_default(),
        "processId": win.process.id,
        "isFocused": focused,
        "focused": focused,
        "isGroupLeader": leader,
        "hasDialog": false,
        "geometry": geometry,
        "application": win.app_name.clone(),
        "monitor": win.monitor.clone(),
    })
}

/// Application-group key: the `application` part of the logical identity,
/// falling back to the raw app name.
fn group_key_of(identity: Option<&WindowIdentity>, fallback: &str) -> String {
    identity
        .and_then(|i| i.logical.split_once(':').map(|(app, _)| app.to_owned()))
        .unwrap_or_else(|| fallback.to_owned())
}

/// Unified camelCase runtime state; `perMonitor[]` groups the weg items by
/// monitor index, `previews[]` lists the live runtime windows of the state.
fn weg_state_json() -> serde_json::Value {
    let metrics = core::automation_metrics();
    let items = WEG_ITEMS_MANAGER.get();
    let interactables = USER_APPS_MANAGER.interactable_windows.to_vec();
    let monitors = get_connected_monitors();
    let workspaces = crate::virtual_desktops::SluWorkspacesManager2::instance();
    let identities = identity_map(&core::window_entries());

    // group leaders: hwnd with highest last-foreground time per app key
    let mut leaders: std::collections::HashMap<String, (isize, i64)> =
        std::collections::HashMap::new();
    for win in &interactables {
        let key = group_key_of(identities.get(&win.hwnd), &win.app_name);
        let best = leaders
            .entry(key)
            .or_insert((win.hwnd, win.last_foreground_at));
        if win.last_foreground_at > best.1 {
            *best = (win.hwnd, win.last_foreground_at);
        }
    }
    let leaders: std::collections::HashMap<String, isize> = leaders
        .into_iter()
        .map(|(key, (hwnd, _))| (key, hwnd))
        .collect();

    let foreground = Window::get_foregrounded().address();

    let win_json = |win: &UserAppWindow| -> serde_json::Value {
        let identity = identities.get(&win.hwnd);
        let leader = leaders
            .get(&group_key_of(identity, &win.app_name))
            .is_some_and(|hwnd| *hwnd == win.hwnd);
        window_json(win, identity, leader, foreground)
    };

    // runtime window ids present on the active workspace of each monitor
    let active_ids: std::collections::HashMap<MonitorId, std::collections::HashSet<isize>> =
        monitors
            .iter()
            .map(|monitor| {
                let ids = workspaces
                    .monitors
                    .get(&monitor.id, |vd| {
                        let active = vd.active_workspace_id().clone();
                        vd.workspaces
                            .rows()
                            .iter()
                            .flatten()
                            .find(|ws| ws.id == active)
                            .map(|ws| {
                                ws.windows
                                    .iter()
                                    .copied()
                                    .collect::<std::collections::HashSet<_>>()
                            })
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                (monitor.id.clone(), ids)
            })
            .collect();

    let per_monitor: Vec<serde_json::Value> = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let item_objs: Vec<serde_json::Value> = items
                .left
                .iter()
                .chain(items.center.iter())
                .chain(items.right.iter())
                .filter_map(|item| match item {
                    WegItem::AppOrFile(data) => {
                        let item_windows: Vec<_> = get_windows_for_item(data, &interactables)
                            .into_iter()
                            .filter(|win| win.monitor == monitor.id)
                            .collect();
                        if item_windows.is_empty() {
                            return None;
                        }
                        Some(serde_json::json!({
                            "name": data.display_name.clone(),
                            "isFocused": item_windows
                                .iter()
                                .any(|win| win.hwnd == foreground),
                            "isMonitored": active_ids
                                .get(&monitor.id)
                                .is_some_and(|set| item_windows.iter().any(|w| set.contains(&w.hwnd))),
                            "isPinned": data.pinned || data.prevent_pinning,
                            "windows": item_windows
                                .iter()
                                .map(|win| win_json(win))
                                .collect::<Vec<_>>(),
                        }))
                    }
                    _ => None,
                })
                .collect();
            serde_json::json!({
                "index": index,
                "monitor": { "id": monitor.id.clone(), "name": monitor.name.clone(), "isPrimary": monitor.is_primary },
                "items": item_objs,
            })
        })
        .collect();

    let previews: Vec<serde_json::Value> = interactables.iter().map(win_json).collect();

    let focused: Vec<serde_json::Value> = interactables
        .iter()
        .filter(|win| win.hwnd == foreground)
        .map(win_json)
        .collect();

    let next_id = {
        let guard = WM_STATE.lock();
        guard
            .state
            .workspaces
            .values()
            .next()
            .map(|tree| tree.next_id)
    };

    serde_json::json!({
        "version": crate::boot::generation(""),
        "nextId": next_id,
        "metadataReady": metrics.metadata_ready,
        "layoutReady": metrics.layout_ready,
        "thumbnailReady": metrics.thumbnail_ready,
        "firstPaint": metrics.first_paint,
        "cacheHits": metrics.cache_hits,
        "cacheMisses": metrics.cache_misses,
        "action": metrics.action,
        "focused": focused,
        "monitored": monitors.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        "monitors": monitors.iter().map(|m| serde_json::json!({
            "id": m.id.clone(), "name": m.name.clone(), "isPrimary": m.is_primary,
        })).collect::<Vec<_>>(),
        "perMonitor": per_monitor,
        "previews": previews,
    })
}
