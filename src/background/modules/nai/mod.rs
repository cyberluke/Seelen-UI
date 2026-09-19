//! NAI semantic desktop kernel.
//!
//! One semantic model consumed by every surface (Weg, Task Switcher, Overview,
//! CLI, MCP, REST). Derived on demand from the authoritative native registries
//! (`weg_core`, monitors, virtual desktops, widget pods) plus the persistent
//! logical identities. UI stores are views, never the source of truth.

pub mod apps;
pub mod infrastructure;

use std::sync::{LazyLock, Mutex};

use seelen_core::{state::WorkspaceId, system_state::WindowEntry};
use serde::Serialize;

use slu_ipc::commands::{NaiCli, NaiCommand};

use crate::error::Result;

// ============================ domain ============================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum NaiNodeKind {
    Application,
    Window,
    Workspace,
    Monitor,
    Widget,
    TrayIcon,
}

/// A directed edge in the semantic desktop graph.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NaiEdge {
    pub kind: &'static str,
    pub to: String,
}

/// A semantic object. `id` is the logical identity (owned across restarts);
/// runtime handles stay as plain reboundable fields, never as identity.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NaiNode {
    pub id: String,
    pub kind: NaiNodeKind,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    pub capabilities: Vec<&'static str>,
    pub relations: Vec<NaiEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<NaiRuntime>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NaiRuntime {
    pub hwnd: Option<isize>,
    pub monitor: Option<String>,
    pub focused: bool,
}

/// A capability descriptor: what the capability does, its risk class and the
/// preferred provider chain. The router selects by capability, not brand.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityDescriptor {
    pub id: &'static str,
    pub risk: &'static str,
    pub interactive_confirmation: &'static str,
    pub undo: &'static str,
    pub latency_class: &'static str,
    pub provider_priority: &'static [&'static str],
}

pub const WINDOW_CAPABILITIES: [&str; 6] = [
    "window.focus",
    "window.maximize",
    "window.restore",
    "window.minimize",
    "window.close",
    "window.move_to_monitor",
];

/// Static machine-local capability registry.
pub fn capabilities() -> Vec<CapabilityDescriptor> {
    let c = |id, risk, confirm, undo, latency| CapabilityDescriptor {
        id,
        risk,
        interactive_confirmation: confirm,
        undo,
        latency_class: latency,
        provider_priority: &["seelen"],
    };
    vec![
        c("window.focus", "reversible", "none", "full", "interactive"),
        c(
            "window.maximize",
            "reversible",
            "none",
            "full",
            "interactive",
        ),
        c(
            "window.restore",
            "reversible",
            "none",
            "full",
            "interactive",
        ),
        c(
            "window.minimize",
            "reversible",
            "none",
            "full",
            "interactive",
        ),
        c(
            "window.close",
            "quasi_reversible",
            "none",
            "limited",
            "interactive",
        ),
        c(
            "window.move_to_monitor",
            "reversible",
            "none",
            "full",
            "interactive",
        ),
        c(
            "workspace.activate",
            "reversible",
            "none",
            "full",
            "interactive",
        ),
        c("tray.activate", "reversible", "none", "none", "interactive"),
        c(
            "media.playpause",
            "reversible",
            "none",
            "none",
            "interactive",
        ),
        c(
            "shell.settings",
            "reversible",
            "none",
            "none",
            "interactive",
        ),
        c(
            "widget.trigger",
            "reversible",
            "none",
            "none",
            "interactive",
        ),
    ]
}

// ============================ graph ============================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NaiGraphValue {
    pub nodes: Vec<NaiNode>,
    pub capabilities: Vec<CapabilityDescriptor>,
}

/// Build the current semantic snapshot of the desktop.
pub fn graph() -> serde_json::Value {
    let mut nodes: Vec<NaiNode> = Vec::new();

    let entries: Vec<WindowEntry> = crate::modules::weg_core::application::window_entries();

    // applications (grouped, ordered by first appearance)
    let mut seen_apps: Vec<String> = Vec::new();
    for entry in &entries {
        if !seen_apps.contains(&entry.application) {
            seen_apps.push(entry.application.clone());
        }
    }
    for app in &seen_apps {
        let count = entries.iter().filter(|e| &e.application == app).count();
        let children: Vec<NaiEdge> = entries
            .iter()
            .filter(|e| &e.application == app)
            .map(|e| NaiEdge {
                kind: "contains",
                to: e.logical_identity.clone(),
            })
            .collect();
        nodes.push(NaiNode {
            id: app.clone(),
            kind: NaiNodeKind::Application,
            title: app.clone(),
            subtitle: Some(format!("{count} windows")),
            capabilities: vec![],
            relations: children,
            runtime: None,
        });
    }

    // windows
    for entry in &entries {
        nodes.push(NaiNode {
            id: entry.logical_identity.clone(),
            kind: NaiNodeKind::Window,
            title: entry.display_title.clone(),
            subtitle: entry.alias.clone(),
            capabilities: WINDOW_CAPABILITIES.to_vec(),
            relations: vec![NaiEdge {
                kind: "belongs_to",
                to: entry.application.clone(),
            }],
            runtime: Some(NaiRuntime {
                hwnd: Some(entry.hwnd),
                monitor: Some(entry.monitor.to_string()),
                focused: entry.is_focused,
            }),
        });
    }

    // workspaces (per monitor grid)
    let vds = crate::virtual_desktops::handlers::get_virtual_desktops();
    #[allow(clippy::for_kv_map)]
    for (monitor_id, monitor) in vds.monitors.iter() {
        for row in monitor.workspaces.rows() {
            for workspace in row {
                let is_active = monitor.active_workspace_id() == &workspace.id;
                let mut relations = vec![NaiEdge {
                    kind: "visible_on",
                    to: format!("monitor:{monitor_id}"),
                }];
                for hwnd in &workspace.windows {
                    // link contained windows by runtime id when resolvable
                    if let Some(entry) = entries.iter().find(|e| &e.hwnd == hwnd) {
                        relations.push(NaiEdge {
                            kind: "contains",
                            to: entry.logical_identity.clone(),
                        });
                    }
                }
                nodes.push(NaiNode {
                    id: workspace.id.to_string(),
                    kind: NaiNodeKind::Workspace,
                    title: workspace
                        .name
                        .clone()
                        .unwrap_or_else(|| workspace.id.to_string()),
                    subtitle: Some(if is_active {
                        "active".to_string()
                    } else {
                        "inactive".to_string()
                    }),
                    capabilities: vec!["workspace.activate"],
                    relations,
                    runtime: None,
                });
            }
        }
    }

    // monitors
    for monitor_id in crate::modules::monitors::MonitorManager::instance().get_cached_ids() {
        nodes.push(NaiNode {
            id: format!("monitor:{monitor_id}"),
            kind: NaiNodeKind::Monitor,
            title: monitor_id.to_string(),
            subtitle: None,
            capabilities: vec!["window.move_to_monitor"],
            relations: vec![],
            runtime: None,
        });
    }

    // shell widget pods
    crate::widgets::manager::WIDGET_MANAGER
        .deployments
        .for_each(|(_, deployment)| {
            deployment.pods.for_each(|(label, pod)| {
                nodes.push(NaiNode {
                    id: label.decoded.clone(),
                    kind: NaiNodeKind::Widget,
                    title: label.decoded.clone(),
                    subtitle: Some(format!("{:?}", pod.status())),
                    capabilities: vec!["widget.trigger"],
                    relations: vec![],
                    runtime: None,
                });
            });
        });

    serde_json::to_value(NaiGraphValue {
        nodes,
        capabilities: capabilities(),
    })
    .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }))
}

// ============================ activation ============================

/// Resolve one object and activate it via the native command core.
pub fn activate(identification: &str) -> Result<serde_json::Value> {
    // workspaces first (name or id)
    let vds = crate::virtual_desktops::handlers::get_virtual_desktops();
    for monitor in vds.monitors.values() {
        for row in monitor.workspaces.rows() {
            for workspace in row {
                let by_name = workspace
                    .name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(identification));
                let by_id = workspace.id.to_string() == identification;
                if by_name || by_id {
                    crate::virtual_desktops::handlers::switch_workspace(WorkspaceId(
                        workspace.id.0.clone(),
                    ))?;
                    push_undo(NaiUndo {
                        kind: "workspace.activate".into(),
                        previous: Some(monitor.active_workspace_id().to_string()),
                        current: Some(workspace.id.to_string()),
                    });
                    return Ok(serde_json::json!({
                        "activated": { "id": workspace.id.to_string(), "kind": "workspace" },
                        "success": true,
                    }));
                }
            }
        }
    }

    // windows through the native command core
    let entries = crate::modules::weg_core::application::window_entries();
    let previous_focus = entries
        .iter()
        .find(|e| e.is_focused)
        .map(|e| e.logical_identity.clone());
    let action = crate::modules::weg_core::application::focus_window(identification, "nai")?;
    push_undo(NaiUndo {
        kind: "window.focus".into(),
        previous: previous_focus,
        current: Some(action.identity.clone()),
    });
    Ok(serde_json::json!({ "activated": action }))
}

// ============================ undo ============================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NaiUndo {
    pub kind: String,
    pub previous: Option<String>,
    pub current: Option<String>,
}

static UNDO_STACK: LazyLock<Mutex<Vec<NaiUndo>>> = LazyLock::new(|| Mutex::new(Vec::new()));

const UNDO_CAPACITY: usize = 50;

fn push_undo(record: NaiUndo) {
    let mut stack = UNDO_STACK.lock().unwrap_or_else(|e| e.into_inner());
    if stack.len() == UNDO_CAPACITY {
        stack.remove(0);
    }
    stack.push(record);
}

/// Reverse the last reversible NAI action.
pub fn undo_last() -> Option<serde_json::Value> {
    let record = UNDO_STACK.lock().unwrap_or_else(|e| e.into_inner()).pop()?;
    match record.kind.as_str() {
        "window.focus" => {
            if let Some(prev) = &record.previous {
                // best effort re-focus of the previously focused window
                crate::modules::weg_core::application::focus_window(prev, "nai-undo").ok()?;
            }
            Some(serde_json::to_value(&record).ok()?)
        }
        "workspace.activate" => {
            if let Some(prev) = &record.previous {
                let id: WorkspaceId = prev.clone().into();
                crate::virtual_desktops::handlers::switch_workspace(id).ok()?;
            }
            Some(serde_json::to_value(&record).ok()?)
        }
        _ => Some(serde_json::to_value(&record).ok()?),
    }
}

// ============================ CLI bridge ============================

/// `slu nai ...` funnels into the same native command core; the UI, REST and
/// MCP control planes stay identical.
pub fn process_cli(cli: NaiCli) -> Result<Option<String>> {
    let value = match cli.subcommand {
        NaiCommand::Graph => graph(),
        NaiCommand::Capabilities => serde_json::to_value(capabilities()).unwrap(),
        NaiCommand::Activate { identification } => activate(&identification)?,
        NaiCommand::Undo => undo_last().unwrap_or(serde_json::Value::Null),
        NaiCommand::Trace => {
            serde_json::to_value(crate::modules::weg_core::application::get_trace()).unwrap()
        }
        NaiCommand::Apps => serde_json::to_value(apps::apps()).unwrap(),
        NaiCommand::Launch { name } => apps::launch(&name)?,
    };
    Ok(Some(value.to_string()))
}
