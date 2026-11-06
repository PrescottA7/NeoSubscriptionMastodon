// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

/// Per-user credentials on one node.
///
/// A proxy server maintains its own account list per node: the same person may
/// be `alice` on one machine and `a-01` on another, with unrelated passwords.
/// Credentials therefore belong to the pair, not to the user.
///
/// A row also *is* the authorisation: a user only receives a node when a row
/// exists for that pair, so there is no "visible but unusable" state.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "node_credentials")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub node_id: i64,
    pub user_id: i64,
    /// Identifier used by VLESS and TUIC.
    pub uuid: String,
    /// Secret used by Trojan, Hysteria2, TUIC, and AnyTLS; encrypted at rest.
    pub password: String,
    /// A human label for the account on this node, e.g. what the server operator
    /// named the client entry. Falls back to the e-mail convention clients show.
    pub email: String,
    /// Disables one pair without touching the user or the node.
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::nodes::Entity",
        from = "Column::NodeId",
        to = "super::nodes::Column::Id"
    )]
    Node,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id"
    )]
    User,
}

impl Related<super::nodes::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Node.def()
    }
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
