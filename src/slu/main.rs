mod art;
mod resources;

use clap::Parser;
use slu_ipc::{
    AppIpc,
    commands::{AppCli, AppCommand, CommandExecutionMode, SluCliCommand},
    messages::AppMessage,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() {
    let cli = AppCli::parse();

    if cli.verbose {
        println!("Received args: {:#?}", std::env::args().collect::<Vec<_>>());
        println!("Parsed CLI: {cli:#?}");
    }

    if let Err(err) = run(cli).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run(cli: AppCli) -> Result<()> {
    // `--version` is handled by clap itself; these flags are answered by the
    // targeted main instance over IPC.
    if cli.command.is_none() {
        if !cli.boot && !cli.instances {
            return Err("No command given. Try '--help'.".into());
        }
        return send_to_main_instance(cli).await;
    }

    let mode = cli.command.as_ref().map(|c| c.execution_mode());
    match mode {
        Some(CommandExecutionMode::Direct) => process_direct(cli).await,
        _ => send_to_main_instance(cli).await,
    }
}

async fn process_direct(cli: AppCli) -> Result<()> {
    match cli.command {
        Some(AppCommand::Art(cmd)) => art::process(cmd),
        Some(AppCommand::Resource(cmd)) => resources::process(cmd).await?,
        _ => return Err("Command does not support direct execution".into()),
    }
    Ok(())
}

async fn send_to_main_instance(cli: AppCli) -> Result<()> {
    let working_dir = std::env::current_dir()?;
    let args: Vec<String> = std::env::args()
        .map(|arg| {
            if arg.starts_with("./")
                || arg.starts_with(".\\")
                || arg.starts_with("../")
                || arg.starts_with("..\\")
            {
                working_dir.join(&arg).to_string_lossy().to_string()
            } else {
                arg
            }
        })
        .collect();

    if cli.verbose {
        println!("Sending {args:#?}");
    }

    let payload = AppIpc::send_for_payload(AppMessage::Cli(args)).await?;
    if let Some(data) = payload {
        if cli.json {
            println!("{data}");
        } else {
            match serde_json::from_str::<serde_json::Value>(&data) {
                Ok(value) => println!("{}", serde_json::to_string_pretty(&value)?),
                Err(_) => println!("{data}"),
            }
        }
    }
    Ok(())
}
