pub use slu_ipc::commands::WidgetCli;
use slu_ipc::commands::WidgetCommand;

use seelen_core::{
    resource::WidgetId,
    state::{WidgetDebugInfo, WidgetTriggerPayload},
};

use crate::{
    boot,
    error::Result,
    widgets::{manager::WIDGET_MANAGER, trigger_widget},
};

/// Returns `Some(json)` for data commands (`List`, `Boot`).
pub fn run(cmd: WidgetCli) -> Result<Option<String>> {
    match cmd.command {
        WidgetCommand::Trigger { widget_id } => {
            trigger_widget(WidgetTriggerPayload::new(widget_id.into()))?;
        }
        WidgetCommand::List => {
            let mut result: Vec<WidgetDebugInfo> = Vec::new();
            WIDGET_MANAGER.deployments.for_each(|(_, deployment)| {
                deployment.pods.for_each(|(_, pod)| {
                    result.push(WidgetDebugInfo {
                        label: pod.label.raw.clone(),
                        widget_id: pod.label.widget_id.to_string(),
                        monitor_id: pod.label.monitor_id.as_ref().map(|m| m.to_string()),
                        instance_id: pod.label.instance_id.map(|id| id.to_string()),
                        status: *pod.status(),
                        webview_window_id: pod.hwnd(),
                    });
                });
            });
            return Ok(Some(serde_json::to_string(&result)?));
        }
        WidgetCommand::Boot { widget_id } => {
            let value = match widget_id {
                Some(id) => {
                    let id = WidgetId::from(id.as_str());
                    let label = WIDGET_MANAGER
                        .deployments
                        .get(&id, |deployment| {
                            let mut first: Option<String> = None;
                            deployment.pods.for_each(|(label, _)| {
                                if first.is_none() {
                                    first = Some(label.raw.clone());
                                }
                            });
                            first
                        })
                        .flatten();
                    match label {
                        Some(label) => boot::snapshot(Some(&label)),
                        None => boot::snapshot(None),
                    }
                }
                None => boot::snapshot(None),
            };
            return Ok(Some(value.to_string()));
        }
        WidgetCommand::Devtools { widget_id } => {
            let id = WidgetId::from(widget_id.as_str());
            let labels: Vec<String> = WIDGET_MANAGER
                .deployments
                .get(&id, |deployment| {
                    let mut collected = Vec::new();
                    deployment.pods.for_each(|(label, _)| {
                        collected.push(label.raw.clone());
                    });
                    collected
                })
                .unwrap_or_default();
            if labels.is_empty() {
                return Err("Widget not found".into());
            }
            for raw in labels {
                crate::widgets::debug_open_dev_tools(raw)?;
            }
        }
    }
    Ok(None)
}
