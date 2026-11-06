// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};
use serde::Serialize;

use crate::auth::RequireAuth;
use crate::db::settings::{self, AppSettings, SettingsUpdate};
use crate::error::{RequestError, RequestResult};
use crate::routes::extract::ApiJson;
use crate::state::AppState;
use crate::validate::Validator;

#[derive(Serialize)]
struct SettingsResponse {
    #[serde(flatten)]
    settings: AppSettings,
    disguise_page: String,
    available_cores: Vec<&'static str>,
}

#[derive(serde::Deserialize)]
struct DisguisePageUpdate {
    html: String,
}

pub fn routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/settings", get(get_settings).put(update_settings))
        .route("/api/settings/disguise-page", put(update_disguise_page))
        .route_layer(axum::middleware::from_extractor_with_state::<RequireAuth, AppState>(
            state.clone(),
        ))
}

async fn get_settings(State(state): State<AppState>) -> RequestResult<Json<SettingsResponse>> {
    let settings = state.settings().await?;
    let disguise_page = state.disguise().page().to_owned();

    Ok(Json(SettingsResponse {
        settings,
        disguise_page,
        available_cores: crate::render::supported_cores()
            .iter()
            .map(|core| core.as_str())
            .collect(),
    }))
}

async fn update_settings(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<SettingsUpdate>,
) -> RequestResult<Json<SettingsResponse>> {
    validate_settings(&payload)?;

    let updated = settings::update(&state.db, &payload)
        .await
        .map_err(|error| crate::error::RequestError::bad_request(error.to_string()))?;
    state.reload_disguise().await?;

    // `update` returns the freshly resolved settings, so the response is accurate
    // without a second read.
    Ok(Json(SettingsResponse {
        settings: updated,
        disguise_page: state.disguise().page().to_owned(),
        available_cores: crate::render::supported_cores()
            .iter()
            .map(|core| core.as_str())
            .collect(),
    }))
}

/// Rule set mirrored by `frontend/src/validation.ts::validateSettingsForm`.
fn validate_settings(payload: &SettingsUpdate) -> Result<(), RequestError> {
    let mut validator = Validator::new();

    if let Some(prefix) = payload.subscription_prefix.as_deref() {
        let trimmed = prefix.trim();
        if trimmed.is_empty() {
            validator.push("subscription_prefix", "订阅路径前缀不能为空");
        } else if !trimmed.starts_with('/') {
            validator.push("subscription_prefix", "订阅路径前缀必须以 / 开头");
        } else if trimmed.len() > 64 {
            validator.push("subscription_prefix", "订阅路径前缀最多 64 个字符");
        } else if trimmed
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '\\'))
        {
            validator.push("subscription_prefix", "订阅路径前缀不能包含空格、? 或 #");
        }
    }

    if let Some(target) = payload.default_target.as_deref() {
        let trimmed = target.trim();
        let known = crate::render::supported_cores()
            .iter()
            .any(|core| core.as_str() == trimmed)
            || trimmed == "uri";
        validator.require(
            known,
            "default_target",
            format!(
                "不支持的默认格式 “{trimmed}”，可选 uri 或 {}",
                crate::render::supported_cores()
                    .iter()
                    .map(|core| core.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        );
    }

    if let Some(site_name) = payload.site_name.as_deref() {
        validator.text("site_name", "站点名称", site_name, 1, 32);
    }

    if let Some(delay) = payload.failure_delay_ms {
        validator.require(
            delay <= 10_000,
            "failure_delay_ms",
            "失败延迟最多 10000 毫秒",
        );
    }

    if let Some(url) = payload.disguise_template_url.as_deref() {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            match url::Url::parse(trimmed) {
                Ok(parsed) if parsed.scheme() == "https" && parsed.host_str().is_some() => {}
                Ok(_) => validator.push(
                    "disguise_template_url",
                    "样板页地址必须是 https:// 开头并包含域名",
                ),
                Err(_) => validator.push("disguise_template_url", "样板页地址不是合法的 URL"),
            }
        }
    }

    if let Some(ip) = payload.disguise_template_ip.as_deref() {
        let trimmed = ip.trim();
        if !trimmed.is_empty() && trimmed.parse::<std::net::IpAddr>().is_err() {
            validator.push("disguise_template_ip", "服务器 IP 必须是合法地址");
        }
    }

    if payload.disguise_passthrough == Some(true) {
        let url = payload
            .disguise_template_url
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        // The toggle is only meaningful together with a sample URL.
        validator.require(
            !url.is_empty(),
            "disguise_template_url",
            "开启 404 透传前请先填写样板页地址",
        );
    }

    validator.finish()
}

/// Replaces the disguise page. Submitting an empty body restores the built-in one.
async fn update_disguise_page(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<DisguisePageUpdate>,
) -> RequestResult<Json<SettingsResponse>> {
    settings::set_disguise_page(&state.db, &payload.html).await?;
    state.reload_disguise().await?;

    let settings = state.settings().await?;
    Ok(Json(SettingsResponse {
        settings,
        disguise_page: state.disguise().page().to_owned(),
        available_cores: crate::render::supported_cores()
            .iter()
            .map(|core| core.as_str())
            .collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update(
        prefix: Option<&str>,
        target: Option<&str>,
        site_name: Option<&str>,
    ) -> SettingsUpdate {
        SettingsUpdate {
            subscription_prefix: prefix.map(str::to_owned),
            default_target: target.map(str::to_owned),
            failure_delay_ms: None,
            site_name: site_name.map(str::to_owned),
            disguise_passthrough: None,
            disguise_template_url: None,
            disguise_template_ip: None,
            disguise_template_port: None,
        }
    }

    #[test]
    fn valid_settings_pass() {
        assert!(validate_settings(&update(Some("/sub"), Some("sing-box"), Some("NSM"))).is_ok());
    }

    #[test]
    fn prefix_needs_a_leading_slash() {
        let error = validate_settings(&update(Some("sub"), None, None)).expect_err("should fail");
        assert_eq!(error.fields()[0].field, "subscription_prefix");
    }

    #[test]
    fn unknown_target_is_rejected_and_lists_choices() {
        let error = validate_settings(&update(None, Some("clash"), None)).expect_err("should fail");
        let message = &error.fields()[0].message;
        assert!(message.contains("clash"));
        assert!(message.contains("sing-box"));
    }

    #[test]
    fn blank_site_name_is_rejected() {
        let error = validate_settings(&update(None, None, Some("  "))).expect_err("should fail");
        assert_eq!(error.fields()[0].field, "site_name");
    }

    fn passthrough(url: &str, ip: &str, enabled: bool) -> SettingsUpdate {
        SettingsUpdate {
            subscription_prefix: None,
            default_target: None,
            failure_delay_ms: None,
            site_name: None,
            disguise_passthrough: Some(enabled),
            disguise_template_url: Some(url.to_owned()),
            disguise_template_ip: Some(ip.to_owned()),
            disguise_template_port: None,
        }
    }

    #[test]
    fn passthrough_accepts_an_https_template() {
        assert!(validate_settings(&passthrough("https://sample.example.com/", "", true)).is_ok());
    }

    #[test]
    fn passthrough_rejects_a_plain_http_template() {
        let error = validate_settings(&passthrough("http://sample.example.com/", "", true))
            .expect_err("should fail");
        assert_eq!(error.fields()[0].field, "disguise_template_url");
    }

    #[test]
    fn passthrough_needs_a_url_when_enabled() {
        let error = validate_settings(&passthrough("  ", "", true)).expect_err("should fail");
        assert_eq!(error.fields()[0].field, "disguise_template_url");
    }

    #[test]
    fn a_bad_override_ip_is_rejected() {
        let error =
            validate_settings(&passthrough("https://sample.example.com/", "not-an-ip", true))
                .expect_err("should fail");
        assert_eq!(error.fields()[0].field, "disguise_template_ip");
    }
}
