use axum::{
    extract::FromRequestParts,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set};
use serde::Serialize;

use crate::db::{admins::AdminSummary, entities::sessions};
use crate::utils::{hash_secret, random_secret};

pub const SESSION_COOKIE_NAME: &str = "nsm_session";
/// Administrator sessions last 30 days.
pub const SESSION_MAX_AGE_SECONDS: i64 = 60 * 60 * 24 * 30;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

/// Reads the session token out of a `Cookie` header.
pub fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let prefix = format!("{SESSION_COOKIE_NAME}=");

    raw.split(';').find_map(|cookie| {
        cookie
            .trim()
            .strip_prefix(&prefix)
            .map(|value| value.to_owned())
            .filter(|value| !value.is_empty())
    })
}

/// Creates, resolves, and revokes administrator sessions.
#[derive(Clone)]
pub struct SessionController {
    db: DatabaseConnection,
}

impl SessionController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Issues a session token, storing only its hash.
    pub async fn issue(&self, _admin: &AdminSummary) -> Result<String, DbErr> {
        let token = random_secret();
        let now = Utc::now();

        sessions::ActiveModel {
            token_hash: Set(hash_secret(&token)),
            expires_at: Set(now + Duration::seconds(SESSION_MAX_AGE_SECONDS)),
            created_at: Set(now),
        }
        .insert(&self.db)
        .await?;

        self.purge_expired().await?;
        Ok(token)
    }

    /// Returns `true` when the token maps to a live session.
    pub async fn resolve(&self, token: &str) -> Result<bool, DbErr> {
        let token_hash = hash_secret(token);
        let Some(session) = sessions::Entity::find_by_id(token_hash).one(&self.db).await? else {
            return Ok(false);
        };

        if session.expires_at <= Utc::now() {
            self.revoke(token).await?;
            return Ok(false);
        }

        Ok(true)
    }

    pub async fn revoke(&self, token: &str) -> Result<(), DbErr> {
        sessions::Entity::delete_by_id(hash_secret(token))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn revoke_all(&self) -> Result<(), DbErr> {
        sessions::Entity::delete_many().exec(&self.db).await?;
        Ok(())
    }

    async fn purge_expired(&self) -> Result<(), DbErr> {
        sessions::Entity::delete_many()
            .filter(sessions::Column::ExpiresAt.lt(Utc::now()))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn active_count(&self) -> Result<i64, DbErr> {
        use sea_orm::PaginatorTrait;
        sessions::Entity::find()
            .filter(sessions::Column::ExpiresAt.gt(Utc::now()))
            .count(&self.db)
            .await
            .map(|count| count as i64)
    }
}

/// Extractor that rejects unauthenticated requests with a 401 JSON body.
pub struct RequireAuth(pub AdminSummary);

impl FromRequestParts<crate::state::AppState> for RequireAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &crate::state::AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_session_token(&parts.headers).ok_or_else(unauthorized)?;

        let live = state
            .sessions
            .resolve(&token)
            .await
            .map_err(internal_error)?;
        if !live {
            return Err(unauthorized());
        }

        let admin = state
            .db_admin()
            .await
            .map_err(internal_error)?
            .ok_or_else(unauthorized)?;

        Ok(Self(admin))
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "登录状态已失效，请重新登录".to_owned(),
        }),
    )
        .into_response()
}

fn internal_error(error: impl std::fmt::Display) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: format!("internal server error: {error}"),
        }),
    )
        .into_response()
}

/// Expiry timestamp for a freshly issued cookie.
pub fn session_expiry() -> DateTime<Utc> {
    Utc::now() + Duration::seconds(SESSION_MAX_AGE_SECONDS)
}

/// Builds the cookie used to clear a session in the browser.
///
/// A cookie is only removed when the response carries `Max-Age=0` (and/or an
/// `Expires` in the past); setting an empty value alone leaves it in place.
pub fn clearing_cookie(secure: bool) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(secure)
        .max_age(time::Duration::seconds(0))
        .expires(time::OffsetDateTime::UNIX_EPOCH)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn token_is_extracted_from_cookie_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=dark; nsm_session=abc123; other=1"),
        );
        assert_eq!(extract_session_token(&headers).as_deref(), Some("abc123"));
    }

    #[test]
    fn missing_cookie_yields_none() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("theme=dark"));
        assert!(extract_session_token(&headers).is_none());
    }

    #[test]
    fn absent_header_yields_none() {
        assert!(extract_session_token(&HeaderMap::new()).is_none());
    }

    #[test]
    fn empty_cookie_value_is_ignored() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("nsm_session=; x=1"),
        );
        assert!(extract_session_token(&headers).is_none());
    }

    #[test]
    fn clearing_cookie_expires_immediately() {
        let rendered = clearing_cookie(false).to_string();
        let lowered = rendered.to_ascii_lowercase();

        assert_eq!(clearing_cookie(false).name(), SESSION_COOKIE_NAME);
        assert!(lowered.contains("max-age=0"), "{rendered}");
        assert!(lowered.contains("expires=thu, 01 jan 1970"), "{rendered}");
        assert!(lowered.contains("httponly"), "{rendered}");
    }

    #[test]
    fn clearing_cookie_is_secure_when_configured() {
        let rendered = clearing_cookie(true).to_string().to_ascii_lowercase();
        assert!(rendered.contains("secure"), "{rendered}");
    }
}
