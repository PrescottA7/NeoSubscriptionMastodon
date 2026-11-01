use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::auth::RequireAuth;
use crate::db::users::{self, CreatedUser, NewUser, UpdateUser, UserSummary};
use crate::error::{RequestError, RequestResult};
use crate::routes::extract::ApiJson;
use crate::state::AppState;
use crate::validate::Validator;

/// A one-time view containing the plaintext subscription token.
#[derive(Serialize)]
struct ProvisionedUser {
    user: UserSummary,
    token: String,
    subscription_path: String,
    subscription_url: String,
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/{id}", get(get_user).patch(update_user).delete(delete_user))
        .route("/api/users/{id}/reset-link", post(reset_link))
        .route("/api/users/{id}/credentials", get(list_user_credentials))
        .route_layer(axum::middleware::from_extractor_with_state::<RequireAuth, AppState>(
            state.clone(),
        ))
}

async fn list_users(State(state): State<AppState>) -> RequestResult<Json<Vec<UserSummary>>> {
    Ok(Json(users::list(&state.db).await?))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<UserSummary>> {
    users::get(&state.db, id)
        .await?
        .map(Json)
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))
}

async fn create_user(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<NewUser>,
) -> RequestResult<Response> {
    validate_new_user(&payload)?;

    let CreatedUser { user, token } = users::create(&state.db, &payload).await?;
    let settings = state.settings().await?;

    Ok((
        StatusCode::CREATED,
        Json(ProvisionedUser {
            subscription_path: format!("{}/{}", settings.subscription_prefix, token),
            subscription_url: format!("{}/{}", settings.subscription_prefix, token),
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
) -> RequestResult<Json<UserSummary>> {
    validate_user_update(&payload)?;

    users::update(&state.db, id, &payload)
        .await?
        .map(Json)
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))
}

/// Rule set mirrored by `frontend/src/validation.ts::validateUserForm`.
fn validate_new_user(payload: &NewUser) -> Result<(), RequestError> {
    let mut validator = Validator::new();
    validator.optional_text("remark", "备注", Some(payload.remark.as_str()), 64);
    validator.finish()
}

fn validate_user_update(payload: &UpdateUser) -> Result<(), RequestError> {
    let mut validator = Validator::new();
    validator.optional_text("remark", "备注", payload.remark.as_deref(), 64);
    validator.finish()
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

/// Issues a fresh subscription token, revoking the previous link immediately.
async fn reset_link(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> RequestResult<Json<ProvisionedUser>> {
    let (user, token) = users::reset_token(&state.db, id)
        .await?
        .ok_or_else(|| RequestError::not_found(format!("用户 {id} 不存在")))?;
    let settings = state.settings().await?;

    Ok(Json(ProvisionedUser {
        subscription_path: format!("{}/{}", settings.subscription_prefix, token),
        subscription_url: format!("{}/{}", settings.subscription_prefix, token),
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
            token_hint: "abcd1234".to_owned(),
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
