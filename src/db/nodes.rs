// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};

use crate::protocol::{ProtocolName, ProxyKind, ServerConfig};

use super::credentials::{self, CredentialInput};
use super::entities::nodes;

pub type Node = nodes::Model;

/// Node row plus its parsed protocol configuration.
#[derive(Debug, Clone, Serialize)]
pub struct NodeSummary {
    pub id: i64,
    pub name: String,
    pub protocol: String,
    pub server: String,
    pub port: u16,
    pub enabled: bool,
    pub sort_order: i32,
    pub params: ProxyKind,
    /// How the node itself runs. Absent for older rows.
    pub server_params: Option<ServerConfig>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

/// A node paired with the proxy credentials of one subscription user.
#[derive(Debug, Clone)]
pub struct ResolvedNode {
    pub node: Node,
    pub params: ProxyKind,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeInput {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub params: ProxyKind,
    /// How the node runs. Omitted by older clients, so it stays optional.
    #[serde(default)]
    pub server_params: Option<ServerConfig>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sort_order: i32,
    /// Credentials per user. A user without a row here does not receive the node.
    #[serde(default)]
    pub credentials: Vec<CredentialInput>,
}

impl NodeInput {
    /// Protocol name implied by the submitted parameters.
    pub fn protocol(&self) -> &'static str {
        self.params.protocol_str()
    }

    fn enabled(&self) -> bool {
        self.enabled
    }
}

fn default_true() -> bool {
    true
}

/// Serialises protocol parameters for storage.
fn serialize_params(params: &ProxyKind) -> Result<String, DbErr> {
    serde_json::to_string(params).map_err(|error| DbErr::Custom(error.to_string()))
}

/// Serialises the server half, which is optional for older rows.
fn serialize_server_params(params: Option<&ServerConfig>) -> Result<Option<String>, DbErr> {
    match params {
        Some(params) => serde_json::to_string(params)
            .map(Some)
            .map_err(|error| DbErr::Custom(error.to_string())),
        None => Ok(None),
    }
}

fn deserialize_params(protocol: &str, raw: &str) -> Result<ProxyKind, DbErr> {
    let parsed: ProxyKind = serde_json::from_str(raw).map_err(|error| {
        DbErr::Custom(format!(
            "node protocol `{protocol}` has invalid params: {error}"
        ))
    })?;

    if parsed.protocol_str() != protocol {
        return Err(DbErr::Custom(format!(
            "node protocol `{protocol}` does not match stored params `{}`",
            parsed.protocol_str()
        )));
    }

    Ok(parsed)
}

fn to_summary(node: Node) -> Result<NodeSummary, DbErr> {
    let params = deserialize_params(&node.protocol, &node.params)?;
    let server_params = match node.server_params.as_deref() {
        Some(raw) if !raw.is_empty() => Some(serde_json::from_str(raw).map_err(|error| {
            DbErr::Custom(format!(
                "node {} has invalid server params: {error}",
                node.id
            ))
        })?),
        _ => None,
    };

    Ok(NodeSummary {
        id: node.id,
        name: node.name,
        protocol: node.protocol,
        server: node.server,
        port: node.port as u16,
        enabled: node.enabled,
        sort_order: node.sort_order,
        params,
        server_params,
        created_at: node.created_at,
        updated_at: node.updated_at,
    })
}

pub async fn list(conn: &DatabaseConnection) -> Result<Vec<NodeSummary>, DbErr> {
    let rows = nodes::Entity::find()
        .order_by_asc(nodes::Column::SortOrder)
        .order_by_asc(nodes::Column::Id)
        .all(conn)
        .await?;

    rows.into_iter().map(to_summary).collect()
}

pub async fn get(conn: &DatabaseConnection, id: i64) -> Result<Option<NodeSummary>, DbErr> {
    match nodes::Entity::find_by_id(id).one(conn).await? {
        Some(node) => Ok(Some(to_summary(node)?)),
        None => Ok(None),
    }
}

/// Flips `enabled` without touching anything else, so a list-view toggle can
/// never resubmit (and silently overwrite) protocol parameters.
pub async fn set_enabled(
    conn: &DatabaseConnection,
    id: i64,
    enabled: bool,
) -> Result<Option<NodeSummary>, DbErr> {
    let existing = match nodes::Entity::find_by_id(id).one(conn).await? {
        Some(node) => node,
        None => return Ok(None),
    };

    if existing.enabled != enabled {
        let mut active: nodes::ActiveModel = existing.into();
        active.enabled = Set(enabled);
        active.updated_at = Set(Utc::now());
        active.update(conn).await?;
    }

    get(conn, id).await
}

pub async fn count_by_protocol(conn: &DatabaseConnection) -> Result<Vec<(String, i64)>, DbErr> {
    let rows = nodes::Entity::find().all(conn).await?;
    let mut counts: Vec<(String, i64)> = Vec::new();
    for protocol in ProtocolName::ALL {
        let name = protocol.as_str();
        let count = rows.iter().filter(|row| row.protocol == name).count() as i64;
        if count > 0 {
            counts.push((name.to_owned(), count));
        }
    }
    Ok(counts)
}

pub async fn create(
    conn: &DatabaseConnection,
    key: &crate::crypto::EncryptionKey,
    input: &NodeInput,
) -> Result<NodeSummary, DbErr> {
    input.params.validate().map_err(DbErr::Custom)?;

    let created = nodes::ActiveModel {
        name: Set(input.name.trim().to_owned()),
        protocol: Set(input.protocol().to_owned()),
        server: Set(input.server.trim().to_owned()),
        port: Set(input.port as i32),
        params: Set(serialize_params(&input.params)?),
        server_params: Set(serialize_server_params(input.server_params.as_ref())?),
        enabled: Set(input.enabled()),
        sort_order: Set(input.sort_order),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    credentials::replace_for_node(conn, key, created.id, &created.name, &input.credentials).await?;
    to_summary(created)
}

pub async fn update(
    conn: &DatabaseConnection,
    key: &crate::crypto::EncryptionKey,
    id: i64,
    input: &NodeInput,
) -> Result<Option<NodeSummary>, DbErr> {
    let Some(existing) = nodes::Entity::find_by_id(id).one(conn).await? else {
        return Ok(None);
    };

    input.params.validate().map_err(DbErr::Custom)?;

    let mut active: nodes::ActiveModel = existing.into();
    active.name = Set(input.name.trim().to_owned());
    active.protocol = Set(input.protocol().to_owned());
    active.server = Set(input.server.trim().to_owned());
    active.port = Set(input.port as i32);
    active.params = Set(serialize_params(&input.params)?);
    active.server_params = Set(serialize_server_params(input.server_params.as_ref())?);
    active.enabled = Set(input.enabled());
    active.sort_order = Set(input.sort_order);
    active.updated_at = Set(Utc::now());

    let updated = active.update(conn).await?;
    credentials::replace_for_node(conn, key, updated.id, &updated.name, &input.credentials).await?;
    Ok(Some(to_summary(updated)?))
}

pub async fn delete(conn: &DatabaseConnection, id: i64) -> Result<bool, DbErr> {
    credentials::delete_for_node(conn, id).await?;
    let changed = nodes::Entity::delete_by_id(id)
        .exec(conn)
        .await?
        .rows_affected;
    Ok(changed > 0)
}

pub async fn count_enabled(conn: &DatabaseConnection) -> Result<i64, DbErr> {
    use sea_orm::PaginatorTrait;
    nodes::Entity::find()
        .filter(nodes::Column::Enabled.eq(true))
        .count(conn)
        .await
        .map(|count| count as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Hysteria2Config, TlsConfig, VlessConfig};

    fn sample_kind() -> ProxyKind {
        ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn params_round_trip_through_storage_format() {
        let kind = sample_kind();
        let raw = serialize_params(&kind).expect("serialize");
        let back = deserialize_params("vless", &raw).expect("deserialize");
        assert_eq!(kind, back);
    }

    #[test]
    fn protocol_mismatch_is_rejected() {
        let raw = serialize_params(&sample_kind()).expect("serialize");
        assert!(deserialize_params("trojan", &raw).is_err());
    }

    #[test]
    fn invalid_json_is_rejected() {
        assert!(deserialize_params("vless", "{").is_err());
    }

    #[test]
    fn input_exposes_protocol_name() {
        let input = NodeInput {
            name: "n".to_owned(),
            server: "example.com".to_owned(),
            port: 443,
            params: ProxyKind::Hysteria2(Hysteria2Config::default()),
            enabled: true,
            sort_order: 0,
            server_params: None,
            credentials: Vec::new(),
        };
        assert_eq!(input.protocol(), "hysteria2");
        assert!(input.enabled());
    }
}
