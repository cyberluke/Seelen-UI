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

/// JSON store catalog: schema + curated entries (source of truth for the
/// AI Apps store surface).
#[tauri::command(async)]
pub fn nai_catalog() -> Result<serde_json::Value> {
    nai::store::catalog()
}

/// One catalog entry by stable id.
#[tauri::command(async)]
pub fn nai_catalog_entry(id: String) -> Result<Option<serde_json::Value>> {
    nai::store::catalog_entry(&id)
}

/// Install a catalog entry through its declared executor.
#[tauri::command(async)]
pub fn nai_install(id: String) -> Result<serde_json::Value> {
    nai::store::install(&id)
}

/// Update an installed catalog entry.
#[tauri::command(async)]
pub fn nai_update(id: String) -> Result<serde_json::Value> {
    nai::store::update(&id)
}

/// Uninstall a catalog entry.
#[tauri::command(async)]
pub fn nai_uninstall(id: String) -> Result<serde_json::Value> {
    nai::store::uninstall(&id)
}

/// Launch an installed catalog entry.
#[tauri::command(async)]
pub fn nai_store_launch(id: String) -> Result<serde_json::Value> {
    nai::store::launch(&id)
}

/// Activities: persistent cognitive environments.
#[tauri::command(async)]
pub fn nai_activities() -> Result<Vec<nai::Activity>> {
    Ok(nai::activities())
}

/// Context Capsules: compact semantic bundles bound to window aliases.
#[tauri::command(async)]
pub fn nai_capsules() -> Result<Vec<nai::ContextCapsule>> {
    Ok(nai::capsules())
}

/// Multimodal model gateway contract (Qwen3-VL/OpenVINO binding contract).
#[tauri::command(async)]
pub fn nai_gateway_models() -> Result<serde_json::Value> {
    Ok(nai::gateway_models())
}

/// Qdrant-backed semantic search with bounded offline cosine fallback.
#[tauri::command(async)]
pub async fn nai_semantic_search(query: String, limit: Option<usize>) -> Result<serde_json::Value> {
    let vector: Vec<f32> = query.bytes().map(|b| b as f32 / 255.0).collect();
    Ok(nai::semantic::search(&vector, limit.unwrap_or(5).max(1)).await)
}

// ===================== Shorts / media engine =====================

/// Shorts candidate search through the YouTube Data API.
#[tauri::command(async)]
pub async fn nai_shorts_search(query: String, limit: Option<usize>) -> Result<serde_json::Value> {
    nai::shorts::search(&query, limit.unwrap_or(10))
        .await
        .map_err(|e| e.into())
}

/// Enqueue a Shorts candidate with its queue reason.
#[tauri::command(async)]
pub fn nai_shorts_enqueue(video_id: String, reason: Option<String>) -> serde_json::Value {
    nai::shorts::enqueue(&video_id, &reason.unwrap_or_default())
}

/// Next item of the vertical queue.
#[tauri::command(async)]
pub fn nai_shorts_next() -> serde_json::Value {
    nai::shorts::next()
}

/// Full queue state.
#[tauri::command(async)]
pub fn nai_shorts_queue() -> serde_json::Value {
    nai::shorts::queue_state()
}

/// PiP shell-object contract.
#[tauri::command(async)]
pub fn nai_pip_contract() -> serde_json::Value {
    nai::shorts::pip_contract()
}

// ===================== Social fabric =====================

/// Mastodon home timeline.
#[tauri::command(async)]
pub async fn nai_social_timeline(limit: Option<usize>) -> Result<serde_json::Value> {
    nai::fabric::timeline_home(limit.unwrap_or(10))
        .await
        .map_err(|e| e.into())
}

/// Mastodon notifications.
#[tauri::command(async)]
pub async fn nai_social_notifications(limit: Option<usize>) -> Result<serde_json::Value> {
    nai::fabric::notifications(limit.unwrap_or(10))
        .await
        .map_err(|e| e.into())
}

/// Mastodon local timeline.
#[tauri::command(async)]
pub async fn nai_social_local(limit: Option<usize>) -> Result<serde_json::Value> {
    nai::fabric::timeline_local(limit.unwrap_or(10))
        .await
        .map_err(|e| e.into())
}

/// Compose a Mastodon status.
#[tauri::command(async)]
pub async fn nai_social_compose(status: String) -> Result<serde_json::Value> {
    nai::fabric::compose(&status).await.map_err(|e| e.into())
}

/// v271 agentic chat with typed context.
#[tauri::command(async)]
pub async fn nai_v271_chat(prompt: String) -> Result<serde_json::Value> {
    nai::fabric::v271_chat(&prompt).await.map_err(|e| e.into())
}

/// Upsert a semantic vector (mirrored to the offline snapshot).
#[tauri::command(async)]
pub async fn nai_semantic_upsert(id: String, vector: Vec<f32>) -> Result<serde_json::Value> {
    Ok(nai::semantic::upsert(&id, &vector).await)
}

// ===================== CLI surface (`slu nai ...`) =====================

pub fn process_cli(cli: NaiCli) -> Result<Option<String>> {
    nai::process_cli(cli)
}
