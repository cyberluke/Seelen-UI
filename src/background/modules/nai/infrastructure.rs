use seelen_core::system_state::WindowEntry;

use slu_ipc::commands::NaiCli;

use crate::error::Result;

use super as nai;

// ===================== tauri command surface (UI) =====================

/// Snapshot of the semantic desktop graph.
#[tauri::command(async)]
pub fn nai_graph() -> Result<serde_json::Value> {
    Ok(nai::graph())
}

/// Capability registry descriptors.
#[tauri::command(async)]
pub fn nai_capabilities() -> Result<Vec<nai::CapabilityDescriptor>> {
    Ok(nai::capabilities())
}

/// Activate an object by logical identity / alias / workspace name.
#[tauri::command(async)]
pub fn nai_activate(identification: String) -> Result<serde_json::Value> {
    nai::activate(&identification)
}

/// Window-entry style projection of one node (identity + runtime fields).
#[tauri::command(async)]
pub fn nai_node(identification: String) -> Result<Option<WindowEntry>> {
    Ok(crate::modules::weg_core::application::get_window(
        &identification,
    ))
}

/// Undo the last reversible NAI action.
#[tauri::command(async)]
pub fn nai_undo_last() -> Result<Option<serde_json::Value>> {
    Ok(nai::undo_last())
}

/// NAI app registry (launcher table across all fork surfaces).
#[tauri::command(async)]
pub fn nai_apps() -> Result<Vec<nai::apps::AppDescriptor>> {
    Ok(nai::apps::apps())
}

/// Launch one registry entry by id or displayed name.
#[tauri::command(async)]
pub fn nai_launch(id: String) -> Result<serde_json::Value> {
    nai::apps::launch(&id)
}

// ===================== CLI surface (`slu nai ...`) =====================

pub fn process_cli(cli: NaiCli) -> Result<Option<String>> {
    nai::process_cli(cli)
}
