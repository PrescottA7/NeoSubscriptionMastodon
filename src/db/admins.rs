// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use chrono::Utc;
use rand::RngCore;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};

use super::entities::admins;

pub type Admin = admins::Model;

/// Administrator identity handed to the frontend; never includes the hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSummary {
    pub id: i64,
    pub username: String,
}

impl From<Admin> for AdminSummary {
    fn from(admin: Admin) -> Self {
        Self {
            id: admin.id,
            username: admin.username,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewAdmin {
    pub username: String,
    pub password: String,
}

pub async fn has_admin(conn: &DatabaseConnection) -> Result<bool, DbErr> {
    Ok(admins::Entity::find().one(conn).await?.is_some())
}

pub async fn get_admin(conn: &DatabaseConnection) -> Result<Option<AdminSummary>, DbErr> {
    Ok(admins::Entity::find()
        .order_by_asc(admins::Column::Id)
        .one(conn)
        .await?
        .map(AdminSummary::from))
}

/// Creates the administrator. Returns `None` when one already exists, so the
/// setup endpoint can stay idempotent-safe.
pub async fn create_admin(
    conn: &DatabaseConnection,
    input: &NewAdmin,
) -> Result<Option<AdminSummary>, DbErr> {
    if has_admin(conn).await? {
        return Ok(None);
    }

    let created = admins::ActiveModel {
        id: Set(1),
        username: Set(input.username.trim().to_owned()),
        password_hash: Set(hash_password(&input.password)?),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    }
    .insert(conn)
    .await?;

    Ok(Some(AdminSummary::from(created)))
}

pub async fn update_admin(
    conn: &DatabaseConnection,
    input: &NewAdmin,
) -> Result<Option<AdminSummary>, DbErr> {
    let Some(existing) = admins::Entity::find_by_id(1).one(conn).await? else {
        return Ok(None);
    };

    let mut active: admins::ActiveModel = existing.into();
    active.username = Set(input.username.trim().to_owned());
    if !input.password.is_empty() {
        active.password_hash = Set(hash_password(&input.password)?);
    }
    active.updated_at = Set(Utc::now());

    Ok(Some(AdminSummary::from(active.update(conn).await?)))
}

/// Verifies credentials, returning the admin summary on success.
pub async fn verify_login(
    conn: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<Option<AdminSummary>, DbErr> {
    let admin = admins::Entity::find()
        .filter(admins::Column::Username.eq(username.trim()))
        .one(conn)
        .await?;

    let Some(admin) = admin else {
        return Ok(None);
    };

    if verify_password(&admin.password_hash, password)? {
        Ok(Some(AdminSummary::from(admin)))
    } else {
        Ok(None)
    }
}

fn hash_password(password: &str) -> Result<String, DbErr> {
    let mut random_src = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut random_src);
    let salt = SaltString::encode_b64(&random_src)
        .map_err(|error| DbErr::Custom(format!("failed to build salt: {error}")))?;

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| DbErr::Custom(format!("failed to hash password: {error}")))
}

fn verify_password(stored_hash: &str, password: &str) -> Result<bool, DbErr> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|error| DbErr::Custom(format!("failed to parse password hash: {error}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hashes_verify() {
        let hash = hash_password("correct horse").expect("hash");
        assert!(verify_password(&hash, "correct horse").expect("verify"));
        assert!(!verify_password(&hash, "wrong").expect("verify"));
    }

    #[test]
    fn hashes_are_salted() {
        let first = hash_password("same").expect("hash");
        let second = hash_password("same").expect("hash");
        assert_ne!(first, second);
    }

    #[test]
    fn malformed_hash_is_rejected() {
        assert!(verify_password("not-a-hash", "x").is_err());
    }
}
