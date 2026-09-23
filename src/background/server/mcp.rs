//! Minimal stateless Model Context Protocol server (JSON-RPC 2.0 over HTTP).
//!
//! Every tool dispatches into the same native `weg_core` command core used by
//! the GUI webviews, the REST adapter and the CLI. No UI simulation, no
//! coordinate clicking: resolution happens against the Win32-backed registry.

use serde_json::{Value, json};

use crate::modules::weg_core::application as core;

pub const PROTOCOL_VERSION: &str = "2024-11-05";

fn tool(name: &str, description: &str, input: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": serde_json::from_str::<Value>(input).ok()
            .unwrap_or_else(|| json!({"type": "object"}))
    })
}

pub fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "list_windows",
            "List all managed windows with runtime ids and logical identities.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "list_applications",
            "List grouped applications with window counts.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "get_window",
            "Resolve one window by runtime id, logical identity, alias or title.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "find_windows",
            "Fuzzy search windows by identity / title / alias.",
            r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
        ),
        tool(
            "focus_window",
            "Focus a window (atomic native sequence).",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "maximize_window",
            "Maximize a window.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "restore_window",
            "Restore a window from maximized.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "minimize_window",
            "Minimize a window.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "close_window",
            "Close a window.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "focus_and_maximize_window",
            "Focus and maximize in one native sequence; returns structured result.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "move_window_to_monitor",
            "Move a window to the n-th monitor (0 based).",
            r#"{"type":"object","properties":{"identification":{"type":"string"},"monitor":{"type":"integer"}},"required":["identification","monitor"]}"#,
        ),
        tool(
            "get_window_order",
            "Manual order of a group (application key).",
            r#"{"type":"object","properties":{"app":{"type":"string"}},"required":["app"]}"#,
        ),
        tool(
            "set_window_order",
            "Persist manual order for a group.",
            r#"{"type":"object","properties":{"app":{"type":"string"},"identities":{"type":"array","items":{"type":"string"}}},"required":["app","identities"]}"#,
        ),
        tool(
            "move_window_in_group",
            "Move one identity to a position of the manual order.",
            r#"{"type":"object","properties":{"app":{"type":"string"},"identity":{"type":"string"},"to":{"type":"integer"}},"required":["app","identity","to"]}"#,
        ),
        tool(
            "get_taskbar_items",
            "Pinned taskbar item ids in manual order.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "set_taskbar_item_order",
            "Persist taskbar item order (ids).",
            r#"{"type":"object","properties":{"ids":{"type":"array","items":{"type":"string"}}},"required":["ids"]}"#,
        ),
        tool(
            "get_active_window",
            "The currently focused managed window.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "get_recent_windows",
            "Recently focused windows (MRU snapshot).",
            r#"{"type":"object","properties":{"limit":{"type":"integer"}}}"#,
        ),
        tool(
            "get_window_thumbnail",
            "Cached thumbnail for a window handle.",
            r#"{"type":"object","properties":{"hwnd":{"type":"integer"}},"required":["hwnd"]}"#,
        ),
        tool(
            "get_window_identity",
            "Resolved identity fields for one window.",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "set_window_alias",
            "Assign a short persistent alias to a logical identity.",
            r#"{"type":"object","properties":{"alias":{"type":"string"},"identity":{"type":"string"}},"required":["alias","identity"]}"#,
        ),
        // ── system tray ──────────────────────────────────────────────────
        tool(
            "list_tray_icons",
            "List notification-area icons: logical identity, runtime id, tooltip, app metadata, pin/order/online flags.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "list_pinned_tray_icons",
            "List pinned tray icons in persisted order.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "get_tray_pin_state",
            "Persisted pinned tray identity order.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "pin_tray_icon",
            "Pin a tray icon by logical identity key.",
            r#"{"type":"object","properties":{"logicalId":{"type":"string"}},"required":["logicalId"]}"#,
        ),
        tool(
            "unpin_tray_icon",
            "Unpin a tray icon by logical identity key.",
            r#"{"type":"object","properties":{"logicalId":{"type":"string"}},"required":["logicalId"]}"#,
        ),
        tool(
            "set_tray_pin_order",
            "Persist the pinned tray order.",
            r#"{"type":"object","properties":{"order":{"type":"array","items":{"type":"string"}}},"required":["order"]}"#,
        ),
        tool(
            "send_tray_action",
            "Forward a native tray action (LeftClick|RightClick|MiddleClick|LeftDoubleClick|HoverEnter|HoverLeave|HoverMove).",
            r#"{"type":"object","properties":{"logicalId":{"type":"string"},"action":{"type":"string"}},"required":["logicalId","action"]}"#,
        ),
        // ── NAI semantic kernel ─────────────────────────────────────────
        tool(
            "nai_graph",
            "Snapshot of the semantic desktop graph: windows, applications, workspaces, monitors, widgets with logical identity + capabilities.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_capabilities",
            "Capability registry descriptors (id, risk, confirmation, undo, latency class, provider priority).",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_activate",
            "Activate an object by logical identity, alias or workspace name (switches workspace if matched, otherwise focuses window).",
            r#"{"type":"object","properties":{"identification":{"type":"string"}},"required":["identification"]}"#,
        ),
        tool(
            "nai_undo_last",
            "Reverse the last reversible NAI action.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_apps",
            "NAI app registry: launcher table for browser, mail, office, voice, memory, search, qos, governor.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_launch",
            "Launch one NAI app by id (e.g. email) or displayed name (e.g. NAI E-Mail).",
            r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}"#,
        ),
        // ── NAI JSON store ────────────────────────────────────────────────
        tool(
            "nai_catalog",
            "Full JSON store catalog: schema, distributionType and curated entries.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_catalog_entry",
            "One catalog entry by stable id.",
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#,
        ),
        tool(
            "nai_install",
            "Install one catalog entry through its declared executor (winget / vsix / direct open).",
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#,
        ),
        tool(
            "nai_update",
            "Update one installed catalog entry.",
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#,
        ),
        tool(
            "nai_uninstall",
            "Uninstall one catalog entry.",
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#,
        ),
        tool(
            "nai_store_launch",
            "Launch an installed catalog entry.",
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}"#,
        ),
        // ── Activities / capsules / model gateway ─────────────────────────
        tool(
            "nai_activities",
            "Activities: persistent cognitive environments with bound window identities.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_capsules",
            "Context Capsules: semantic bundles derived from window aliases with model/QoS profile.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_gateway_models",
            "Multimodal model gateway contract: endpoints, Qwen3-VL/OpenVINO capability metadata and media.analyze_frames schema.",
            r#"{"type":"object"}"#,
        ),
        tool(
            "nai_semantic_search",
            "Qdrant-backed semantic search with bounded offline cosine fallback.",
            r#"{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},"required":["query"]}"#,
        ),
    ]
}

fn call_tool(name: &str, args: Option<&Value>) -> Result<Value, String> {
    let empty = json!({});
    let args = args.unwrap_or(&empty);
    let get = |k: &str| -> String {
        args.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let num = |k: &str| -> u32 { args.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as u32 };

    // helpers
    fn move_in_group(app: &str, identity: &str, to: usize) -> Result<Value, String> {
        let mut order = core::get_window_order(app);
        if order.is_empty() {
            order = core::window_entries()
                .into_iter()
                .filter(|e| e.application == app)
                .map(|e| {
                    e.logical_identity
                        .rsplit_once(':')
                        .map(|(_, l)| l.to_string())
                        .unwrap_or(e.logical_identity)
                })
                .collect();
        }
        let idx = order
            .iter()
            .position(|id| id.eq_ignore_ascii_case(identity))
            .ok_or("identity not found in order")?;
        let item = order.remove(idx);
        let target = to.min(order.len());
        order.insert(target, item);
        core::set_window_order(app, &order);
        serde_json::to_value(&order).map_err(|e| e.to_string())
    }

    let value: Value = match name {
        "list_windows" => serde_json::to_value(core::window_entries()).unwrap(),
        "list_applications" => {
            use std::collections::HashMap;
            let mut counts: HashMap<String, usize> = HashMap::new();
            for e in core::window_entries() {
                *counts.entry(e.application).or_default() += 1;
            }
            let apps: Vec<_> = counts
                .into_iter()
                .map(
                    |(key, window_count)| seelen_core::system_state::ApplicationEntry {
                        key,
                        window_count,
                    },
                )
                .collect();
            serde_json::to_value(apps).unwrap()
        }
        "get_window" => serde_json::to_value(core::get_window(&get("identification"))).unwrap(),
        "find_windows" => serde_json::to_value(core::find_windows(&get("query"))).unwrap(),
        "focus_window" => serde_json::to_value(
            core::focus_window(&get("identification"), "mcp").map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "maximize_window" => serde_json::to_value(
            core::maximize_window(&get("identification"), "mcp").map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "restore_window" => serde_json::to_value(
            core::restore_window(&get("identification"), "mcp").map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "minimize_window" => serde_json::to_value(
            core::minimize_window(&get("identification"), "mcp").map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "close_window" => serde_json::to_value(
            core::close_window(&get("identification"), "mcp").map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "focus_and_maximize_window" => serde_json::to_value(
            core::focus_and_maximize_window(&get("identification"), "mcp")
                .map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "move_window_to_monitor" => serde_json::to_value(
            core::move_window_to_monitor(&get("identification"), num("monitor"), "mcp")
                .map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "get_window_order" => serde_json::to_value(core::get_window_order(&get("app"))).unwrap(),
        "set_window_order" => {
            let identities: Vec<String> = args
                .get("identities")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            core::set_window_order(&get("app"), &identities);
            json!({ "success": true })
        }
        "move_window_in_group" => move_in_group(&get("app"), &get("identity"), num("to") as usize)?,
        "get_taskbar_items" => serde_json::to_value(core::get_taskbar_order()).unwrap(),
        "set_taskbar_item_order" => {
            let ids: Vec<String> = args
                .get("ids")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let mut items = crate::state::application::WEG_ITEMS_MANAGER.get();
            let mut pool: Vec<seelen_core::state::WegItem> = Vec::new();
            pool.append(&mut items.left);
            pool.append(&mut items.center);
            pool.append(&mut items.right);
            let mut by_id: std::collections::HashMap<String, seelen_core::state::WegItem> = pool
                .into_iter()
                .map(|it| (it.id().to_string(), it))
                .collect();
            let (l, c, r) = (items.left.len(), items.center.len(), items.right.len());
            let mut rebuild = |count: usize| -> Vec<seelen_core::state::WegItem> {
                let mut v = Vec::with_capacity(count);
                for id in &ids {
                    if let Some(item) = by_id.remove(id.as_str()) {
                        v.push(item);
                        if v.len() == count {
                            break;
                        }
                    }
                }
                v
            };
            let left = rebuild(l);
            let center = rebuild(c);
            let right = rebuild(r);
            items.left = left;
            items.center = center;
            items.right = right;
            crate::state::application::WEG_ITEMS_MANAGER
                .write(items)
                .map_err(|e| e.to_string())?;
            json!({ "success": true })
        }
        "get_active_window" => {
            let hwnd = crate::windows_api::window::Window::get_foregrounded().address();
            let entries = core::window_entries();
            let entry = entries
                .iter()
                .find(|e| e.hwnd == hwnd)
                .cloned()
                .or_else(|| entries.iter().find(|e| e.is_focused).cloned());
            serde_json::to_value(entry).unwrap()
        }
        "get_recent_windows" => {
            serde_json::to_value(core::get_recent_windows(num("limit").max(1))).unwrap()
        }
        "get_window_thumbnail" => {
            let hwnd = args.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as isize;
            let previews =
                crate::modules::apps::application::previews::WinPreviewManager::instance()
                    .get_previews();
            serde_json::to_value(previews.get(&hwnd)).unwrap()
        }
        "get_window_identity" => {
            let entry = core::get_window(&get("identification"));
            serde_json::to_value(entry.map(|e| {
                json!({
                    "runtimeWindowId": e.runtime_window_id,
                    "logicalIdentity": e.logical_identity,
                    "alias": e.alias,
                    "application": e.application,
                    "hwnd": e.hwnd,
                })
            }))
            .unwrap()
        }
        "set_window_alias" => {
            core::set_window_alias(&get("alias"), &get("identity"));
            json!({ "success": true })
        }
        // ── system tray (same native command core as the webviews/CLI) ───
        "list_tray_icons" => {
            serde_json::to_value(crate::modules::system_tray::infrastructure::list_tray_icons())
                .unwrap()
        }
        "list_pinned_tray_icons" => serde_json::to_value(
            crate::modules::system_tray::infrastructure::list_pinned_tray_icons(),
        )
        .unwrap(),
        "get_tray_pin_state" => {
            serde_json::to_value(crate::modules::system_tray::infrastructure::get_tray_pin_state())
                .unwrap()
        }
        "pin_tray_icon" => {
            crate::modules::system_tray::infrastructure::pin_tray_icon(get("logicalId"))
                .map_err(|e| e.to_string())?;
            json!({ "success": true })
        }
        "unpin_tray_icon" => {
            crate::modules::system_tray::infrastructure::unpin_tray_icon(get("logicalId"))
                .map_err(|e| e.to_string())?;
            json!({ "success": true })
        }
        "set_tray_pin_order" => {
            let order: Vec<String> = args
                .get("order")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            crate::modules::system_tray::infrastructure::set_tray_pin_order(order)
                .map_err(|e| e.to_string())?;
            json!({ "success": true })
        }
        "send_tray_action" => {
            let parsed =
                crate::cli::tray_cli::parse_action(&get("action")).map_err(|e| e.to_string())?;
            crate::modules::system_tray::infrastructure::send_tray_action(get("logicalId"), parsed)
                .map_err(|e| e.to_string())?;
            json!({ "success": true })
        }
        // ── NAI semantic kernel ──────────────────────────────────────────
        "nai_graph" => crate::modules::nai::graph(),
        "nai_capabilities" => serde_json::to_value(crate::modules::nai::capabilities()).unwrap(),
        "nai_activate" => serde_json::to_value(
            crate::modules::nai::activate(&get("identification")).map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "nai_undo_last" => serde_json::to_value(crate::modules::nai::undo_last()).unwrap(),
        "nai_apps" => serde_json::to_value(crate::modules::nai::apps::apps()).unwrap(),
        "nai_launch" => serde_json::to_value(
            crate::modules::nai::apps::launch(&get("name")).map_err(|e| e.to_string())?,
        )
        .unwrap(),
        // ── NAI JSON store ────────────────────────────────────────────────
        "nai_catalog" => crate::modules::nai::store::catalog().map_err(|e| e.to_string())?,
        "nai_catalog_entry" => serde_json::to_value(
            crate::modules::nai::store::catalog_entry(&get("id")).map_err(|e| e.to_string())?,
        )
        .unwrap(),
        "nai_install" => {
            crate::modules::nai::store::install(&get("id")).map_err(|e| e.to_string())?
        }
        "nai_update" => {
            crate::modules::nai::store::update(&get("id")).map_err(|e| e.to_string())?
        }
        "nai_uninstall" => {
            crate::modules::nai::store::uninstall(&get("id")).map_err(|e| e.to_string())?
        }
        "nai_store_launch" => {
            crate::modules::nai::store::launch(&get("id")).map_err(|e| e.to_string())?
        }
        // ── Activities / capsules / model gateway ─────────────────────────
        "nai_activities" => serde_json::to_value(crate::modules::nai::activities()).unwrap(),
        "nai_capsules" => serde_json::to_value(crate::modules::nai::capsules()).unwrap(),
        "nai_gateway_models" => crate::modules::nai::gateway_models(),
        "nai_semantic_search" => {
            let query = get("query");
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
            let limit = limit.max(1);
            let vector: Vec<f32> = query.bytes().map(|b| b as f32 / 255.0).collect();
            tauri::async_runtime::block_on(crate::modules::nai::semantic::search(&vector, limit))
        }
        other => return Err(format!("unknown tool: {other}")),
    };
    Ok(value)
}

/// Dispatch a raw JSON-RPC request body (stateless). Returns the json answer.
pub fn handle_jsonrpc(body: &str) -> String {
    let request: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(err) => {
            return json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("parse error: {err}") }
            })
            .to_string();
        }
    };

    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let params = request.get("params").cloned();

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "nai-os", "version": env!("CARGO_PKG_VERSION") }
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => {
            let name = params
                .as_ref()
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let args = params.as_ref().and_then(|p| p.get("arguments"));
            match call_tool(name, args) {
                Ok(value) => Ok(
                    json!({ "content": [{ "type": "json", "text": value.to_string() }], "isError": false }),
                ),
                Err(msg) => {
                    Ok(json!({ "content": [{ "type": "text", "text": msg }], "isError": true }))
                }
            }
        }
        "" => Err((-32600, "empty method".into())),
        other => Err((-32601, format!("method not found: {other}"))),
    };

    let answer = match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    };
    answer.to_string()
}
