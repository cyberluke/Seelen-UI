mod debugger;
mod self_pipe;
pub mod shortcuts;
mod svc_pipe;
pub mod tray_cli;
mod uri;

pub use self_pipe::SelfPipe;
pub use svc_pipe::ServicePipe;

use std::sync::atomic::Ordering;

use slu_ipc::{
    AppIpc,
    commands::{AppCommand, RuntimeCommand},
    messages::AppMessage,
};

use crate::{
    boot,
    error::Result,
    resources::cli as resources_cli,
    virtual_desktops::cli as vd_cli,
    widgets::{
        cli as widget_cli, popups::cli as popups_cli, show_settings,
        task_switcher::cli as task_switcher_cli, wallpaper_manager::cli as wallpaper_cli,
        weg::cli as weg_cli, window_manager::cli as wm_cli,
    },
};

/// WARNING: NAI-OS.exe CLI commands are deprecated.
///
/// Use `slu` instead.
#[derive(Debug, clap::Parser)]
#[command(version, name = "NAI-OS")]
struct MainCli {
    #[arg(long, default_value_t)]
    silent: bool,
    #[arg(long, default_value_t)]
    verbose: bool,
    #[arg(long, default_value_t)]
    json: bool,
    /// Path or URI to open (e.g. from the Windows protocol handler).
    uri: Option<String>,
}

/// Called at startup by the main executable. If a URI is present it is forwarded
/// to the running instance via IPC and the process exits; otherwise returns Ok(())
/// so normal app startup continues.
pub async fn handle_console_client() -> Result<()> {
    use clap::Parser;
    let cli = match MainCli::try_parse() {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    if cli.silent {
        crate::SILENT.store(true, Ordering::SeqCst);
    }
    if cli.verbose {
        crate::VERBOSE.store(true, Ordering::SeqCst);
        println!("Received args: {:#?}", std::env::args().collect::<Vec<_>>());
        println!("Parsed CLI: {cli:#?}");
    }

    if let Some(uri) = cli.uri {
        AppIpc::send(AppMessage::OpenUri(uri))
            .await
            .map_err(|_| "Can't establish connection, ensure NAI OS is running.")?;
        std::process::exit(0);
    }

    Ok(())
}

/// Returns `Some(json)` when the command produced a structured payload.
pub async fn process_app_command(cmd: AppCommand) -> Result<Option<String>> {
    match cmd {
        AppCommand::Settings => {
            show_settings()?;
        }
        AppCommand::VirtualDesk(command) => {
            vd_cli::process(command)?;
        }
        AppCommand::Debugger(command) => {
            debugger::process(command)?;
        }
        AppCommand::WindowManager(command) => {
            wm_cli::process(command)?;
        }
        AppCommand::Weg(command) => {
            return weg_cli::process(command);
        }
        AppCommand::Widget(command) => {
            if let Some(payload) = widget_cli::run(command)? {
                return Ok(Some(payload));
            }
        }
        AppCommand::Resource(command) => {
            resources_cli::process(command).await?;
        }
        AppCommand::Popup(command) => {
            popups_cli::process(command)?;
        }
        AppCommand::TaskSwitcher(command) => {
            task_switcher_cli::process(command)?;
        }
        AppCommand::Wallpaper(command) => {
            wallpaper_cli::process(command)?;
        }
        AppCommand::ToggleShortcutsPause => {
            shortcuts::toggle_pause()?;
        }
        AppCommand::Tray(cli) => {
            return tray_cli::process(cli).await;
        }
        AppCommand::Runtime(cli) => {
            let value = match cli.subcommand {
                RuntimeCommand::Instance => boot::instance_info(),
                RuntimeCommand::Provenance => boot::provenance(),
            };
            return Ok(Some(value.to_string()));
        }
        AppCommand::Nai(cli) => {
            return crate::modules::nai::infrastructure::process_cli(cli);
        }
        _ => {
            return Err("Command does not support instance execution".into());
        }
    }
    Ok(None)
}
