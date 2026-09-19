use seelen_core::system_state::{
    AutomationMetrics, PreviewLatency, TraceFrame, WindowActionResult, WindowEntry,
};

use crate::error::Result;

use super::application as core;

// ===================== tauri command surface (UI) =====================
// Thin wrappers over the native window command core. All control planes
// (UI, CLI, MCP, REST) funnel into the same `core::` functions.

#[tauri::command(async)]
pub fn weg_get_window_entries() -> Result<Vec<WindowEntry>> {
    Ok(core::task_switcher_entries())
}

#[tauri::command(async)]
pub fn weg_get_window(identification: String) -> Result<Option<WindowEntry>> {
    Ok(core::get_window(&identification))
}

#[tauri::command(async)]
pub fn weg_find_windows(query: String) -> Result<Vec<WindowEntry>> {
    Ok(core::find_windows(&query))
}

#[tauri::command(async)]
pub fn weg_focus_window(identification: String) -> Result<WindowActionResult> {
    core::focus_window(&identification, "ui")
}

#[tauri::command(async)]
pub fn weg_maximize_window(identification: String) -> Result<WindowActionResult> {
    core::maximize_window(&identification, "ui")
}

#[tauri::command(async)]
pub fn weg_restore_window(identification: String) -> Result<WindowActionResult> {
    core::restore_window(&identification, "ui")
}

#[tauri::command(async)]
pub fn weg_minimize_window(identification: String) -> Result<WindowActionResult> {
    core::minimize_window(&identification, "ui")
}

#[tauri::command(async)]
pub fn weg_focus_and_maximize_window(identification: String) -> Result<WindowActionResult> {
    core::focus_and_maximize_window(&identification, "ui")
}

#[tauri::command(async)]
pub fn weg_get_window_order(app: String) -> Result<Vec<String>> {
    Ok(core::get_window_order(&app))
}

#[tauri::command(async)]
pub fn weg_set_window_order(app: String, identities: Vec<String>) -> Result<()> {
    core::set_window_order(&app, &identities);
    Ok(())
}

#[tauri::command(async)]
pub fn weg_set_window_alias(alias: String, identity: String) -> Result<()> {
    core::set_window_alias(&alias, &identity);
    Ok(())
}

#[tauri::command(async)]
pub fn weg_get_recent_windows(limit: Option<u32>) -> Result<Vec<WindowEntry>> {
    Ok(core::get_recent_windows(limit.unwrap_or(10)))
}

#[tauri::command(async)]
pub fn weg_report_preview_latency(latency: PreviewLatency) -> Result<()> {
    core::report_preview_latency(&latency);
    Ok(())
}

#[tauri::command(async)]
pub fn weg_get_automation_metrics() -> Result<AutomationMetrics> {
    Ok(core::automation_metrics())
}

#[tauri::command(async)]
pub fn weg_get_trace() -> Result<Vec<TraceFrame>> {
    Ok(crate::modules::weg_core::application::get_trace())
}

/// Hide only the `@seelen/weg-preview` webview (grouped window popup).
///
/// The preview is kept warm: the window is hidden, not destroyed, not reloaded,
/// so the next trigger paints immediately from the still-valid model. The main
/// `@seelen/weg` webview is never touched by this command. Calling it while the
/// preview is already hidden is a no-op.
#[tauri::command(async)]
pub fn weg_hide_preview() -> Result<()> {
    use tauri::Manager;

    use crate::widgets::webview::WidgetWebviewLabel;

    let app = crate::app::get_app_handle();
    // `@seelen/weg-preview` is a `Single` instance widget: the raw label is
    // the plain widget id encoded exactly like the loader created it.
    let label = WidgetWebviewLabel::new(
        &seelen_core::resource::WidgetId::from("@seelen/weg-preview"),
        None,
        None,
    );
    if let Some(window) = app.get_webview_window(&label.raw) {
        let _ = window.hide();
        log::trace!("preview lifecycle: preview-hidden via WegHidePreview");
    }
    Ok(())
}
