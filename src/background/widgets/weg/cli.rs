pub use slu_ipc::commands::WegCli;
use slu_ipc::commands::WegCommand;

use seelen_core::{
    state::{WegItem, WegItemData},
    system_state::UserAppWindow,
};
use windows::Win32::UI::WindowsAndMessaging::SW_MINIMIZE;

use crate::{
    error::Result,
    modules::{
        apps::application::USER_APPS_MANAGER,
        weg_core::{application as core, infrastructure},
    },
    state::application::WEG_ITEMS_MANAGER,
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
        WegCommand::Metrics => json(&core::automation_metrics()),
        WegCommand::Trace => json(&core::get_trace()),
        WegCommand::ForegroundOrRunApp { .. } => None,
    };
    Ok(payload)
}
