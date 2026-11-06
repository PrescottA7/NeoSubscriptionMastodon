// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::Serialize;

/// A published proxy endpoint.
///
/// `params` holds the **client** half (how to reach the node) and
/// `server_params` holds the **server** half (how the node runs). They are kept
/// apart because a subscription only needs the former, while the machine
/// operator needs the latter — including the REALITY private key, which has no
/// business in a client document.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "nodes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub protocol: String,
    pub server: String,
    pub port: i32,
    pub params: String,
    /// `None` for nodes created before server-side configuration existed.
    pub server_params: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::node_credentials::Entity")]
    NodeCredentials,
}

impl Related<super::node_credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NodeCredentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
