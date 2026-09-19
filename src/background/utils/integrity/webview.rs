use base64::Engine;
use tauri::webview_version;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_shell::ShellExt;

use crate::{app::get_app_handle, error::Result};

use super::IntegrityError;

pub async fn validate_webview_runtime() -> std::result::Result<(), IntegrityError> {
    match webview_version() {
        Ok(version) => {
            let major: u32 = version
                .split('.')
                .next()
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            if major < 110 {
                Err(IntegrityError::WebviewRuntimeOutdated)
            } else {
                Ok(())
            }
        }
        Err(_) => Err(IntegrityError::WebviewRuntimeNotInstalled),
    }
}

pub fn show_not_installed_dialog(app: &tauri::AppHandle) -> Result<()> {
    let ok_pressed = app
        .dialog()
        .message(t!("runtime.not_found_description"))
        .title(t!("runtime.not_found"))
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCustom(
            t!("runtime.download").to_string(),
        ))
        .blocking_show();
    if ok_pressed {
        open_webview2_download(app)?;
    }
    Ok(())
}

pub fn show_outdated_dialog(app: &tauri::AppHandle) -> Result<()> {
    let ok_pressed = app
        .dialog()
        .message(t!("runtime.outdated_description", min_version = "110"))
        .title(t!("runtime.outdated"))
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::OkCustom(
            t!("runtime.download").to_string(),
        ))
        .blocking_show();
    if ok_pressed {
        open_webview2_download(app)?;
    }
    Ok(())
}

fn open_webview2_download(app: &tauri::AppHandle) -> Result<()> {
    let url = "https://developer.microsoft.com/en-us/microsoft-edge/webview2/?form=MA13LH#download";
    #[allow(deprecated)]
    app.shell().open(url, None)?;
    Ok(())
}

/// Try creating a webview window, tauri for some reason could panic stopping the setup hook and for some reason
/// the panic hook is not catching this so this implementation is a workaround for that.
const PROBE_ATTEMPTS: usize = 2;
const PROBE_ATTEMPT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// A single create/destroy round-trip through the event loop.
fn probe_webview_once() -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let label = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("@seelen/integrity");
        let window = tauri::WebviewWindowBuilder::new(
            get_app_handle(),
            &label,
            tauri::WebviewUrl::App("vanilla/integrity/index.html".into()),
        )
        .visible(false)
        .build()?;
        window.hwnd()?; // build could not fail so we check for the handle.
        window.destroy()?; // close the fake window
        let _ = tx.send(());
        Result::Ok(())
    });

    rx
}

pub async fn check_for_webview_optimal_state() -> std::result::Result<(), IntegrityError> {
    log::info!("Testing webview optimal state...");

    for attempt in 1..=PROBE_ATTEMPTS {
        let rx = probe_webview_once();

        tokio::select! {
            _ = rx => {
                log::info!("Webview optimal state confirmed.");
                return Ok(());
            }
            _ = tokio::time::sleep(PROBE_ATTEMPT_TIMEOUT) => {
                // The first window creation carries the WebView2 loader + environment
                // init and can legitimately take ~2.5s on its own; a second instance
                // sharing the user-data folder adds more. One retry covers real cold
                // starts while still failing on a genuinely dead environment.
                log::warn!("Webview optimal state check timed out (attempt {attempt}/{PROBE_ATTEMPTS}).");
                // Give the (possibly still running) probe thread a moment to finish its
                // create+destroy so the retried window can reuse the same label.
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
        }
    }

    log::error!("Webview optimal state check timed out after {PROBE_ATTEMPTS} attempts.");
    Err(IntegrityError::WebviewOptimalStateFailed)
}
