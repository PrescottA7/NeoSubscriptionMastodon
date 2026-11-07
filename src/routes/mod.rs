// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

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
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::json;

use crate::state::{AppState, RequestScope};

/// Paths served by the admin single-page application, relative to the admin
/// prefix.
const SPA_ROUTES: [&str; 3] = ["/", "/login", "/users"];

/// The SPA also owns `/settings` and everything under `/nodes`, including deep
/// links such as `/nodes/12`.
fn is_spa_route(path: &str) -> bool {
    SPA_ROUTES.contains(&path) || path == "/settings" || path.starts_with("/nodes")
}

/// The admin API, mounted under the admin prefix (the root by default).
fn api_router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(auth::routes(state))
        .merge(users::routes(state))
        .merge(nodes::routes(state))
        .merge(settings::routes(state))
        .merge(render::routes(state))
        .route("/api/health", get(health))
}

pub fn routes(state: &AppState) -> Router<AppState> {
    let prefix = state.admin_prefix().to_owned();
    let hosts_hidden =
        state.config.subscription_host.is_some() || state.config.admin_host.is_some();

    // An empty prefix keeps the historical root layout.
    let api = if prefix.is_empty() {
        api_router(state)
    } else {
        Router::new().nest(&prefix, api_router(state))
    };

    // Matched API routes bypass `dispatch`, so host scoping has to be enforced
    // here as well; otherwise the admin API would answer on the subscription
    // host.
    let api = if hosts_hidden {
        api.layer(axum::middleware::from_fn_with_state(
            state.clone(),
            reject_subscription_host,
        ))
    } else {
        api
    };

    api.fallback(dispatch)
}

/// Answers 404 for admin routes reached through the subscription host.
async fn reject_subscription_host(
    State(state): State<AppState>,
    request: Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    if state.scope_for(request.headers()) != RequestScope::Admin {
        return disguise(&state, &request.into_parts().0).await;
    }
    next.run(request).await
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Terminal handler for every path the router did not match.
async fn dispatch(State(state): State<AppState>, request: Request<Body>) -> Response {
    let (parts, _body) = request.into_parts();
    let scope = state.scope_for(&parts.headers);

    // On a dedicated admin host the subscription prefix is never served; on a
    // dedicated subscription host, or from an unrecognised host, the admin UI
    // and API stay hidden.
    if matches!(scope, RequestScope::Mixed | RequestScope::Subscription)
        && let Some(response) = try_subscription(&state, &parts).await
    {
        return response;
    }
    if matches!(scope, RequestScope::Mixed | RequestScope::Admin)
        && let Some(response) = try_admin(&state, &parts).await
    {
        return response;
    }

    disguise(&state, &parts).await
}

/// Serves a subscription request when the path sits under the prefix.
async fn try_subscription(
    state: &AppState,
    parts: &axum::http::request::Parts,
) -> Option<Response> {
    let path = parts.uri.path();
    let prefix = state.subscription_prefix();

    if path != prefix && !path.starts_with(&format!("{prefix}/")) {
        return None;
    }

    let token = path[prefix.len()..].trim_start_matches('/');
    Some(subscription::respond(state, parts.headers.clone(), parts.uri.query(), token).await)
}

/// Serves an API or SPA request when the path sits under the admin prefix.
///
/// Unknown paths return `None` so the caller can answer with the disguise page;
/// claiming them here would make the admin area trivially discoverable.
async fn try_admin(state: &AppState, parts: &axum::http::request::Parts) -> Option<Response> {
    let path = parts.uri.path();
    let prefix = state.admin_prefix();

    let rest = if prefix.is_empty() {
        path
    } else if path == prefix {
        "/"
    } else {
        let stripped = path.strip_prefix(&format!("{prefix}/"))?;
        // Rebuild the rooted path so the checks below stay prefix-agnostic.
        return admin_response(&format!("/{stripped}"), prefix).await;
    };

    admin_response(rest, prefix).await
}

async fn admin_response(rest: &str, prefix: &str) -> Option<Response> {
    if rest.starts_with("/api/") {
        return Some(api_not_found());
    }

    if is_admin_asset(rest) {
        return Some(crate::static_content::serve_static(prefix, rest).await);
    }

    None
}

/// Answers a probe with the decoy page, or mirrors the sample site.
async fn disguise(state: &AppState, parts: &axum::http::request::Parts) -> Response {
    let settings = state.settings_cached();

    if parts.method == Method::GET || parts.method == Method::HEAD {
        if let Some(response) = passthrough(&settings, parts).await {
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
    let target = crate::disguise_passthrough::Target::from_settings(settings, &parts.uri)?;

    // The static page has to honour the configured delay as well, otherwise the
    // two branches would be distinguishable by response time.
    if settings.failure_delay_ms > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(settings.failure_delay_ms)).await;
    }

    match target.mirror(&parts.method, &parts.headers).await {
        Ok(mirror) => Some(crate::disguise_passthrough::to_response(mirror)),
        Err(error) => {
            tracing::warn!(%error, "disguise passthrough failed, serving the static page");
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
        Json(json!({ "error": "API endpoint not found", "code": "NOT_FOUND" })),
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
