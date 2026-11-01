use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};

use crate::utils::{hash_secret, random_secret};

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
    pub token_hint: String,
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
            token_hint: user.token_hint,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUser {
    #[serde(default)]
    pub remark: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// A user plus the one-time plaintext subscription token.
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

/// Resolves a subscription token to its user.
pub async fn find_by_token(
    conn: &DatabaseConnection,
    token_hash: &str,
) -> Result<Option<User>, DbErr> {
    users::Entity::find()
        .filter(users::Column::TokenHash.eq(token_hash))
        .one(conn)
        .await
}

pub async fn create(conn: &DatabaseConnection, input: &NewUser) -> Result<CreatedUser, DbErr> {
    let token = random_secret();

    let created = users::ActiveModel {
        remark: Set(input.remark.trim().to_owned()),
        enabled: Set(input.enabled),
        token_hash: Set(hash_secret(&token)),
        token_hint: Set(token_hint(&token)),
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
) -> Result<Option<UserSummary>, DbErr> {
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
    active.updated_at = Set(Utc::now());

    Ok(Some(UserSummary::from(active.update(conn).await?)))
}

/// Replaces the subscription token, immediately revoking the previous link.
pub async fn reset_token(
    conn: &DatabaseConnection,
    id: i64,
) -> Result<Option<(UserSummary, String)>, DbErr> {
    let Some(existing) = users::Entity::find_by_id(id).one(conn).await? else {
        return Ok(None);
    };

    let token = random_secret();
    let mut active: users::ActiveModel = existing.into();
    active.token_hash = Set(hash_secret(&token));
    active.token_hint = Set(token_hint(&token));
    active.updated_at = Set(Utc::now());

    let updated = active.update(conn).await?;
    Ok(Some((UserSummary::from(updated), token)))
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

/// First eight characters of a token, safe to show in listings.
fn token_hint(token: &str) -> String {
    token.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hint_is_a_short_prefix() {
        assert_eq!(token_hint("abcdefghijklmn"), "abcdefgh");
        assert_eq!(token_hint("abc"), "abc");
    }
}
