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

/// Starts the background HTTP server. Intended to be spawned once at app startup.
pub async fn start_server() {
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
        .push(Router::with_path("metrics").get(v1_metrics))
        .push(Router::with_path("trace").get(v1_trace));

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
