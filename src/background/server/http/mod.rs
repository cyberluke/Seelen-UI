use salvo::prelude::*;

use crate::resources::RESOURCES;

/// Fallback port ("SLU" interpreted as a base-36 number) used when settings
/// were not loaded yet.
pub const LOCAL_API_PORT: u16 = 37_074;

const SCALAR_HTML: &str = include_str!("./scalar.html");

/// Serves the Scalar API reference page. We render this ourselves instead of using
/// `salvo_oapi::scalar::Scalar` so we can load the official Scalar CDN script directly
/// and freely configure it (theme, GitHub link, etc.) in `scalar.html`.
#[handler]
async fn scalar_docs(res: &mut Response) {
    res.render(Text::Html(SCALAR_HTML));
}

#[derive(serde::Serialize, salvo::oapi::ToSchema)]
struct PingResponse {
    name: &'static str,
    version: &'static str,
}

/// Ping
///
/// Returns the name and version of the application.
#[endpoint]
async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Themes
///
/// Returns a list of all available themes.
#[endpoint(tag("Resources"))]
async fn themes() -> Json<Vec<std::sync::Arc<seelen_core::state::Theme>>> {
    Json(RESOURCES.themes())
}

/// Icon Packs
///
/// Returns a list of all available icon packs.
#[endpoint(tag("Resources"))]
async fn icon_packs() -> Json<Vec<std::sync::Arc<seelen_core::state::IconPack>>> {
    Json(RESOURCES.icon_packs())
}

/// Theme Tokens
///
/// Returns the design tokens resolved from all currently enabled themes.
/// Enabled themes are merged in activation order, so themes activated later
/// take priority over the ones activated before them.
#[endpoint(tag("Resources"))]
async fn theme_tokens(
    // Whether to resolve dark-mode tokens. Themes without `tokensDark` fall back to `tokens`.
    dark: salvo::oapi::extract::QueryParam<bool, false>,
) -> Json<seelen_core::state::ThemeTokens> {
    let dark = dark.into_inner().unwrap_or(false);
    let state = crate::state::application::FULL_STATE.load();

    let themes_by_id: std::collections::HashMap<_, _> = RESOURCES
        .themes()
        .into_iter()
        .map(|theme| (theme.id.clone(), theme))
        .collect();

    let tokens = state
        .settings
        .active_themes
        .iter()
        .filter_map(|id| themes_by_id.get(id))
        .filter_map(|theme| {
            if dark {
                theme.tokens_dark.as_ref().or(theme.tokens.as_ref())
            } else {
                theme.tokens.as_ref()
            }
        })
        .cloned()
        .fold(seelen_core::state::ThemeTokens::default(), |acc, tokens| {
            acc.merge(tokens)
        });

    Json(tokens)
}

// ============================ v1 control plane ============================
// Thin JSON adapters over the native WindowCommandCore. Same implementations
// as the GUI webviews, the MCP server and the CLI; no duplicated logic.

fn write_json<T: serde::Serialize + Send + Sync + 'static>(res: &mut Response, value: &T) {
    res.render(Json(value));
}

fn write_action(
    res: &mut Response,
    result: crate::error::Result<seelen_core::system_state::WindowActionResult>,
) {
    match result {
        Ok(action) => res.render(Json(action)),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err.to_string()));
        }
    }
}

/// GET /v1/windows
#[handler]
async fn v1_windows(res: &mut Response) {
    write_json(
        res,
        &crate::modules::weg_core::application::window_entries(),
    );
}

/// GET /v1/apps
#[handler]
async fn v1_apps(res: &mut Response) {
    use std::collections::HashMap;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in crate::modules::weg_core::application::window_entries() {
        *counts.entry(e.application).or_default() += 1;
    }
    let apps: Vec<_> = counts
        .into_iter()
        .map(
            |(key, window_count)| seelen_core::system_state::ApplicationEntry { key, window_count },
        )
        .collect();
    write_json(res, &apps);
}

/// GET /v1/windows/{id}
#[handler]
async fn v1_window(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    match crate::modules::weg_core::application::get_window(&id) {
        Some(entry) => res.render(Json(entry)),
        None => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Text::Plain("window not found"));
        }
    }
}

fn id_from(req: &mut Request) -> String {
    req.param::<String>("id").unwrap_or_default()
}

/// POST /v1/windows/{id}/focus
#[handler]
async fn v1_window_focus(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::focus_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/maximize
#[handler]
async fn v1_window_maximize(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::maximize_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/restore
#[handler]
async fn v1_window_restore(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::restore_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/minimize
#[handler]
async fn v1_window_minimize(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::minimize_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/close
#[handler]
async fn v1_window_close(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::close_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/focus-maximize (atomic)
#[handler]
async fn v1_window_focus_maximize(req: &mut Request, res: &mut Response) {
    write_action(
        res,
        crate::modules::weg_core::application::focus_and_maximize_window(&id_from(req), "rest"),
    );
}

/// POST /v1/windows/{id}/monitors/{index}
#[handler]
async fn v1_window_monitor(req: &mut Request, res: &mut Response) {
    let index = req.param::<u32>("index").unwrap_or(0);
    write_action(
        res,
        crate::modules::weg_core::application::move_window_to_monitor(&id_from(req), index, "rest"),
    );
}

/// GET /v1/groups/{appId}/order
#[handler]
async fn v1_group_order(req: &mut Request, res: &mut Response) {
    let app = req.param::<String>("appId").unwrap_or_default();
    write_json(
        res,
        &crate::modules::weg_core::application::get_window_order(&app),
    );
}

/// PATCH /v1/groups/{appId}/order  body: ["idA","idB",...]
#[handler]
async fn v1_set_group_order(req: &mut Request, res: &mut Response) {
    let app = req.param::<String>("appId").unwrap_or_default();
    let body = match req.payload().await {
        Ok(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Err(_) => String::new(),
    };
    match serde_json::from_str::<Vec<String>>(&body) {
        Ok(identities) => {
            crate::modules::weg_core::application::set_window_order(&app, &identities);
            res.status_code(StatusCode::NO_CONTENT);
        }
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(format!("invalid body: {err}")));
        }
    }
}

/// GET /v1/taskbar/items
#[handler]
async fn v1_taskbar_items(res: &mut Response) {
    write_json(
        res,
        &crate::modules::weg_core::application::get_taskbar_order(),
    );
}

/// GET /v1/metrics
#[handler]
async fn v1_metrics(res: &mut Response) {
    write_json(
        res,
        &crate::modules::weg_core::application::automation_metrics(),
    );
}

/// GET /v1/trace
#[handler]
async fn v1_trace(res: &mut Response) {
    write_json(res, &crate::modules::weg_core::application::get_trace());
}

// ============================ v1 NAI semantic plane ============================
// Thin JSON adapters over the same native NAI kernel used by the webviews,
// MCP and CLI; no duplicated logic.

fn write_result(res: &mut Response, result: crate::error::Result<serde_json::Value>) {
    match result {
        Ok(value) => res.render(Json(value)),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err.to_string()));
        }
    }
}

/// GET /v1/nai/graph
#[handler]
async fn v1_nai_graph(res: &mut Response) {
    write_json(res, &crate::modules::nai::graph());
}

/// GET /v1/nai/capabilities
#[handler]
async fn v1_nai_capabilities(res: &mut Response) {
    write_json(res, &crate::modules::nai::capabilities());
}

/// POST /v1/nai/activate/{id}
#[handler]
async fn v1_nai_activate(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::activate(&id));
}

/// POST /v1/nai/undo
#[handler]
async fn v1_nai_undo(res: &mut Response) {
    write_json(res, &crate::modules::nai::undo_last());
}

/// GET /v1/nai/activities
#[handler]
async fn v1_nai_activities(res: &mut Response) {
    write_json(res, &crate::modules::nai::activities());
}

/// GET /v1/nai/capsules
#[handler]
async fn v1_nai_capsules(res: &mut Response) {
    write_json(res, &crate::modules::nai::capsules());
}

/// GET /v1/nai/gateway/models
#[handler]
async fn v1_nai_gateway_models(res: &mut Response) {
    write_json(res, &crate::modules::nai::gateway_models());
}

/// GET /v1/nai/telemetry
#[handler]
async fn v1_nai_telemetry(res: &mut Response) {
    write_json(res, &crate::modules::nai::telemetry::sample());
}

/// GET /v1/nai/semantic/{query}  (optional ?limit=)
#[handler]
async fn v1_nai_semantic(req: &mut Request, res: &mut Response) {
    let query = req.param::<String>("query").unwrap_or_default();
    let limit = req.query::<usize>("limit").unwrap_or(5).max(1);
    let vector: Vec<f32> = query.bytes().map(|b| b as f32 / 255.0).collect();
    let value = crate::modules::nai::semantic::search(&vector, limit).await;
    write_json(res, &value);
}

/// GET /v1/nai/apps
#[handler]
async fn v1_nai_apps(res: &mut Response) {
    write_json(res, &crate::modules::nai::apps::apps());
}

/// POST /v1/nai/apps/{id}/launch
#[handler]
async fn v1_nai_app_launch(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::apps::launch(&id));
}

// ── Media / Shorts engine ───────────────────────────────────────────────────

/// GET /v1/nai/shorts/{query}  (optional ?limit=)
#[handler]
async fn v1_nai_shorts(req: &mut Request, res: &mut Response) {
    let query = req.param::<String>("query").unwrap_or_default();
    let limit = req.query::<usize>("limit").unwrap_or(10);
    match crate::modules::nai::shorts::search(&query, limit).await {
        Ok(value) => write_json(res, &value),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err));
        }
    }
}

/// POST /v1/nai/shorts/enqueue/{id}  (optional ?reason=)
#[handler]
async fn v1_nai_shorts_enqueue(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    let reason = req.query::<String>("reason").unwrap_or_default();
    let value = crate::modules::nai::shorts::enqueue(&id, &reason);
    write_json(res, &value);
}

/// POST /v1/nai/shorts/next
#[handler]
async fn v1_nai_shorts_next(res: &mut Response) {
    write_json(res, &crate::modules::nai::shorts::next());
}

/// GET /v1/nai/shorts/queue
#[handler]
async fn v1_nai_shorts_queue(res: &mut Response) {
    write_json(res, &crate::modules::nai::shorts::queue_state());
}

/// GET /v1/nai/pip
#[handler]
async fn v1_nai_pip(res: &mut Response) {
    write_json(res, &crate::modules::nai::shorts::pip_contract());
}

// ── Social fabric (Mastodon / v271) ─────────────────────────────────────────

/// GET /v1/nai/social/{lane}  lane = home|local|notifications (optional ?limit=)
#[handler]
async fn v1_nai_social(req: &mut Request, res: &mut Response) {
    let lane = req.param::<String>("lane").unwrap_or_default();
    let limit = req.query::<usize>("limit").unwrap_or(10);
    let result = match lane.as_str() {
        "home" => crate::modules::nai::fabric::timeline_home(limit).await,
        "local" => crate::modules::nai::fabric::timeline_local(limit).await,
        "notifications" => crate::modules::nai::fabric::notifications(limit).await,
        other => Err(format!("unknown social lane: {other}")),
    };
    match result {
        Ok(value) => write_json(res, &value),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err));
        }
    }
}

/// POST /v1/nai/social/compose  (?status=)
#[handler]
async fn v1_nai_social_compose(req: &mut Request, res: &mut Response) {
    let status = req.query::<String>("status").unwrap_or_default();
    match crate::modules::nai::fabric::compose(&status).await {
        Ok(value) => write_json(res, &value),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err));
        }
    }
}

/// POST /v1/nai/v271/{prompt}
#[handler]
async fn v1_nai_v271(req: &mut Request, res: &mut Response) {
    let prompt = req.param::<String>("prompt").unwrap_or_default();
    match crate::modules::nai::fabric::v271_chat(&prompt).await {
        Ok(value) => write_json(res, &value),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err));
        }
    }
}

/// GET /v1/nai/semantic-upsert/{id}  (?v=1,2,3)
#[handler]
async fn v1_nai_semantic_upsert(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    let raw = req.query::<String>("v").unwrap_or_default();
    let vector: Vec<f32> = raw
        .split(',')
        .filter_map(|part| part.trim().parse::<f32>().ok())
        .collect();
    let value = crate::modules::nai::semantic::upsert(&id, &vector).await;
    write_json(res, &value);
}

/// GET /v1/store/catalog
#[handler]
async fn v1_store_catalog(res: &mut Response) {
    write_result(res, crate::modules::nai::store::catalog());
}

/// GET /v1/store/catalog/{id}
#[handler]
async fn v1_store_catalog_entry(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(
        res,
        crate::modules::nai::store::catalog_entry(&id)
            .map(|entry| entry.unwrap_or(serde_json::Value::Null)),
    );
}

/// POST /v1/store/{id}/install
#[handler]
async fn v1_store_install(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::store::install(&id));
}

/// POST /v1/store/{id}/update
#[handler]
async fn v1_store_update(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::store::update(&id));
}

/// POST /v1/store/{id}/uninstall
#[handler]
async fn v1_store_uninstall(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::store::uninstall(&id));
}

/// POST /v1/store/{id}/launch
#[handler]
async fn v1_store_launch(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::store::launch(&id));
}

/// GET /v1/store/{id}/status  (independent install-state detection)
#[handler]
async fn v1_store_status(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_result(res, crate::modules::nai::store::status(&id));
}

/// GET /v1/store/jobs  (recent background package jobs, newest first)
#[handler]
async fn v1_store_jobs(res: &mut Response) {
    write_result(res, crate::modules::nai::store::jobs());
}

/// POST /v1/store/jobs/{jobId}/cancel  (kill + reap the direct child)
#[handler]
async fn v1_store_job_cancel(req: &mut Request, res: &mut Response) {
    let raw = req.param::<String>("jobId").unwrap_or_default();
    match raw.parse::<u64>() {
        Ok(job_id) => write_json(res, &crate::modules::nai::store::cancel(job_id)),
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(format!("invalid job id: {raw}")));
        }
    }
}

// ============================ v1 system tray ============================
// Same native `system_tray` command core used by the webviews, MCP and CLI.

/// GET /v1/tray/icons
#[handler]
async fn v1_tray_icons(res: &mut Response) {
    write_json(
        res,
        &crate::modules::system_tray::infrastructure::list_tray_icons(),
    );
}

/// GET /v1/tray/pinned
#[handler]
async fn v1_tray_pinned(res: &mut Response) {
    write_json(
        res,
        &crate::modules::system_tray::infrastructure::list_pinned_tray_icons(),
    );
}

/// GET /v1/tray/pins
#[handler]
async fn v1_tray_pins(res: &mut Response) {
    write_json(
        res,
        &crate::modules::system_tray::infrastructure::get_tray_pin_state(),
    );
}

fn write_tray_result(res: &mut Response, result: crate::error::Result<()>) {
    match result {
        Ok(()) => {
            res.status_code(StatusCode::NO_CONTENT);
        }
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err.to_string()));
        }
    }
}

/// POST /v1/tray/{id}/pin
#[handler]
async fn v1_tray_pin(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_tray_result(
        res,
        crate::modules::system_tray::infrastructure::pin_tray_icon(id),
    );
}

/// POST /v1/tray/{id}/unpin
#[handler]
async fn v1_tray_unpin(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    write_tray_result(
        res,
        crate::modules::system_tray::infrastructure::unpin_tray_icon(id),
    );
}

/// PATCH /v1/tray/order  body: ["keyA","keyB",...]
#[handler]
async fn v1_tray_order(req: &mut Request, res: &mut Response) {
    let body = match req.payload().await {
        Ok(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Err(_) => String::new(),
    };
    match serde_json::from_str::<Vec<String>>(&body) {
        Ok(order) => write_tray_result(
            res,
            crate::modules::system_tray::infrastructure::set_tray_pin_order(order),
        ),
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(format!("invalid body: {err}")));
        }
    }
}

/// POST /v1/tray/{id}/{action}
#[handler]
async fn v1_tray_action(req: &mut Request, res: &mut Response) {
    let id = req.param::<String>("id").unwrap_or_default();
    let action = req.param::<String>("action").unwrap_or_default();
    let parsed = match crate::cli::tray_cli::parse_action(&action) {
        Ok(parsed) => parsed,
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Text::Plain(err.to_string()));
            return;
        }
    };
    write_tray_result(
        res,
        crate::modules::system_tray::infrastructure::send_tray_action(id, parsed),
    );
}

// ============================ mcp json-rpc ============================

/// Minimal stateless MCP (JSON-RPC 2.0) over the same native core.
#[handler]
async fn mcp(req: &mut Request, res: &mut Response) {
    let body = match req.payload().await {
        Ok(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Err(_) => String::new(),
    };
    let answer = crate::server::mcp::handle_jsonrpc(&body);
    res.render(Text::Json(answer));
}

/// Starts the background HTTP server on its own thread, with a dedicated single-threaded
/// runtime, so it never competes with the main runtime workers. Intended to be called once
/// at app startup.
pub fn start_http_server() {
    let spawned = std::thread::Builder::new()
        .name("http-server".into())
        .spawn(|| {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    log::error!("Failed to create HTTP server runtime: {err:?}");
                    return;
                }
            };
            runtime.block_on(run_http_server());
        });

    if let Err(err) = spawned {
        log::error!("Failed to spawn HTTP server thread: {err:?}");
    }
}

async fn run_http_server() {
    let api = Router::new()
        .push(Router::with_path("ping").get(ping))
        .push(
            Router::with_path("resources")
                .push(Router::with_path("themes").get(themes))
                .push(Router::with_path("themes/tokens").get(theme_tokens))
                .push(Router::with_path("icon-packs").get(icon_packs)),
        );

    let v1 = Router::new()
        .push(
            Router::with_path("windows").get(v1_windows).push(
                Router::with_path("{id}")
                    .get(v1_window)
                    .push(Router::with_path("focus").post(v1_window_focus))
                    .push(Router::with_path("maximize").post(v1_window_maximize))
                    .push(Router::with_path("restore").post(v1_window_restore))
                    .push(Router::with_path("minimize").post(v1_window_minimize))
                    .push(Router::with_path("close").post(v1_window_close))
                    .push(Router::with_path("focus-maximize").post(v1_window_focus_maximize))
                    .push(Router::with_path("monitors/{index}").post(v1_window_monitor)),
            ),
        )
        .push(Router::with_path("apps").get(v1_apps))
        .push(
            Router::with_path("groups/{appId}/order")
                .get(v1_group_order)
                .patch(v1_set_group_order),
        )
        .push(Router::with_path("taskbar/items").get(v1_taskbar_items))
        .push(
            Router::with_path("tray")
                .push(Router::with_path("icons").get(v1_tray_icons))
                .push(Router::with_path("pinned").get(v1_tray_pinned))
                .push(Router::with_path("pins").get(v1_tray_pins))
                .push(Router::with_path("order").patch(v1_tray_order))
                .push(
                    Router::with_path("{id}")
                        .push(Router::with_path("pin").post(v1_tray_pin))
                        .push(Router::with_path("unpin").post(v1_tray_unpin))
                        .push(Router::with_path("{action}").post(v1_tray_action)),
                ),
        )
        .push(Router::with_path("metrics").get(v1_metrics))
        .push(Router::with_path("trace").get(v1_trace))
        .push(
            Router::with_path("nai")
                .push(Router::with_path("graph").get(v1_nai_graph))
                .push(Router::with_path("capabilities").get(v1_nai_capabilities))
                .push(Router::with_path("activate/{id}").post(v1_nai_activate))
                .push(Router::with_path("undo").post(v1_nai_undo))
                .push(Router::with_path("activities").get(v1_nai_activities))
                .push(Router::with_path("capsules").get(v1_nai_capsules))
                .push(Router::with_path("gateway/models").get(v1_nai_gateway_models))
                .push(Router::with_path("telemetry").get(v1_nai_telemetry))
                .push(Router::with_path("semantic/{query}").get(v1_nai_semantic))
                .push(
                    Router::with_path("semantic-upsert/{id}")
                        .post(v1_nai_semantic_upsert)
                        .get(v1_nai_semantic_upsert),
                )
                .push(Router::with_path("pip").get(v1_nai_pip))
                .push(
                    Router::with_path("shorts")
                        .push(Router::with_path("queue").get(v1_nai_shorts_queue))
                        .push(Router::with_path("next").post(v1_nai_shorts_next))
                        .push(Router::with_path("enqueue/{id}").post(v1_nai_shorts_enqueue))
                        .push(Router::with_path("{query}").get(v1_nai_shorts)),
                )
                .push(
                    Router::with_path("social")
                        .push(Router::with_path("compose").post(v1_nai_social_compose))
                        .push(Router::with_path("{lane}").get(v1_nai_social)),
                )
                .push(Router::with_path("v271/{prompt}").post(v1_nai_v271))
                .push(
                    Router::with_path("apps").get(v1_nai_apps).push(
                        Router::with_path("{id}")
                            .push(Router::with_path("launch").post(v1_nai_app_launch)),
                    ),
                ),
        )
        .push(
            Router::with_path("store")
                .push(Router::with_path("catalog").get(v1_store_catalog))
                .push(Router::with_path("catalog/{id}").get(v1_store_catalog_entry))
                .push(
                    Router::with_path("jobs")
                        .get(v1_store_jobs)
                        .push(Router::with_path("{jobId}").post(v1_store_job_cancel)),
                )
                .push(
                    Router::with_path("{id}")
                        .push(Router::with_path("install").post(v1_store_install))
                        .push(Router::with_path("update").post(v1_store_update))
                        .push(Router::with_path("uninstall").post(v1_store_uninstall))
                        .push(Router::with_path("launch").post(v1_store_launch))
                        .push(Router::with_path("status").get(v1_store_status)),
                ),
        );

    let doc = OpenApi::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).merge_router(&api);

    let automation = {
        let state = crate::state::application::FULL_STATE.load();
        state.settings.by_widget.weg.automation.clone()
    };
    log::info!(
        "automation gate enabled={} rest={} mcp={} port={}",
        automation.enabled,
        automation.rest_enabled,
        automation.mcp_enabled,
        automation.rest_port
    );

    let mut router = Router::new()
        .push(api)
        .push(doc.into_router("/api-doc/openapi.json"))
        .push(Router::with_path("api-doc").goal(scalar_docs));

    if automation.enabled {
        if automation.mcp_enabled {
            router = router.push(Router::with_path("mcp").post(mcp));
        }
        if automation.rest_enabled {
            router = router.push(Router::with_path("v1").push(v1));
        }
    }

    let port = if automation.rest_enabled {
        automation.rest_port
    } else {
        LOCAL_API_PORT
    };

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let acceptor = match TcpListener::new(addr).try_bind().await {
        Ok(acceptor) => acceptor,
        Err(err) => {
            log::error!("Failed to bind HTTP server on {addr}: {err:?}");
            return;
        }
    };

    log::info!("HTTP server listening on {addr}");
    Server::new(acceptor).serve(router).await;
}
