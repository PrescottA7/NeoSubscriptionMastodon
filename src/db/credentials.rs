//! Per-node, per-user proxy credentials.
//!
//! Each proxy server keeps its own account list, so the same person can be
//! `alice` on one node and something else on another. A row here is both the
//! credential and the authorisation: a user only receives a node when a row
//! exists for the pair.

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::EncryptionKey;
use crate::utils::random_secret;

use super::entities::{node_credentials, nodes, users};

pub type Credential = node_credentials::Model;

/// Credential row as exposed to the admin API.
///
/// `uuid` and `password` are plaintext here: the admin has to copy them into the
/// proxy server's client list, and the admin API already requires a session.
#[derive(Debug, Clone, Serialize)]
pub struct CredentialView {
    pub node_id: i64,
    pub user_id: i64,
    pub uuid: String,
    pub password: String,
    pub email: String,
    pub enabled: bool,
}

/// Per-node credential submitted from the admin UI.
#[derive(Debug, Clone, Deserialize)]
pub struct CredentialInput {
    pub user_id: i64,
    /// Leave empty to generate a fresh UUID.
    #[serde(default)]
    pub uuid: Option<String>,
    /// Leave empty to generate a fresh password.
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// A credential resolved for rendering: plaintext and ready to substitute.
#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub uuid: String,
    pub password: String,
    pub email: String,
}

/// Generates the values a fresh credential row starts with.
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_password() -> String {
    random_secret()
}

/// Default account label, matching the `email` field proxy servers display.
pub fn default_email(remark: &str, username_hint: &str, node_name: &str) -> String {
    let label = if !remark.is_empty() { remark } else { username_hint };
    let label: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let label = label.trim_matches('-');
    let label = if label.is_empty() { "user" } else { label };
    let node: String = node_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let node = node.trim_matches('-');
    let node = if node.is_empty() { "node" } else { node };
    format!("{label}@{node}")
}

pub async fn list_for_node(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    node_id: i64,
) -> Result<Vec<CredentialView>, DbErr> {
    let rows = node_credentials::Entity::find()
        .filter(node_credentials::Column::NodeId.eq(node_id))
        .order_by_asc(node_credentials::Column::UserId)
        .all(conn)
        .await?;

    rows.into_iter()
        .map(|row| to_view(key, row))
        .collect::<Result<Vec<_>, _>>()
}

pub async fn list_for_user(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    user_id: i64,
) -> Result<Vec<CredentialView>, DbErr> {
    let rows = node_credentials::Entity::find()
        .filter(node_credentials::Column::UserId.eq(user_id))
        .order_by_asc(node_credentials::Column::NodeId)
        .all(conn)
        .await?;

    rows.into_iter()
        .map(|row| to_view(key, row))
        .collect::<Result<Vec<_>, _>>()
}

/// Replaces the credential set of one node.
///
/// Rows that disappear from `inputs` are revoked, which immediately removes the
/// node from those users' subscriptions.
pub async fn replace_for_node(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    node_id: i64,
    node_name: &str,
    inputs: &[CredentialInput],
) -> Result<(), DbErr> {
    let node_name = node_name.to_owned();
    node_credentials::Entity::delete_many()
        .filter(node_credentials::Column::NodeId.eq(node_id))
        .exec(conn)
        .await?;

    for input in inputs {
        let uuid = match input.uuid.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => value.to_owned(),
            _ => generate_uuid(),
        };
        let password = match input.password.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => value.to_owned(),
            _ => generate_password(),
        };

        let email = match input.email.as_deref().map(str::trim) {
            Some(value) if !value.is_empty() => value.to_owned(),
            _ => default_email("", &format!("user{}", input.user_id), &node_name),
        };

        node_credentials::ActiveModel {
            node_id: Set(node_id),
            user_id: Set(input.user_id),
            uuid: Set(uuid),
            password: Set(key
                .encrypt(&password)
                .map_err(|error| DbErr::Custom(error.to_string()))?),
            email: Set(email),
            enabled: Set(input.enabled),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(conn)
        .await?;
    }

    Ok(())
}

/// Regenerates the secret half of one credential row.
pub async fn rotate(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    node_id: i64,
    user_id: i64,
) -> Result<Option<CredentialView>, DbErr> {
    let Some(existing) = node_credentials::Entity::find()
        .filter(node_credentials::Column::NodeId.eq(node_id))
        .filter(node_credentials::Column::UserId.eq(user_id))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let mut active: node_credentials::ActiveModel = existing.into();
    active.uuid = Set(generate_uuid());
    active.password = Set(key
        .encrypt(&generate_password())
        .map_err(|error| DbErr::Custom(error.to_string()))?);
    active.updated_at = Set(Utc::now());

    Ok(Some(to_view(key, active.update(conn).await?)?))
}

/// Enabled credentials of one user, joined with the node they belong to.
///
/// The join enforces that a disabled node, a disabled user, or a disabled
/// credential all make the pair disappear from the subscription.
pub async fn resolved_for_user(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    user_id: i64,
) -> Result<Vec<(nodes::Model, ResolvedCredential)>, DbErr> {
    let rows = node_credentials::Entity::find()
        .filter(node_credentials::Column::UserId.eq(user_id))
        .filter(node_credentials::Column::Enabled.eq(true))
        .all(conn)
        .await?;

    let mut resolved = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(node) = nodes::Entity::find_by_id(row.node_id).one(conn).await? else {
            continue;
        };
        if !node.enabled {
            continue;
        }
        let password = key
            .decrypt(&row.password)
            .map_err(|error| DbErr::Custom(format!("credential {}/{}: {error}", row.node_id, row.user_id)))?;
        resolved.push((
            node,
            ResolvedCredential {
                uuid: row.uuid,
                password,
                email: row.email,
            },
        ));
    }

    resolved.sort_by(|a, b| {
        a.0.sort_order
            .cmp(&b.0.sort_order)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });

    Ok(resolved)
}

/// Credentials of a single pair, used by the admin render endpoint.
pub async fn resolved_for_pair(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    node_id: i64,
    user_id: i64,
) -> Result<Option<ResolvedCredential>, DbErr> {
    let Some(row) = node_credentials::Entity::find()
        .filter(node_credentials::Column::NodeId.eq(node_id))
        .filter(node_credentials::Column::UserId.eq(user_id))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };

    let password = key
        .decrypt(&row.password)
        .map_err(|error| DbErr::Custom(format!("credential {node_id}/{user_id}: {error}")))?;

    Ok(Some(ResolvedCredential {
        uuid: row.uuid,
        password,
        email: row.email,
    }))
}

pub async fn delete_for_user(conn: &DatabaseConnection, user_id: i64) -> Result<(), DbErr> {
    node_credentials::Entity::delete_many()
        .filter(node_credentials::Column::UserId.eq(user_id))
        .exec(conn)
        .await?;
    Ok(())
}

pub async fn delete_for_node(conn: &DatabaseConnection, node_id: i64) -> Result<(), DbErr> {
    node_credentials::Entity::delete_many()
        .filter(node_credentials::Column::NodeId.eq(node_id))
        .exec(conn)
        .await?;
    Ok(())
}

fn to_view(key: &EncryptionKey, row: Credential) -> Result<CredentialView, DbErr> {
    let password = key
        .decrypt(&row.password)
        .map_err(|error| DbErr::Custom(error.to_string()))?;

    Ok(CredentialView {
        node_id: row.node_id,
        user_id: row.user_id,
        uuid: row.uuid,
        password,
        email: row.email,
        enabled: row.enabled,
    })
}

/// Convenience used by the UI to prefill a new credential row.
pub async fn suggested_for_node(
    conn: &DatabaseConnection,
    key: &EncryptionKey,
    node_id: i64,
    node_name: &str,
) -> Result<Vec<CredentialView>, DbErr> {
    let all_users = users::Entity::find()
        .order_by_asc(users::Column::Id)
        .all(conn)
        .await?;
    let existing = list_for_node(conn, key, node_id).await?;

    let mut suggested = Vec::new();
    for user in all_users {
        if let Some(found) = existing.iter().find(|row| row.user_id == user.id) {
            suggested.push(found.clone());
            continue;
        }
        suggested.push(CredentialView {
            node_id,
            user_id: user.id,
            uuid: generate_uuid(),
            password: generate_password(),
            email: default_email(&user.remark, &format!("user{}", user.id), node_name),
            enabled: true,
        });
    }

    Ok(suggested)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_uuids_are_unique() {
        let first = generate_uuid();
        let second = generate_uuid();
        assert_ne!(first, second);
        assert_eq!(first.len(), 36);
    }

    #[test]
    fn generated_passwords_are_long_enough() {
        assert!(generate_password().len() >= 16);
    }

    #[test]
    fn email_defaults_to_user_at_node() {
        assert_eq!(
            default_email("Alice", "user1", "Tokyo 01"),
            "Alice@tokyo-01"
        );
    }

    #[test]
    fn email_falls_back_when_labels_are_empty() {
        assert_eq!(default_email("", "user7", "!!!"), "user7@node");
        assert_eq!(default_email("  ", "", ""), "user@node");
    }
}
