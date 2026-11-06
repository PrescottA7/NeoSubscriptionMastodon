// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::Serialize;

/// The single administrator account. `id` is always 1.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "admins")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
