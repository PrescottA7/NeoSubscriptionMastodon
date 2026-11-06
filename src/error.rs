// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use anyhow::anyhow;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::error;

pub type RequestResult<T> = Result<T, RequestError>;

/// Message returned for unexpected failures. Deliberately generic: a 500 body
/// must never expose SQL, file paths, or internal identifiers.
pub const INTERNAL_MESSAGE: &str = "服务器内部错误，请稍后重试";

/// A single field-level problem, so the UI can point at the offending input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldError {
    /// Dotted path of the offending value, e.g. `port` or `params.tls.server_name`.
    pub field: String,
    /// Human readable, already localised description.
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Response body shared by every error path.
///
/// - `error` is always safe to show: either an explicitly chosen message or the
///   generic [`INTERNAL_MESSAGE`].
/// - `details` carries per-field validation problems, omitted when empty.
/// - `request_id` lets an operator correlate a user report with the server log,
///   which holds the real cause.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    details: Vec<FieldError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

/// Error wrapper.
///
/// Two distinct kinds:
/// - **Public** errors carry a message chosen by the handler, because it is
///   meaningful to the caller (validation, conflict, not found).
/// - **Internal** errors wrap any `anyhow::Error`; the cause is logged but the
///   response only exposes [`INTERNAL_MESSAGE`].
pub struct RequestError {
    /// Boxed because `Response` is large and this type is returned from every
    /// handler, so its size would otherwise inflate every `Result`.
    response: Box<Response>,
    cause: Option<anyhow::Error>,
    fields: Vec<FieldError>,
}

impl std::fmt::Debug for RequestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause.as_ref() {
            Some(error) => formatter.debug_tuple("RequestError").field(error).finish(),
            None => formatter
                .debug_struct("RequestError")
                .field("response", &self.response.status())
                .finish(),
        }
    }
}

impl RequestError {
    pub fn new(expected_response: impl IntoResponse) -> Self {
        Self::from_response(expected_response.into_response())
    }

    fn from_response(response: Response) -> Self {
        Self {
            cause: None,
            fields: Vec::new(),
            response: Box::new(response),
        }
    }

    /// The per-field problems carried by a validation error, for tests and
    /// callers that want to inspect them.
    pub fn fields(&self) -> &[FieldError] {
        &self.fields
    }

    // ---- public errors: the message is intended for the client ----

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::public(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::public(StatusCode::UNAUTHORIZED, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::public(StatusCode::FORBIDDEN, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::public(StatusCode::NOT_FOUND, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::public(StatusCode::CONFLICT, message)
    }

    /// A message that is deliberately shown to the user, for any status.
    pub fn public(status: StatusCode, message: impl Into<String>) -> Self {
        Self::body(status, message.into(), Vec::new())
    }

    /// Convenience for malformed bodies; keeps axum's status but normalises the
    /// body shape.
    pub fn json(status: StatusCode, message: impl Into<String>) -> Self {
        Self::public(status, message)
    }

    /// Validation failure with per-field detail. The summary is derived from the
    /// first problem so a client that ignores `details` still shows something.
    pub fn validation(details: Vec<FieldError>) -> Self {
        let summary = match details.first() {
            Some(first) if details.len() == 1 => first.message.clone(),
            Some(first) => format!("{}（共 {} 项）", first.message, details.len()),
            None => "请求参数不合法".to_owned(),
        };
        Self::body(StatusCode::UNPROCESSABLE_ENTITY, summary, details)
    }

    // ---- internal errors: the message is logged, never returned ----

    /// Builds an internal (500) error from any cause, tagging the response with
    /// a short id so a user report can be matched to the log entry.
    pub fn internal(cause: anyhow::Error) -> Self {
        let request_id: String = crate::utils::random_secret().chars().take(8).collect();

        // Full detail goes to the log only.
        error!(%request_id, error = %format!("{cause:#}"), "request failed");

        let body = ErrorBody {
            error: INTERNAL_MESSAGE.to_owned(),
            details: Vec::new(),
            request_id: Some(request_id),
        };

        Self {
            cause: Some(cause),
            fields: Vec::new(),
            response: Box::new((StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()),
        }
    }

    fn body(status: StatusCode, message: String, details: Vec<FieldError>) -> Self {
        Self {
            cause: None,
            fields: details.clone(),
            response: Box::new(
                (
                    status,
                    Json(ErrorBody {
                        error: message,
                        details,
                        request_id: None,
                    }),
                )
                    .into_response(),
            ),
        }
    }

    pub fn into_response(self) -> Response {
        *self.response
    }
}

impl<E> From<E> for RequestError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self::internal(value.into())
    }
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        self.into_response()
    }
}

/// Convenience for handlers that need to fail with a plain internal message.
pub fn internal(message: impl Into<String>) -> anyhow::Error {
    anyhow!(message.into())
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn body_of(response: Response) -> (StatusCode, String) {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body");
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn validation_returns_details() {
        let error = RequestError::validation(vec![FieldError::new("port", "端口不能为空")]);
        let (status, body) = body_of(error.into_response()).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(body.contains("\"field\":\"port\""), "{body}");
        assert!(body.contains("端口不能为空"), "{body}");
    }

    #[tokio::test]
    async fn validation_summarises_multiple_fields() {
        let error = RequestError::validation(vec![
            FieldError::new("name", "名称不能为空"),
            FieldError::new("port", "端口不合法"),
        ]);
        let (_, body) = body_of(error.into_response()).await;
        assert!(body.contains("共 2 项"), "{body}");
    }

    #[tokio::test]
    async fn internal_errors_do_not_leak_the_cause() {
        let error: RequestError =
            anyhow::anyhow!("connection to /var/lib/secret.db failed").into();
        let (status, body) = body_of(error.into_response()).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains(INTERNAL_MESSAGE), "{body}");
        assert!(!body.contains("secret.db"), "leaked cause: {body}");
        assert!(body.contains("request_id"), "{body}");
    }

    #[tokio::test]
    async fn public_messages_are_returned_verbatim() {
        let (status, body) = body_of(RequestError::conflict("已存在").into_response()).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(body.contains("已存在"), "{body}");
    }
}
