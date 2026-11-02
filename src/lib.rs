//! NSM (NeoSubscriptionMastodon) — proxy subscription manager.

pub mod auth;
pub mod config;
pub mod crypto;
pub mod db;
pub mod disguise;
pub mod disguise_passthrough;
pub mod error;
pub mod logging;
pub mod protocol;
pub mod reality;
pub mod render;
pub mod routes;
pub mod state;
pub mod static_content;
pub mod utils;
pub mod validate;

use anyhow::Context;
use axum::Router;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

pub use config::AppConfig;
pub use state::AppState;

/// Builds the HTTP application: schema, state, routes, and tracing.
pub async fn build_app(config: AppConfig) -> anyhow::Result<Router> {
    config.ensure_directories()?;

    let state = AppState::new(config).await.context("initialize state")?;

    Ok(routes::routes(&state)
        .with_state(state)
        // Request logging: method, path, status, and latency. Without this the
        // logs stay empty during normal operation, which makes a failed login
        // impossible to diagnose.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        ))
}
