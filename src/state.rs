// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use std::sync::{Arc, RwLock};

use anyhow::Result;
use sea_orm::DatabaseConnection;

use crate::auth::SessionController;
use crate::config::AppConfig;
use crate::crypto::EncryptionKey;
use crate::db::{self, admins::AdminSummary, settings::AppSettings};
use crate::disguise::Disguise;

/// Which part of the service a request is meant for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestScope {
    /// Host routing is off; serve the historical mixed layout.
    Mixed,
    /// The request arrived on the admin host.
    Admin,
    /// The request arrived on the subscription host.
    Subscription,
    /// Host routing is on but the name matched neither host.
    Unknown,
}

/// Host the client asked for, preferring the proxy's `X-Forwarded-Host`.
///
/// Reverse proxies commonly rewrite `Host`, so the original name travels in
/// `X-Forwarded-Host`; the first value wins when several are supplied.
fn request_host(headers: &axum::http::HeaderMap) -> Option<String> {
    let raw = headers
        .get("x-forwarded-host")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get(axum::http::header::HOST)
                .and_then(|value| value.to_str().ok())
        })?;

    let lowered = raw.trim().to_ascii_lowercase();
    let host = match lowered.rfind(']') {
        Some(index) if lowered.starts_with('[') => lowered[..=index].to_owned(),
        _ => lowered.split(':').next().unwrap_or(&lowered).to_owned(),
    };
    (!host.is_empty()).then_some(host)
}

/// Shared application state: configuration, database, credentials key, and the
/// cached settings/disguise page.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub key: Arc<EncryptionKey>,
    pub sessions: SessionController,
    disguise: Arc<RwLock<Disguise>>,
    settings: Arc<RwLock<AppSettings>>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self> {
        let db = db::connect(&config.database_path).await?;
        let key = EncryptionKey::load_or_create(&config.encryption_key_path)?;

        let settings = db::settings::load(&db).await?;
        let custom_page = db::settings::get_disguise_page(&db).await?;
        let disguise = Disguise::from_settings(load_disguise_page(&config, custom_page), &settings);

        Ok(Self {
            disguise: Arc::new(RwLock::new(disguise)),
            settings: Arc::new(RwLock::new(settings)),
            sessions: SessionController::new(db.clone()),
            config,
            key: Arc::new(key),
            db,
        })
    }

    pub async fn db_admin(&self) -> Result<Option<AdminSummary>, sea_orm::DbErr> {
        db::admins::get_admin(&self.db).await
    }

    /// Reads settings from the database, bypassing the cache.
    pub async fn settings(&self) -> Result<AppSettings, sea_orm::DbErr> {
        db::settings::load(&self.db).await
    }

    /// Cached settings, used on the request hot path.
    pub fn settings_cached(&self) -> AppSettings {
        self.settings
            .read()
            .expect("settings lock poisoned")
            .clone()
    }

    /// Path prefix the admin UI and API are served under; empty means the root.
    pub fn admin_prefix(&self) -> &str {
        self.config.admin_prefix()
    }

    /// Subscription prefix actually in use.
    ///
    /// A prefix pinned in `config.yml` wins so an operator can fix the link
    /// layout at deploy time; otherwise the database value applies.
    pub fn subscription_prefix(&self) -> String {
        match self.config.pinned_subscription_prefix() {
            Some(pinned) => pinned.to_owned(),
            None => self.settings_cached().subscription_prefix,
        }
    }

    /// Which side of the service a request belongs to.
    ///
    /// When either host is configured, a request that matches neither is treated
    /// as a probe: it gets the disguise page rather than the admin area. Setting
    /// both hosts to the same name therefore serves both sides from that one
    /// hostname while still rejecting every other hostname.
    pub fn scope_for(&self, headers: &axum::http::HeaderMap) -> RequestScope {
        let host = request_host(headers);
        let subscription = self.config.subscription_host.as_deref();
        let admin = self.config.admin_host.as_deref();

        if subscription.is_none() && admin.is_none() {
            return RequestScope::Mixed;
        }

        match host.as_deref() {
            Some(host) if Some(host) == subscription && Some(host) == admin => RequestScope::Mixed,
            Some(host) if Some(host) == subscription => RequestScope::Subscription,
            Some(host) if Some(host) == admin => RequestScope::Admin,
            _ => RequestScope::Unknown,
        }
    }

    /// Current disguise page. Cloning is cheap because the body is behind an `Arc`.
    pub fn disguise(&self) -> Disguise {
        self.disguise
            .read()
            .expect("disguise lock poisoned")
            .clone()
    }

    /// Rebuilds the cached settings and disguise page after a change.
    pub async fn reload_disguise(&self) -> Result<(), sea_orm::DbErr> {
        let settings = self.settings().await?;
        let custom_page = db::settings::get_disguise_page(&self.db).await?;
        let disguise = Disguise::from_settings(load_disguise_page(&self.config, custom_page), &settings);

        *self.disguise.write().expect("disguise lock poisoned") = disguise;
        *self.settings.write().expect("settings lock poisoned") = settings;
        Ok(())
    }
}

/// Precedence for the disguise body: database setting, then file, then built-in.
fn load_disguise_page(config: &AppConfig, database_page: Option<String>) -> Option<String> {
    if database_page.is_some() {
        return database_page;
    }

    config
        .disguise_page
        .as_ref()
        .and_then(|path| std::fs::read_to_string(path).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn state_is_created_with_fresh_database_and_key() {
        let temp = tempdir().expect("temp dir");
        let config = AppConfig::test_config(temp.path());
        let state = AppState::new(config).await.expect("state");

        assert!(temp.path().join("nsm.sqlite3").exists());
        assert!(temp.path().join("encryption.key").exists());
        assert!(state.db_admin().await.expect("admin lookup").is_none());
        assert_eq!(state.settings_cached().subscription_prefix, "/s");
    }

    #[tokio::test]
    async fn settings_change_is_cached_after_reload() {
        let temp = tempdir().expect("temp dir");
        let config = AppConfig::test_config(temp.path());
        let state = AppState::new(config).await.expect("state");

        db::settings::set_disguise_page(&state.db, "<html>decoy</html>")
            .await
            .expect("store page");
        db::settings::set(&state.db, db::settings::KEY_SUBSCRIPTION_PREFIX, "/x")
            .await
            .expect("store prefix");
        state.reload_disguise().await.expect("reload");

        assert_eq!(state.disguise().page(), "<html>decoy</html>");
        assert_eq!(state.settings_cached().subscription_prefix, "/x");
    }

    #[test]
    fn database_page_wins_over_file() {
        let temp = tempdir().expect("temp dir");
        let file = temp.path().join("page.html");
        std::fs::write(&file, "<html>file</html>").expect("write");

        let mut config = AppConfig::test_config(temp.path());
        config.disguise_page = Some(file);

        let page = load_disguise_page(&config, Some("<html>db</html>".to_owned()));
        assert_eq!(page.as_deref(), Some("<html>db</html>"));
    }

    #[test]
    fn missing_file_falls_back_to_none() {
        let temp = tempdir().expect("temp dir");
        let mut config = AppConfig::test_config(temp.path());
        config.disguise_page = Some(temp.path().join("missing.html"));

        assert!(load_disguise_page(&config, None).is_none());
    }

    fn headers(pairs: &[(&str, &str)]) -> axum::http::HeaderMap {
        let mut map = axum::http::HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).expect("name"),
                axum::http::HeaderValue::from_str(value).expect("value"),
            );
        }
        map
    }

    #[test]
    fn host_is_read_from_forwarded_header_first() {
        let map = headers(&[
            ("x-forwarded-host", "sub.example.com, proxy.internal"),
            ("host", "panel.example.com"),
        ]);
        assert_eq!(request_host(&map).as_deref(), Some("sub.example.com"));
    }

    #[test]
    fn host_falls_back_to_the_plain_header() {
        let map = headers(&[("host", "Panel.Example.COM:8443")]);
        assert_eq!(request_host(&map).as_deref(), Some("panel.example.com"));
    }

    #[tokio::test]
    async fn scopes_follow_the_configured_hosts() {
        let temp = tempdir().expect("temp dir");
        let mut config = AppConfig::test_config(temp.path());
        config.subscription_host = Some("sub.example.com".to_owned());
        config.admin_host = Some("panel.example.com".to_owned());
        let state = AppState::new(config).await.expect("state");

        let sub = headers(&[("host", "sub.example.com")]);
        let admin = headers(&[("host", "panel.example.com")]);
        let other = headers(&[("host", "elsewhere.example.net")]);

        assert_eq!(state.scope_for(&sub), RequestScope::Subscription);
        assert_eq!(state.scope_for(&admin), RequestScope::Admin);
        assert_eq!(state.scope_for(&other), RequestScope::Unknown);
        assert_eq!(
            state.scope_for(&axum::http::HeaderMap::new()),
            RequestScope::Unknown
        );
    }

    #[tokio::test]
    async fn a_shared_host_serves_both_sides_but_still_hides_others() {
        let temp = tempdir().expect("temp dir");
        let mut config = AppConfig::test_config(temp.path());
        config.subscription_host = Some("example.com".to_owned());
        config.admin_host = Some("example.com".to_owned());
        let state = AppState::new(config).await.expect("state");

        let shared = headers(&[("host", "example.com")]);
        let other = headers(&[("host", "elsewhere.example.net")]);

        assert_eq!(state.scope_for(&shared), RequestScope::Mixed);
        assert_eq!(state.scope_for(&other), RequestScope::Unknown);
    }

    #[tokio::test]
    async fn scope_routing_is_off_without_configured_hosts() {
        let temp = tempdir().expect("temp dir");
        let state = AppState::new(AppConfig::test_config(temp.path()))
            .await
            .expect("state");

        let map = headers(&[("host", "sub.example.com")]);
        assert_eq!(state.scope_for(&map), RequestScope::Mixed);
    }

    #[tokio::test]
    async fn a_pinned_subscription_prefix_wins_over_the_database() {
        let temp = tempdir().expect("temp dir");
        let mut config = AppConfig::test_config(temp.path());
        config.subscription_prefix = Some("/link".to_owned());
        let state = AppState::new(config).await.expect("state");

        db::settings::set(&state.db, db::settings::KEY_SUBSCRIPTION_PREFIX, "/db")
            .await
            .expect("store prefix");
        state.reload_disguise().await.expect("reload");

        assert_eq!(state.subscription_prefix(), "/link");
    }

    #[tokio::test]
    async fn without_a_pinned_prefix_the_database_value_applies() {
        let temp = tempdir().expect("temp dir");
        let state = AppState::new(AppConfig::test_config(temp.path()))
            .await
            .expect("state");

        db::settings::set(&state.db, db::settings::KEY_SUBSCRIPTION_PREFIX, "/db")
            .await
            .expect("store prefix");
        state.reload_disguise().await.expect("reload");

        assert_eq!(state.subscription_prefix(), "/db");
    }
}
