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

use crate::utils::random_secret;

use super::entities::users;

pub type User = users::Model;

/// User representation for the admin API.
///
/// There are no proxy credentials here: those are per node and live in
/// `node_credentials`.
#[derive(Debug, Clone, Serialize)]
pub struct UserSummary {
    pub id: i64,
    pub remark: String,
    pub enabled: bool,
    /// Subscription slug, always visible so the link can be shown and edited.
    pub token: String,
    pub access_count: i64,
    pub last_accessed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<User> for UserSummary {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            remark: user.remark,
            enabled: user.enabled,
            token: user.token,
            access_count: user.access_count,
            last_accessed_at: user.last_accessed_at,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewUser {
    #[serde(default)]
    pub remark: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Optional slug; generated when omitted.
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUser {
    #[serde(default)]
    pub remark: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub token: Option<String>,
}

/// A user plus the plaintext subscription slug.
pub struct CreatedUser {
    pub user: UserSummary,
    pub token: String,
}

fn default_true() -> bool {
    true
}

pub async fn list(conn: &DatabaseConnection) -> Result<Vec<UserSummary>, DbErr> {
    let rows = users::Entity::find()
        .order_by_desc(users::Column::Id)
        .all(conn)
        .await?;
    Ok(rows.into_iter().map(UserSummary::from).collect())
}

pub async fn get(conn: &DatabaseConnection, id: i64) -> Result<Option<UserSummary>, DbErr> {
    Ok(users::Entity::find_by_id(id)
        .one(conn)
        .await?
        .map(UserSummary::from))
}

/// Resolves a subscription slug to its user.
pub async fn find_by_token(
    conn: &DatabaseConnection,
    token: &str,
) -> Result<Option<User>, DbErr> {
    users::Entity::find()
        .filter(users::Column::Token.eq(token))
        .one(conn)
        .await
}

/// True when no other user already owns `token`.
pub async fn token_available(
    conn: &DatabaseConnection,
    token: &str,
    except_id: Option<i64>,
) -> Result<bool, DbErr> {
    let existing = users::Entity::find()
        .filter(users::Column::Token.eq(token))
        .one(conn)
        .await?;
    Ok(match existing {
        Some(user) => Some(user.id) == except_id,
        None => true,
    })
}

pub async fn create(conn: &DatabaseConnection, input: &NewUser) -> Result<CreatedUser, SlugError> {
    let token = resolve_slug(conn, input.token.as_deref(), None).await?;

    let created = users::ActiveModel {
        remark: Set(input.remark.trim().to_owned()),
        enabled: Set(input.enabled),
        token: Set(token.clone()),
        last_accessed_at: Set(None),
        access_count: Set(0),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(conn)
    .await?;

    Ok(CreatedUser {
        user: UserSummary::from(created),
        token,
    })
}

pub async fn update(
    conn: &DatabaseConnection,
    id: i64,
    input: &UpdateUser,
) -> Result<Option<UserSummary>, SlugError> {
    let Some(existing) = users::Entity::find_by_id(id).one(conn).await? else {
        return Ok(None);
    };

    let mut active: users::ActiveModel = existing.into();
    if let Some(remark) = input.remark.as_deref() {
        active.remark = Set(remark.trim().to_owned());
    }
    if let Some(enabled) = input.enabled {
        active.enabled = Set(enabled);
    }
    if let Some(token) = input.token.as_deref() {
        let token = token.trim();
        if !token.is_empty() {
            active.token = Set(resolve_slug(conn, Some(token), Some(id)).await?);
        }
    }
    active.updated_at = Set(Utc::now());

    Ok(Some(UserSummary::from(active.update(conn).await?)))
}

/// Generates a fresh subscription slug, immediately revoking the previous link.
pub async fn reset_token(
    conn: &DatabaseConnection,
    id: i64,
) -> Result<Option<(UserSummary, String)>, SlugError> {
    let Some(existing) = users::Entity::find_by_id(id).one(conn).await? else {
        return Ok(None);
    };

    let token = resolve_slug(conn, None, Some(id)).await?;
    let mut active: users::ActiveModel = existing.into();
    active.token = Set(token.clone());
    active.updated_at = Set(Utc::now());

    let updated = active.update(conn).await?;
    Ok(Some((UserSummary::from(updated), token)))
}

/// Returns `requested` when it is free, otherwise a generated slug.
///
/// A caller-supplied slug that is already taken is an error rather than a silent
/// rewrite: handing back a different link than the one asked for would be a
/// surprise, and the operator can pick another name.
pub async fn resolve_slug(
    conn: &DatabaseConnection,
    requested: Option<&str>,
    except_id: Option<i64>,
) -> Result<String, SlugError> {
    if let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        if token_available(conn, requested, except_id).await? {
            return Ok(requested.to_owned());
        }
        return Err(SlugError::Taken(requested.to_owned()));
    }

    // A generated slug is long enough that collisions are practically impossible,
    // but retry anyway rather than surfacing a database error.
    for _ in 0..16 {
        let candidate = random_secret();
        if token_available(conn, &candidate, except_id).await? {
            return Ok(candidate);
        }
    }
    Err(SlugError::Exhausted)
}

/// Why a subscription slug could not be assigned.
#[derive(Debug)]
pub enum SlugError {
    /// Another user already owns the requested slug.
    Taken(String),
    /// No free slug could be generated.
    Exhausted,
    Db(DbErr),
}

impl From<DbErr> for SlugError {
    fn from(error: DbErr) -> Self {
        Self::Db(error)
    }
}

impl std::fmt::Display for SlugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Taken(slug) => write!(f, "订阅路径 “{slug}” 已被占用"),
            Self::Exhausted => write!(f, "无法生成唯一的订阅路径，请重试"),
            Self::Db(error) => write!(f, "{error}"),
        }
    }
}

pub async fn delete(conn: &DatabaseConnection, id: i64) -> Result<bool, DbErr> {
    super::credentials::delete_for_user(conn, id).await?;
    let changed = users::Entity::delete_by_id(id).exec(conn).await?.rows_affected;
    Ok(changed > 0)
}

pub async fn count(conn: &DatabaseConnection) -> Result<i64, DbErr> {
    use sea_orm::PaginatorTrait;
    users::Entity::find().count(conn).await.map(|count| count as i64)
}

pub async fn count_enabled(conn: &DatabaseConnection) -> Result<i64, DbErr> {
    use sea_orm::PaginatorTrait;
    users::Entity::find()
        .filter(users::Column::Enabled.eq(true))
        .count(conn)
        .await
        .map(|count| count as i64)
}

/// Records a successful subscription fetch for the dashboard.
pub async fn record_access(conn: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    let Some(existing) = users::Entity::find_by_id(id).one(conn).await? else {
        return Ok(());
    };

    let next_count = existing.access_count + 1;
    let mut active: users::ActiveModel = existing.into();
    active.last_accessed_at = Set(Some(Utc::now()));
    active.access_count = Set(next_count);
    active.update(conn).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_slug_is_url_safe() {
        let slug = random_secret();
        assert!(!slug.is_empty());
        assert!(slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
