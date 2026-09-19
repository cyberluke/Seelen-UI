use std::collections::HashSet;
use std::sync::Once;

use seelen_core::{
    handlers::SeelenEvent,
    system_state::{SysTrayIcon, SysTrayIconId, SystrayIconAction, TrayIconInfo, TrayPinState},
};

use crate::{
    app::emit_to_webviews, error::Result, modules::system_tray::application::SystemTrayManager,
    state::application::FULL_STATE,
};

fn get_system_tray_manager() -> &'static SystemTrayManager {
    static TAURI_EVENT_REGISTRATION: Once = Once::new();
    TAURI_EVENT_REGISTRATION.call_once(|| {
        SystemTrayManager::subscribe(|_event| {
            emit_to_webviews(
                SeelenEvent::SystemTrayChanged,
                SystemTrayManager::instance().icons(),
            );
        });
    });
    SystemTrayManager::instance()
}

#[tauri::command(async)]
pub fn get_system_tray_icons() -> Vec<SysTrayIcon> {
    get_system_tray_manager().icons()
}

#[tauri::command(async)]
pub fn send_system_tray_icon_action(id: SysTrayIconId, action: SystrayIconAction) -> Result<()> {
    get_system_tray_manager().send_action(&id, &action)?;
    Ok(())
}

// ── Semantic desktop integration ────────────────────────────────────────────

fn current_pin_state() -> TrayPinState {
    TrayPinState {
        order: FULL_STATE
            .load()
            .settings
            .by_widget
            .fancy_toolbar
            .tray
            .pinned
            .clone(),
    }
}

fn write_pin_state(order: Vec<String>) -> Result<()> {
    let mut settings = FULL_STATE.load().settings.clone();
    settings.by_widget.fancy_toolbar.tray.pinned = order;
    settings.sanitize()?;
    FULL_STATE.rcu(move |state| {
        let mut next = state.cloned();
        next.settings = settings.clone();
        next
    });
    FULL_STATE.load().write_settings()?;
    crate::backups::application::on_settings_saved();
    Ok(())
}

fn make_info(icon: &SysTrayIcon, pinned: &TrayPinState) -> TrayIconInfo {
    let order = pinned
        .order
        .iter()
        .position(|k| *k == icon.logical_id)
        .map(|i| (i + 1) as u32);
    TrayIconInfo {
        logical_id: icon.logical_id.clone(),
        runtime_id: icon.stable_id.clone(),
        tooltip: icon.tooltip.clone(),
        application_display_name: icon.application_display_name.clone(),
        process_id: icon.process_id,
        process_path: icon.process_path.clone(),
        process_name: icon.process_name.clone(),
        app_user_model_id: icon.app_user_model_id.clone(),
        order,
        pinned: order.is_some(),
        online: icon.is_visible,
        icon_path: icon.icon_path.clone(),
        icon_image_hash: icon.icon_image_hash.clone(),
    }
}

/// Build info for every currently visible icon, respecting the persisted
/// pin order (pinned first, then any remaining visible icons in enumeration
/// order).
fn build_info_list() -> Vec<TrayIconInfo> {
    let manager = get_system_tray_manager();
    let icons = manager.icons();
    let pins = current_pin_state();

    let mut result: Vec<TrayIconInfo> = Vec::with_capacity(icons.len());
    let mut seen: HashSet<String> = HashSet::new();

    for key in &pins.order {
        if let Some(icon) = icons.iter().find(|i| &i.logical_id == key)
            && seen.insert(icon.logical_id.clone())
        {
            result.push(make_info(icon, &pins));
        }
    }

    let visible: Vec<&SysTrayIcon> = icons.iter().filter(|i| i.is_visible).collect();
    for icon in &visible {
        if seen.contains(&icon.logical_id) {
            continue;
        }
        seen.insert(icon.logical_id.clone());
        result.push(make_info(icon, &pins));
    }
    for icon in icons.iter().filter(|i| !i.is_visible) {
        if seen.contains(&icon.logical_id) {
            continue;
        }
        seen.insert(icon.logical_id.clone());
        result.push(make_info(icon, &pins));
    }
    result
}

#[tauri::command(async)]
pub fn list_tray_icons() -> Vec<TrayIconInfo> {
    build_info_list()
}

#[tauri::command(async)]
pub fn list_pinned_tray_icons() -> Vec<TrayIconInfo> {
    build_info_list()
        .into_iter()
        .filter(|info| info.pinned)
        .collect()
}

#[tauri::command(async)]
pub fn get_tray_pin_state() -> TrayPinState {
    current_pin_state()
}

#[tauri::command(async)]
pub fn pin_tray_icon(logical_id: String) -> Result<()> {
    let mut pins = current_pin_state();
    if pins.order.contains(&logical_id) {
        return Ok(());
    }
    pins.order.push(logical_id);
    write_pin_state(pins.order)?;
    log::trace!("Tray pinned via semantic API");
    Ok(())
}

#[tauri::command(async)]
pub fn unpin_tray_icon(logical_id: String) -> Result<()> {
    let mut pins = current_pin_state();
    pins.order.retain(|k| *k != logical_id);
    write_pin_state(pins.order)?;
    log::trace!("Tray unpinned via semantic API");
    Ok(())
}

#[tauri::command(async)]
pub fn set_tray_pin_order(order: Vec<String>) -> Result<()> {
    let mut seen: HashSet<String> = HashSet::new();
    let dedup: Vec<String> = order
        .into_iter()
        .filter(|k| seen.insert(k.clone()))
        .collect();
    write_pin_state(dedup.clone())?;
    log::trace!("Tray pin order updated, n={}", dedup.len());
    Ok(())
}

#[tauri::command(async)]
pub fn send_tray_action(logical_id: String, action: SystrayIconAction) -> Result<()> {
    let info = list_tray_icons()
        .into_iter()
        .find(|entry| entry.logical_id == logical_id || entry.runtime_id.to_string() == logical_id);
    let Some(info) = info else {
        return Err("Icon not found".into());
    };
    get_system_tray_manager().send_action(&info.runtime_id, &action)?;
    log::trace!("Tray action forwarded: {:?} -> {}", action, info.logical_id);
    Ok(())
}
