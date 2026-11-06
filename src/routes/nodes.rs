// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::auth::RequireAuth;
use crate::db::nodes::{self, NodeInput, NodeSummary};
use crate::error::{RequestError, RequestResult};
use crate::protocol::ProtocolName;
use crate::routes::extract::ApiJson;
use crate::state::AppState;
use crate::validate::Validator;

#[derive(Serialize)]
struct ProtocolDescriptor {
    name: &'static str,
    default_port: u16,
    requires_tls: bool,
    supports_transport: bool,
}

#[derive(Serialize)]
struct NodeDetail {
    #[serde(flatten)]
    node: NodeSummary,
    credentials: Vec<crate::db::credentials::CredentialView>,
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/protocols", get(list_protocols))
        .route("/api/nodes", get(list_nodes).post(create_node))
        .route(
            "/api/nodes/{id}",
            get(get_node).put(update_node).delete(delete_node),
        )
        .route("/api/nodes/{id}/credentials", get(list_node_credentials))
        .route(
            "/api/nodes/{id}/enabled",
            axum::routing::patch(set_node_enabled),
        )
        .route("/api/nodes/{id}/suggested-credentials", get(suggest_credentials))
        .route(
            "/api/nodes/{id}/credentials/{user_id}/rotate",
            post(rotate_credential),
        )
        .route_layer(axum::middleware::from_extractor_with_state::<RequireAuth, AppState>(
            state.clone(),
        ))
}

/// Describes the protocols the UI should offer, so the form stays in sync with
/// the Rust-side parameter model.
async fn list_protocols() -> Json<Vec<ProtocolDescriptor>> {
    Json(
        ProtocolName::ALL
            .iter()
            .map(|protocol| ProtocolDescriptor {
                name: protocol.as_str(),
                default_port: protocol.default_port(),
                requires_tls: protocol.requires_tls(),
                supports_transport: protocol.supports_transport(),
            })
            .collect(),
    )
}

async fn list_nodes(State(state): State<AppState>) -> RequestResult<Json<Vec<NodeSummary>>> {
    Ok(Json(nodes::list(&state.db).await?))
}

async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<NodeDetail>> {
    let node = nodes::get(&state.db, id)
        .await?
        .ok_or_else(|| RequestError::not_found(format!("节点 {id} 不存在")))?;
    let credentials =
        crate::db::credentials::list_for_node(&state.db, &state.key, id).await?;

    Ok(Json(NodeDetail { node, credentials }))
}

async fn create_node(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<NodeInput>,
) -> RequestResult<Response> {
    validate(&payload)?;
    ensure_users_exist(&state, &payload).await?;
    let node = nodes::create(&state.db, &state.key, &payload).await?;

    Ok((StatusCode::CREATED, Json(node)).into_response())
}

async fn update_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<NodeInput>,
) -> RequestResult<Json<NodeSummary>> {
    validate(&payload)?;
    ensure_users_exist(&state, &payload).await?;
    nodes::update(&state.db, &state.key, id, &payload)
        .await?
        .map(Json)
        .ok_or_else(|| RequestError::not_found(format!("节点 {id} 不存在")))
}

/// Rejects credentials pointing at users that do not exist, so a mistake in the
/// UI surfaces as a field error instead of a foreign-key failure.
async fn ensure_users_exist(state: &AppState, payload: &NodeInput) -> RequestResult<()> {
    if payload.credentials.is_empty() {
        return Ok(());
    }

    let known: std::collections::HashSet<i64> = crate::db::users::list(&state.db)
        .await?
        .into_iter()
        .map(|user| user.id)
        .collect();

    let mut validator = Validator::new();
    for (index, credential) in payload.credentials.iter().enumerate() {
        if credential.user_id > 0 && !known.contains(&credential.user_id) {
            validator.push(
                format!("credentials[{index}].user_id"),
                format!("用户 {} 不存在", credential.user_id),
            );
        }
    }

    validator.finish()
}

/// Credentials currently configured for a node.
async fn list_node_credentials(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<Vec<crate::db::credentials::CredentialView>>> {
    Ok(Json(
        crate::db::credentials::list_for_node(&state.db, &state.key, id).await?,
    ))
}

/// Fresh values for every user, so the editor can prefill new rows.
async fn suggest_credentials(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<Vec<crate::db::credentials::CredentialView>>> {
    let node = nodes::get(&state.db, id)
        .await?
        .ok_or_else(|| RequestError::not_found(format!("节点 {id} 不存在")))?;

    Ok(Json(
        crate::db::credentials::suggested_for_node(&state.db, &state.key, id, &node.name).await?,
    ))
}

/// Regenerates the uuid and password of one pair.
async fn rotate_credential(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(i64, i64)>,
) -> RequestResult<Json<crate::db::credentials::CredentialView>> {
    crate::db::credentials::rotate(&state.db, &state.key, id, user_id)
        .await?
        .map(Json)
        .ok_or_else(|| RequestError::not_found("该用户在此节点上没有凭据"))
}

#[derive(serde::Deserialize)]
struct EnabledInput {
    enabled: bool,
}

/// Toggles a node from the list view without resubmitting its full config.
async fn set_node_enabled(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<EnabledInput>,
) -> RequestResult<Json<NodeSummary>> {
    nodes::set_enabled(&state.db, id, payload.enabled)
        .await?
        .map(Json)
        .ok_or_else(|| RequestError::not_found(format!("节点 {id} 不存在")))
}

async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<StatusCode> {    if nodes::delete(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(RequestError::not_found(format!("节点 {id} 不存在")))
    }
}

/// Rejects broken submissions before they reach the database, reporting every
/// problem at once with the field it belongs to.
#[allow(clippy::result_large_err)]
fn validate(input: &NodeInput) -> Result<(), RequestError> {
    let mut validator = Validator::new();

    validator.text("name", "节点名称", &input.name, 1, 64);
    validator.host("server", &input.server);
    validator.port("port", input.port);

    let mut seen = std::collections::HashSet::new();
    for (index, credential) in input.credentials.iter().enumerate() {
        if credential.user_id <= 0 {
            validator.push(
                format!("credentials[{index}].user_id"),
                "请选择凭据所属的用户",
            );
        } else if !seen.insert(credential.user_id) {
            validator.push(
                format!("credentials[{index}].user_id"),
                "同一用户在该节点上只能有一组凭据",
            );
        }
        if let Some(uuid) = credential.uuid.as_deref().map(str::trim)
            && !uuid.is_empty()
            && uuid::Uuid::parse_str(uuid).is_err()
        {
            validator.push(
                format!("credentials[{index}].uuid"),
                "UUID 格式不正确，例如 5783a3e7-e373-51cd-8642-c83782b807c5",
            );
        }
    }

    for (field, message) in protocol_errors(&input.params) {
        validator.push(field, message);
    }

    validator.finish()
}

/// Flattens the protocol-level validation messages into `(field, message)` pairs.
fn protocol_errors(params: &crate::protocol::ProxyKind) -> Vec<(&'static str, String)> {
    let mut errors = Vec::new();

    if let Err(message) = params.validate() {
        errors.push(("params", message));
    }

    match params {
        crate::protocol::ProxyKind::Vless(config) => {
            if let Some(flow) = config.flow_value()
                && flow != "xtls-rprx-vision"
            {
                errors.push(("params.flow", format!("不支持 flow `{flow}`")));
            }
        }
        crate::protocol::ProxyKind::Hysteria2(config) => {
            for range in &config.server_ports {
                if crate::protocol::kinds::hysteria2::parse_port_range(range).is_none() {
                    errors.push((
                        "params.server_ports",
                        format!("端口范围 “{range}” 不合法，应形如 2080:3000 或 443"),
                    ));
                }
            }
            if let Some(obfs) = config.obfs.as_ref() {
                if obfs.password.is_empty() {
                    errors.push(("params.obfs.password", "启用混淆时必须填写密码".to_owned()));
                }
                if let (Some(min), Some(max)) = (obfs.min_packet_size, obfs.max_packet_size)
                    && min > max
                {
                    errors.push((
                        "params.obfs.max_packet_size",
                        "最大包大小不能小于最小包大小".to_owned(),
                    ));
                }
            }
        }
        crate::protocol::ProxyKind::Tuic(config)
            if config.udp_over_stream
                && config.udp_relay_mode == crate::protocol::UdpRelayMode::Quic =>
        {
            errors.push((
                "params.udp_relay_mode",
                "UDP over Stream 与 quic 中继模式不能同时启用".to_owned(),
            ));
        }
        _ => {}
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{AnytlsConfig, Hysteria2Config, TlsConfig, VlessConfig};

    fn base_input(params: crate::protocol::ProxyKind) -> NodeInput {
        NodeInput {
            name: "Tokyo".to_owned(),
            server: "example.com".to_owned(),
            port: 443,
            params,
            enabled: true,
            sort_order: 0,
            server_params: None,
            credentials: Vec::new(),
        }
    }

    #[test]
    fn valid_input_passes() {
        let input = base_input(crate::protocol::ProxyKind::Anytls(AnytlsConfig::default()));
        assert!(validate(&input).is_ok());
    }

    #[test]
    fn blank_name_is_rejected() {
        let mut input = base_input(crate::protocol::ProxyKind::Anytls(AnytlsConfig::default()));
        input.name = "  ".to_owned();
        assert!(validate(&input).is_err());
    }

    #[test]
    fn blank_server_is_rejected() {
        let mut input = base_input(crate::protocol::ProxyKind::Anytls(AnytlsConfig::default()));
        input.server = String::new();
        assert!(validate(&input).is_err());
    }

    #[test]
    fn server_with_spaces_is_rejected() {
        let mut input = base_input(crate::protocol::ProxyKind::Anytls(AnytlsConfig::default()));
        input.server = "example .com".to_owned();
        assert!(validate(&input).is_err());
    }

    #[test]
    fn invalid_protocol_params_are_rejected() {
        let input = base_input(crate::protocol::ProxyKind::Hysteria2(Hysteria2Config {
            tls: TlsConfig::default(),
            ..Default::default()
        }));
        assert!(validate(&input).is_err());
    }

    #[test]
    fn vision_flow_requires_tls() {
        let input = base_input(crate::protocol::ProxyKind::Vless(VlessConfig {
            flow: Some("xtls-rprx-vision".to_owned()),
            ..Default::default()
        }));
        assert!(validate(&input).is_err());
    }
}
