// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;

use crate::auth::RequireAuth;
use crate::db::users::{self, CreatedUser, NewUser, SlugError, UpdateUser, UserSummary};
use crate::error::{FieldError, RequestError, RequestResult};
use crate::routes::extract::ApiJson;
use crate::state::AppState;
use crate::validate::Validator;

/// Slug conflicts are user-visible field problems, not internal failures.
fn slug_error(error: SlugError) -> RequestError {
    match error {
        SlugError::Taken(slug) => RequestError::validation(vec![FieldError {
            field: "token".to_owned(),
            message: format!("订阅路径 “{slug}” 已被占用"),
        }]),
        SlugError::Exhausted => RequestError::internal(crate::error::internal(error.to_string())),
        SlugError::Db(inner) => RequestError::internal(inner.into()),
    }
}

/// Payload for a user, including the link the operator can hand out.
#[derive(Serialize)]
struct ProvisionedUser {
    user: UserSummary,
    token: String,
    subscription_path: String,
    subscription_url: String,
}

/// User row for the admin UI: the summary plus the path it resolves to.
#[derive(Serialize)]
struct UserView {
    #[serde(flatten)]
    user: UserSummary,
    subscription_path: String,
}

impl UserView {
    fn new(state: &AppState, user: UserSummary) -> Self {
        let subscription_path = format!("{}/{}", state.subscription_prefix(), user.token);
        Self {
            user,
            subscription_path,
        }
    }
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route(
            "/api/users/{id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
        .route("/api/users/{id}/reset-link", post(reset_link))
        .route("/api/users/{id}/credentials", get(list_user_credentials))
        .route_layer(axum::middleware::from_extractor_with_state::<
            RequireAuth,
            AppState,
        >(state.clone()))
}

async fn list_users(State(state): State<AppState>) -> RequestResult<Json<Vec<UserView>>> {
    let rows = users::list(&state.db).await?;
    Ok(Json(
        rows.into_iter()
            .map(|user| UserView::new(&state, user))
            .collect(),
    ))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<UserView>> {
    users::get(&state.db, id)
        .await?
        .map(|user| Json(UserView::new(&state, user)))
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))
}

async fn create_user(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<NewUser>,
) -> RequestResult<Response> {
    validate_new_user(&payload)?;

    let CreatedUser { user, token } = users::create(&state.db, &payload)
        .await
        .map_err(slug_error)?;
    let prefix = state.subscription_prefix();

    Ok((
        StatusCode::CREATED,
        Json(ProvisionedUser {
            subscription_path: format!("{prefix}/{token}"),
            subscription_url: format!("{prefix}/{token}"),
            user,
            token,
        }),
    )
        .into_response())
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<UpdateUser>,
) -> RequestResult<Json<UserView>> {
    validate_user_update(&payload)?;

    users::update(&state.db, id, &payload)
        .await
        .map_err(slug_error)?
        .map(|user| Json(UserView::new(&state, user)))
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))
}

/// Rule set mirrored by `frontend/src/validation.ts::validateUserForm`.
fn validate_new_user(payload: &NewUser) -> Result<(), RequestError> {
    let mut validator = Validator::new();
    validator.optional_text("remark", "备注", Some(payload.remark.as_str()), 64);
    validate_slug(&mut validator, payload.token.as_deref());
    validator.finish()
}

fn validate_user_update(payload: &UpdateUser) -> Result<(), RequestError> {
    let mut validator = Validator::new();
    validator.optional_text("remark", "备注", payload.remark.as_deref(), 64);
    validate_slug(&mut validator, payload.token.as_deref());
    validator.finish()
}

/// Subscription slugs appear in a URL path, so they are limited to the
/// characters that need no escaping.
fn validate_slug(validator: &mut Validator, token: Option<&str>) {
    let Some(token) = token.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };

    validator.require(token.len() <= 64, "token", "订阅路径最多 64 个字符");
    validator.require(
        token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "token",
        "订阅路径只能包含字母、数字、- 和 _",
    );
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<StatusCode> {
    if users::delete(&state.db, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(RequestError::not_found(format!("用户 {id} 不存在")))
    }
}

/// Issues a fresh subscription slug, revoking the previous link immediately.
async fn reset_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<ProvisionedUser>> {
    let (user, token) = users::reset_token(&state.db, id)
        .await
        .map_err(slug_error)?
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))?;
    let prefix = state.subscription_prefix();

    Ok(Json(ProvisionedUser {
        subscription_path: format!("{prefix}/{token}"),
        subscription_url: format!("{prefix}/{token}"),
        user,
        token,
    }))
}

/// Every node credential this user holds, across all nodes.
async fn list_user_credentials(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<Vec<crate::db::credentials::CredentialView>>> {
    Ok(Json(
        crate::db::credentials::list_for_user(&state.db, &state.key, id).await?,
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn provisioning_payload_omits_credentials() {
        // Serialised `UserSummary` must never contain proxy credentials: they
        // are per node and only returned from the credential endpoints.
        let summary = crate::db::users::UserSummary {
            id: 1,
            remark: String::new(),
            enabled: true,
            token: "alice".to_owned(),
            access_count: 0,
            last_accessed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let json = serde_json::to_string(&summary).expect("serialize");
        assert!(!json.contains("password"));
        assert!(!json.contains("uuid"));
    }
}
