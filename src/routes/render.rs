// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::auth::RequireAuth;
use crate::db::nodes;
use crate::db::users;
use crate::error::{RequestError, RequestResult};
use crate::render::{Core, RenderContext, context::RenderUser, render_with};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RenderQuery {
    /// Target core: `uri`, `sing-box`, `mihomo`, `xray`, or `hysteria2`.
    pub core: Option<String>,
    /// `1` (default) returns the complete document, `0` returns just the node.
    pub full: Option<u8>,
    /// Set to `1` to download the rendered document.
    pub download: Option<u8>,
    /// User whose credentials are substituted. Defaults to any user.
    pub user_id: Option<i64>,
}

#[derive(Serialize)]
struct RenderedDocument {
    core: &'static str,
    content_type: &'static str,
    filename: String,
    content: String,
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/nodes/{id}/render", get(render_node))
        .route("/api/nodes/{id}/server-config", get(render_server))
        .route("/api/render", get(render_all))
        .route_layer(axum::middleware::from_extractor_with_state::<
            RequireAuth,
            AppState,
        >(state.clone()))
}

/// Renders the node's own configuration, ready to be started by the proxy server.
///
/// This is the inbound half: listeners, REALITY private key, certificate paths,
/// and every account NSM knows for the node. It is deliberately separate from the
/// subscription renderers, which produce client documents.
async fn render_server(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<ServerRenderQuery>,
) -> RequestResult<Response> {
    let node = nodes::get(&state.db, id)
        .await?
        .ok_or_else(|| RequestError::not_found(format!("Node {id} not found")))?;

    let Some(server_config) = node.server_params.as_ref() else {
        return Err(RequestError::public_with_code(
            StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::MissingServerConfig,
            "This node has no server configuration yet; fill in the server section on the node edit page",
        ));
    };

    // The stored core is the default; the caller may ask for the other exporter,
    // but only when that core can actually run this listener.
    let core = match query.core.as_deref() {
        Some(value) => crate::protocol::ServerCore::parse(value).ok_or_else(|| {
            RequestError::public_with_code(
                StatusCode::BAD_REQUEST,
                crate::error::ErrorCode::UnsupportedCore,
                format!("Unknown core `{value}`, available options: xray / hysteria2"),
            )
        })?,
        None => server_config.core,
    };
    if !server_config.supports_core(core) {
        return Err(RequestError::public_with_code(
            StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::UnsupportedCore,
            format!(
                "The {} inbound cannot be started by {}",
                server_config.inbound.protocol.as_str(),
                core.as_str()
            ),
        ));
    }

    // Only accounts that are actually enabled become server clients.
    let accounts: Vec<crate::render::server::ServerAccount> =
        crate::db::credentials::list_for_node(&state.db, &state.key, id)
            .await?
            .into_iter()
            .filter(|credential| credential.enabled)
            .map(|credential| {
                crate::render::server::ServerAccount::new(
                    credential.email,
                    credential.uuid,
                    credential.password,
                )
            })
            .collect();

    let content = crate::render::server::render(server_config, &accounts, &node.name, core)
        .map_err(RequestError::internal)?;

    let extension = match core {
        crate::protocol::ServerCore::Xray => "json",
        crate::protocol::ServerCore::Hysteria2 => "yaml",
    };

    Ok(document_response_raw(
        &format!("{}-{}.{}", name_slug(&node.name), core.as_str(), extension),
        content,
        query.download.unwrap_or(0) == 1,
        core,
    ))
}

#[derive(Deserialize)]
pub struct ServerRenderQuery {
    /// Which core to emit. Defaults to the core stored on the node.
    pub core: Option<String>,
    pub download: Option<u8>,
}

fn document_response_raw(
    filename: &str,
    content: String,
    download: bool,
    core: crate::protocol::ServerCore,
) -> Response {
    let content_type = match core {
        crate::protocol::ServerCore::Xray => "application/json; charset=utf-8",
        crate::protocol::ServerCore::Hysteria2 => "text/yaml; charset=utf-8",
    };

    if !download {
        return Json(RenderedDocument {
            core: core.as_str(),
            content_type,
            filename: filename.to_owned(),
            content,
        })
        .into_response();
    }

    let mut response = Response::new(axum::body::Body::from(content));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    *response.status_mut() = StatusCode::OK;
    response
}

/// Renders a single node as a copyable/downloadable config file.
async fn render_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<RenderQuery>,
) -> RequestResult<Response> {
    let core = resolve_core(&query)?;
    let node = nodes::get(&state.db, id)
        .await?
        .ok_or_else(|| RequestError::not_found(format!("Node {id} not found")))?;

    let (user, credentials) = resolve_credentials(&state, node.id, query.user_id).await?;
    let context = RenderContext::new(user, credentials, node.params).with_node(
        node.server.clone(),
        node.port,
        node.name.clone(),
    );

    let content = if query.full.unwrap_or(1) == 0 {
        render_with(core, &context).map_err(|error| RequestError::bad_request(error.to_string()))?
    } else {
        crate::render::render_collection(core, std::slice::from_ref(&context))
            .map_err(|error| RequestError::bad_request(error.to_string()))?
    };

    Ok(document_response(
        core,
        &node.name,
        content,
        query.download.unwrap_or(0) == 1,
    ))
}

/// Renders every node the user holds credentials for, for the requested core.
async fn render_all(
    State(state): State<AppState>,
    Query(query): Query<RenderQuery>,
) -> RequestResult<Response> {
    let core = resolve_core(&query)?;

    let summary = match query.user_id {
        Some(id) => users::get(&state.db, id).await?,
        None => users::list(&state.db)
            .await?
            .into_iter()
            .find(|user| user.enabled),
    };
    let Some(summary) = summary else {
        return Err(RequestError::public_with_code(
            StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::NoUsers,
            "No users yet; create a user first",
        ));
    };

    let user = RenderUser {
        id: summary.id,
        remark: summary.remark,
    };

    let resolved =
        crate::db::credentials::resolved_for_user(&state.db, &state.key, summary.id).await?;

    let contexts: Vec<RenderContext> = resolved
        .into_iter()
        .enumerate()
        .filter_map(|(index, (node, credential))| {
            let params = serde_json::from_str(&node.params).ok()?;
            Some(
                RenderContext::new(
                    user.clone(),
                    crate::render::RenderCredentials::new(
                        credential.uuid,
                        credential.password,
                        credential.email,
                    ),
                    params,
                )
                .with_node(node.server, node.port as u16, node.name)
                .with_index(index),
            )
        })
        .collect();

    if contexts.is_empty() {
        return Err(RequestError::public_with_code(
            StatusCode::NOT_FOUND,
            crate::error::ErrorCode::NoNodes,
            "No available nodes",
        ));
    }

    let content = crate::render::render_collection(core, &contexts)
        .map_err(|error| RequestError::bad_request(error.to_string()))?;

    Ok(document_response(
        core,
        "nsm",
        content,
        query.download.unwrap_or(0) == 1,
    ))
}

/// Builds either an inline or an attachment response.
fn document_response(core: Core, name: &str, content: String, download: bool) -> Response {
    if !download {
        return Json(RenderedDocument {
            core: core.as_str(),
            content_type: core.content_type(),
            filename: format!("{}-{}.{}", name_slug(name), core.as_str(), core.extension()),
            content,
        })
        .into_response();
    }

    let filename = format!("{}-{}.{}", name_slug(name), core.as_str(), core.extension());
    let mut response = Response::new(axum::body::Body::from(content));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/octet-stream"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .unwrap_or(HeaderValue::from_static("attachment")),
    );
    *response.status_mut() = StatusCode::OK;
    response
}

#[allow(clippy::result_large_err)]
fn resolve_core(query: &RenderQuery) -> RequestResult<Core> {
    match query.core.as_deref() {
        Some(value) => Core::parse(value).ok_or_else(|| {
            RequestError::public_with_code(
                StatusCode::BAD_REQUEST,
                crate::error::ErrorCode::UnsupportedCore,
                format!("Unsupported core `{value}`"),
            )
        }),
        None => Ok(Core::Uri),
    }
}

/// Picks the credential set to render with.
///
/// Credentials belong to a (node, user) pair, so a node can only be rendered for
/// a user who actually has an account on it.
async fn resolve_credentials(
    state: &AppState,
    node_id: i64,
    user_id: Option<i64>,
) -> RequestResult<(RenderUser, crate::render::RenderCredentials)> {
    let summary = match user_id {
        Some(id) => users::get(&state.db, id).await?,
        None => users::list(&state.db)
            .await?
            .into_iter()
            .find(|user| user.enabled),
    };

    let Some(summary) = summary else {
        return Err(RequestError::public_with_code(
            StatusCode::BAD_REQUEST,
            crate::error::ErrorCode::NoUsers,
            "No users yet; create a user and configure node credentials for it first",
        ));
    };

    let credential =
        crate::db::credentials::resolved_for_pair(&state.db, &state.key, node_id, summary.id)
            .await?
            .ok_or_else(|| {
                RequestError::public_with_code(
                    StatusCode::NOT_FOUND,
                    crate::error::ErrorCode::NoCredentials,
                    format!("User \"{}\" has no credentials on this node", summary.remark),
                )
            })?;

    Ok((
        RenderUser {
            id: summary.id,
            remark: summary.remark,
        },
        crate::render::RenderCredentials::new(
            credential.uuid,
            credential.password,
            credential.email,
        ),
    ))
}

fn name_slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "node".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_defaults_to_uri() {
        let query = RenderQuery {
            core: None,
            full: None,
            download: None,
            user_id: None,
        };
        assert_eq!(resolve_core(&query).expect("core"), Core::Uri);
    }

    #[test]
    fn unknown_core_is_rejected() {
        let query = RenderQuery {
            core: Some("v2fly".to_owned()),
            full: None,
            download: None,
            user_id: None,
        };
        assert!(resolve_core(&query).is_err());
    }

    #[test]
    fn aliases_resolve() {
        let query = RenderQuery {
            core: Some("clash".to_owned()),
            full: None,
            download: None,
            user_id: None,
        };
        assert_eq!(resolve_core(&query).expect("core"), Core::Mihomo);
    }

    #[test]
    fn slug_is_safe_for_filenames() {
        assert_eq!(name_slug("Tokyo 01"), "Tokyo-01");
        assert_eq!(name_slug("東京"), "node");
        assert_eq!(name_slug("a/b:c"), "a-b-c");
    }
}
