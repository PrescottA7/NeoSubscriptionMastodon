// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    Json, Router,
    extract::State,
    routing::{get, put},
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
    /// True when `config.yml` pins the prefix, which makes the stored value
    /// read-only in the UI.
    subscription_prefix_pinned: bool,
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
        .route_layer(axum::middleware::from_extractor_with_state::<
            RequireAuth,
            AppState,
        >(state.clone()))
}

async fn get_settings(State(state): State<AppState>) -> RequestResult<Json<SettingsResponse>> {
    let settings = state.settings().await?;
    let disguise_page = state.disguise().page().to_owned();

    Ok(Json(SettingsResponse {
        settings,
        subscription_prefix_pinned: state.config.pinned_subscription_prefix().is_some(),
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
        subscription_prefix_pinned: state.config.pinned_subscription_prefix().is_some(),
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
            validator.push("subscription_prefix", "Subscription prefix cannot be empty");
        } else if !trimmed.starts_with('/') {
            validator.push("subscription_prefix", "Subscription prefix must start with /");
        } else if trimmed.len() > 64 {
            validator.push("subscription_prefix", "Subscription prefix must be at most 64 characters");
        } else if trimmed
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '?' | '#' | '\\'))
        {
            validator.push("subscription_prefix", "Subscription prefix cannot contain spaces, ? or #");
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
                "Unsupported default format \"{trimmed}\", available options: uri or {}",
                crate::render::supported_cores()
                    .iter()
                    .map(|core| core.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        );
    }

    if let Some(site_name) = payload.site_name.as_deref() {
        validator.text("site_name", "Site name", site_name, 1, 32);
    }

    if let Some(delay) = payload.failure_delay_ms {
        validator.require(
            delay <= 10_000,
            "failure_delay_ms",
            "Failure delay must be at most 10000 milliseconds",
        );
    }

    if let Some(url) = payload.disguise_template_url.as_deref() {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            match url::Url::parse(trimmed) {
                Ok(parsed) if parsed.scheme() == "https" && parsed.host_str().is_some() => {}
                Ok(_) => validator.push(
                    "disguise_template_url",
                    "Template URL must start with https:// and include a domain",
                ),
                Err(_) => validator.push("disguise_template_url", "Template URL is not a valid URL"),
            }
        }
    }

    if let Some(ip) = payload.disguise_template_ip.as_deref() {
        let trimmed = ip.trim();
        if !trimmed.is_empty() {
            // The override may be an IP or a hostname such as `host.docker.internal`.
            validator.host("disguise_template_ip", trimmed);
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
            "Please fill in the template URL before enabling 404 passthrough",
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
        subscription_prefix_pinned: state.config.pinned_subscription_prefix().is_some(),
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
    fn a_bad_override_address_is_rejected() {
        let error = validate_settings(&passthrough(
            "https://sample.example.com/",
            "https://host.docker.internal",
            true,
        ))
        .expect_err("should fail");
        assert_eq!(error.fields()[0].field, "disguise_template_ip");
    }

    #[test]
    fn an_override_hostname_is_accepted() {
        assert!(
            validate_settings(&passthrough(
                "https://sample.example.com/",
                "host.docker.internal",
                true
            ))
            .is_ok()
        );
    }
}
