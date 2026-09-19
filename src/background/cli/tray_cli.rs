use seelen_core::system_state::SystrayIconAction;
use slu_ipc::commands::TrayCli;

use crate::error::Result;
use crate::modules::system_tray::infrastructure as tray;

/// Process a tray CLI command. Returns a JSON payload string.
pub async fn process(cli: TrayCli) -> Result<Option<String>> {
    use slu_ipc::commands::TrayCommand as T;

    let payload = match cli.command {
        T::ListIcons => serde_json::to_string(&tray::list_tray_icons())?,
        T::ListPinned => serde_json::to_string(&tray::list_pinned_tray_icons())?,
        T::GetPinState => serde_json::to_string(&tray::get_tray_pin_state())?,
        T::Pin { logical_id } => {
            tray::pin_tray_icon(logical_id)?;
            serde_json::to_string(&"ok")?
        }
        T::Unpin { logical_id } => {
            tray::unpin_tray_icon(logical_id)?;
            serde_json::to_string(&"ok")?
        }
        T::SetOrder { order } => {
            tray::set_tray_pin_order(order)?;
            serde_json::to_string(&"ok")?
        }
        T::Send { logical_id, action } => {
            let parsed = parse_action(&action)?;
            tray::send_tray_action(logical_id, parsed)?;
            serde_json::to_string(&"ok")?
        }
    };
    Ok(Some(payload))
}

fn parse_action(s: &str) -> Result<SystrayIconAction> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "leftclick" => SystrayIconAction::LeftClick,
        "rightclick" => SystrayIconAction::RightClick,
        "middleclick" => SystrayIconAction::MiddleClick,
        "leftdoubleclick" => SystrayIconAction::LeftDoubleClick,
        "hoverenter" => SystrayIconAction::HoverEnter,
        "hoverleave" => SystrayIconAction::HoverLeave,
        "hovermove" => SystrayIconAction::HoverMove,
        other => return Err(format!("Unknown tray action: {other}").into()),
    })
}
