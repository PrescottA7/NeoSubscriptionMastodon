// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Request extractors that shape rejections like every other API error.

use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
    response::Response,
};
use serde::de::DeserializeOwned;

use crate::error::RequestError;

/// `Json` wrapper that turns deserialisation failures into a readable message.
///
/// Axum's built-in rejection is a bare `422` carrying a serde description
/// ("missing field `port`"), which reaches the UI as an unhelpful blob. The
/// status code is preserved but the body follows the usual `{ "error": ... }`
/// shape so the frontend surfaces something actionable.
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(request, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                let status = rejection.status();
                Err(RequestError::json(status, describe(&rejection)).into_response())
            }
        }
    }
}

/// Rewrites a serde rejection as advice the caller can act on.
///
/// Only the offending field name and position survive: the surrounding serde
/// text ("invalid type: null, expected u16") describes internal types the
/// client has no reason to know about.
fn describe(rejection: &JsonRejection) -> String {
    let raw = rejection.body_text();

    if raw.contains("Failed to parse the request body as JSON") {
        return "请求数据格式不正确：请求体不是合法的 JSON".to_owned();
    }
    if raw.contains("Content-Type: application/json") {
        return "请求数据格式不正确：请使用 Content-Type: application/json".to_owned();
    }

    match detail(&raw) {
        Some(detail) => format!("请求数据格式不正确：{detail}"),
        None => "请求数据格式不正确：请检查字段类型与必填项".to_owned(),
    }
}

/// Extracts the field name plus position from a serde message.
fn detail(raw: &str) -> Option<String> {
    for (prefix, advice) in [
        ("missing field `", "缺少必填字段"),
        ("unknown field `", "存在未知字段"),
    ] {
        if let Some(start) = raw.find(prefix) {
            let rest = &raw[start + prefix.len()..];
            let name = rest.split('`').next().unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let position = raw
                .find(" at line ")
                .map(|index| raw[index + 1..].trim_end_matches('.').to_owned())
                .unwrap_or_default();
            return Some(
                format!("{advice} `{name}` {position}")
                    .trim_end()
                    .to_owned(),
            );
        }
    }

    if let Some(field) = invalid_type_field(raw) {
        return Some(format!("字段 `{field}` 的类型不正确"));
    }

    None
}

/// Names the field in `port: invalid type: null, expected u16 at line 1 …`.
///
/// Serde prefixes the whole thing with `Failed to deserialize the JSON body
/// into the target type: `, so the search starts after the last colon that
/// introduces the field.
fn invalid_type_field(raw: &str) -> Option<&str> {
    let index = raw.find("invalid type:")?;
    let before = raw[..index].trim_end_matches([':', ' ']);
    let field = before.rsplit([':', ' ']).next().unwrap_or_default();
    if field.is_empty()
        || !field
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
    {
        return None;
    }
    Some(field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_field_is_named() {
        let message = detail("missing field `port` at line 1 column 40").expect("detail");
        assert!(message.contains("port"), "{message}");
        assert!(message.contains("缺少必填字段"), "{message}");
    }

    #[test]
    fn wrong_type_is_named_without_internal_type_names() {
        let message =
            detail("port: invalid type: null, expected u16 at line 1 column 40").expect("detail");
        assert!(message.contains("port"), "{message}");
        assert!(!message.contains("null"), "{message}");
        assert!(!message.contains("u16"), "{message}");
    }

    #[test]
    fn prefixed_wrong_type_still_names_the_field() {
        let raw = "Failed to deserialize the JSON body into the target type: port: invalid type: null, expected u16 at line 1 column 40";
        let message = detail(raw).expect("detail");
        assert!(message.contains("port"), "{message}");
        assert!(!message.contains("target type"), "{message}");
    }

    #[test]
    fn unknown_fields_are_reported() {
        let message = detail("unknown field `extra`, expected one of `a`, `b`").expect("detail");
        assert!(message.contains("extra"), "{message}");
    }

    #[test]
    fn unparseable_text_has_no_detail() {
        assert!(detail("something entirely different").is_none());
    }
}
