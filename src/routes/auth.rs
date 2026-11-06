// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use serde::{Deserialize, Serialize};

use crate::auth::{self, RequireAuth, SESSION_COOKIE_NAME, SESSION_MAX_AGE_SECONDS};
use crate::db::admins::{AdminSummary, NewAdmin};
use crate::error::{RequestError, RequestResult};
use crate::routes::extract::ApiJson;
use crate::state::AppState;

#[derive(Serialize)]
struct SetupStatusResponse {
    needs_setup: bool,
    site_name: String,
}

#[derive(Serialize)]
struct LoginResponse {
    admin: AdminSummary,
}

#[derive(Deserialize)]
struct PasswordChange {
    current_password: String,
    new_password: String,
}

pub fn routes(state: &AppState) -> Router<AppState> {
    let protected = Router::new()
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/auth/password", post(change_password))
        .route_layer(axum::middleware::from_extractor_with_state::<
            RequireAuth,
            AppState,
        >(state.clone()));

    let public = Router::new()
        .route("/api/auth/setup-status", get(setup_status))
        .route("/api/auth/setup", post(setup))
        .route("/api/auth/login", post(login));

    public.merge(protected)
}

async fn setup_status(State(state): State<AppState>) -> RequestResult<Json<SetupStatusResponse>> {
    let needs_setup = state.db_admin().await?.is_none();
    let settings = state.settings().await?;

    Ok(Json(SetupStatusResponse {
        needs_setup,
        site_name: settings.site_name,
    }))
}

/// Creates the administrator on first run. Returns 409 once one exists.
async fn setup(
    State(state): State<AppState>,
    cookies: CookieJar,
    ApiJson(payload): ApiJson<NewAdmin>,
) -> RequestResult<Response> {
    validate_credentials(&payload.username, &payload.password)?;

    let Some(admin) = crate::db::admins::create_admin(&state.db, &payload).await? else {
        return Err(RequestError::conflict("管理员已初始化"));
    };

    let cookie = issue_cookie(&state, cookies).await?;
    Ok((cookie, Json(LoginResponse { admin })).into_response())
}

async fn login(
    State(state): State<AppState>,
    cookies: CookieJar,
    ApiJson(payload): ApiJson<NewAdmin>,
) -> RequestResult<Response> {
    let Some(admin) =
        crate::db::admins::verify_login(&state.db, &payload.username, &payload.password).await?
    else {
        return Err(RequestError::unauthorized("用户名或密码错误"));
    };

    let cookie = issue_cookie(&state, cookies).await?;
    Ok((cookie, Json(LoginResponse { admin })).into_response())
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    cookies: CookieJar,
) -> RequestResult<Response> {
    // Revoke the stored session first so the token cannot be replayed even if
    // the browser keeps a copy.
    if let Some(token) = auth::extract_session_token(&headers) {
        state.sessions.revoke(&token).await?;
    }

    // A cookie is only removed when the response carries an expiry in the past.
    let cookie = auth::clearing_cookie(state.config.cookie_secure, &cookie_path(&state));

    Ok((cookies.add(cookie), StatusCode::NO_CONTENT).into_response())
}

async fn me(RequireAuth(admin): RequireAuth) -> Json<AdminSummary> {
    Json(admin)
}

async fn change_password(
    State(state): State<AppState>,
    RequireAuth(admin): RequireAuth,
    ApiJson(payload): ApiJson<PasswordChange>,
) -> RequestResult<StatusCode> {
    validate_credentials(&admin.username, &payload.new_password)?;

    let verified =
        crate::db::admins::verify_login(&state.db, &admin.username, &payload.current_password)
            .await?;
    if verified.is_none() {
        return Err(RequestError::unauthorized("当前密码不正确"));
    }

    crate::db::admins::update_admin(
        &state.db,
        &NewAdmin {
            username: admin.username,
            password: payload.new_password,
        },
    )
    .await?;

    // Revoke every session so other devices must log in again.
    state.sessions.revoke_all().await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Cookie path matching the admin prefix, so the session never rides along on a
/// subscription request.
fn cookie_path(state: &AppState) -> String {
    match state.admin_prefix() {
        "" => "/".to_owned(),
        prefix => prefix.to_owned(),
    }
}

/// Issues a session token and returns the updated cookie jar.
async fn issue_cookie(state: &AppState, cookies: CookieJar) -> RequestResult<CookieJar> {
    let admin = state
        .db_admin()
        .await?
        .ok_or_else(|| RequestError::unauthorized("管理员不存在"))?;

    let token = state.sessions.issue(&admin).await?;
    let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
        // Scoping the cookie to the admin prefix keeps the session off the
        // subscription paths when both live under the same host.
        .path(cookie_path(state))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(state.config.cookie_secure)
        // Without Max-Age the cookie dies when the browser closes, which both
        // breaks "stay logged in" and hides the real cause of a failed login.
        .max_age(time::Duration::seconds(SESSION_MAX_AGE_SECONDS))
        .expires(time::OffsetDateTime::now_utc() + time::Duration::seconds(SESSION_MAX_AGE_SECONDS))
        .build();

    Ok(cookies.add(cookie))
}

/// Enforces the minimum administrator credential policy.
#[allow(clippy::result_large_err)]
fn validate_credentials(username: &str, password: &str) -> Result<(), RequestError> {
    if username.trim().len() < 3 {
        return Err(RequestError::bad_request("用户名至少 3 个字符"));
    }
    if password.len() < 12 {
        return Err(RequestError::bad_request("密码至少 12 个字符"));
    }
    Ok(())
}

/// Exposes the session cookie name so the frontend can clear it.
#[allow(dead_code)]
pub fn cookie_name() -> &'static str {
    SESSION_COOKIE_NAME
}

#[allow(dead_code)]
fn unused_header_guard() -> header::HeaderName {
    header::SET_COOKIE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_username_is_rejected() {
        assert!(validate_credentials("ab", "longenoughpassword").is_err());
    }

    #[test]
    fn short_password_is_rejected() {
        assert!(validate_credentials("admin", "short").is_err());
    }

    #[test]
    fn valid_credentials_pass() {
        assert!(validate_credentials("admin", "averylongpassword").is_ok());
    }
}
