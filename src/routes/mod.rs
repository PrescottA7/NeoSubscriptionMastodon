//! HTTP routing.
//!
//! The router only registers the admin API and the fragment handlers. Every
//! other path goes through [`dispatch`], which decides between the subscription
//! endpoint, the admin SPA, and the disguise page.

pub mod auth;
pub mod extract;
pub mod nodes;
pub mod render;
pub mod settings;
pub mod subscription;
pub mod users;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::state::AppState;

/// Paths served by the admin single-page application.
const SPA_ROUTES: [&str; 3] = ["/", "/login", "/users"];

/// The SPA also owns `/settings` and everything under `/nodes`, including deep
/// links such as `/nodes/12`.
fn is_spa_route(path: &str) -> bool {
    SPA_ROUTES.contains(&path) || path == "/settings" || path.starts_with("/nodes")
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(auth::routes(state))
        .merge(users::routes(state))
        .merge(nodes::routes(state))
        .merge(settings::routes(state))
        .merge(render::routes(state))
        .route("/api/health", get(health))
        .fallback(dispatch)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Terminal handler for every path the router did not match.
async fn dispatch(State(state): State<AppState>, request: Request<Body>) -> Response {
    let (parts, _body) = request.into_parts();
    let path = parts.uri.path();
    let settings = state.settings_cached();
    let prefix = settings.subscription_prefix.clone();

    if path == prefix || path.starts_with(&format!("{prefix}/")) {
        let token = path[prefix.len()..].trim_start_matches('/');
        return subscription::respond(&state, parts.headers.clone(), parts.uri.query(), token).await;
    }

    if path.starts_with("/api/") {
        return api_not_found();
    }

    if is_admin_asset(path) {
        return crate::static_content::serve_static(parts.uri.clone()).await;
    }

    // Anything else is treated as a probe and answered with the decoy page.
    if parts.method == Method::GET || parts.method == Method::HEAD {
        if let Some(response) = passthrough(&settings, &parts).await {
            return response;
        }
        return state.disguise().not_found().await;
    }
    state.disguise().not_found_empty()
}

/// Mirrors the probe from the configured sample site.
///
/// Returns `None` when pass-through is off or the upstream fails, in which case
/// the caller falls back to the static disguise page. Failures are logged but
/// never surfaced, so the client cannot tell the two paths apart.
async fn passthrough(
    settings: &crate::db::settings::AppSettings,
    parts: &axum::http::request::Parts,
) -> Option<Response> {
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    let target = crate::disguise_passthrough::Target::from_settings(settings, path_and_query)?;

    // The static page has to honour the configured delay as well, otherwise the
    // two branches would be distinguishable by response time.
    if settings.failure_delay_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(settings.failure_delay_ms)).await;
    }

    match target.mirror(&parts.method, &parts.headers).await {
        Ok(mirror) => Some(crate::disguise_passthrough::to_response(mirror)),
        Err(error) => {
            tracing::debug!(%error, "disguise passthrough failed, serving the static page");
            None
        }
    }
}

/// True for SPA routes and asset files belonging to the admin UI.
fn is_admin_asset(path: &str) -> bool {
    if is_spa_route(path) {
        return true;
    }
    // Vite emits hashed bundles under `assets/`, plus a few root-level files.
    path.starts_with("/assets/")
        || matches!(
            path,
            "/favicon.svg" | "/favicon.ico" | "/robots.txt" | "/manifest.webmanifest"
        )
}

/// JSON 404 for unmatched API paths, so the frontend reports a real error
/// instead of receiving an HTML page.
fn api_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "接口不存在" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spa_routes_are_served() {
        assert!(is_admin_asset("/"));
        assert!(is_admin_asset("/login"));
        assert!(is_admin_asset("/users"));
        assert!(is_admin_asset("/settings"));
        assert!(is_admin_asset("/nodes"));
        assert!(is_admin_asset("/nodes/new"));
        assert!(is_admin_asset("/nodes/12"));
    }

    #[test]
    fn hashed_assets_are_served() {
        assert!(is_admin_asset("/assets/index-abc123.js"));
        assert!(is_admin_asset("/favicon.svg"));
    }

    #[test]
    fn probes_are_not_served_by_the_spa() {
        assert!(!is_admin_asset("/wp-login.php"));
        assert!(!is_admin_asset("/.env"));
        assert!(!is_admin_asset("/sometoken"));
        assert!(!is_admin_asset("/admin"));
    }
}
