use std::sync::{Arc, RwLock};

use anyhow::Result;
use sea_orm::DatabaseConnection;

use crate::auth::SessionController;
use crate::config::AppConfig;
use crate::crypto::EncryptionKey;
use crate::db::{self, admins::AdminSummary, settings::AppSettings};
use crate::disguise::Disguise;

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
        let disguise = Disguise::new(load_disguise_page(&config, custom_page), &settings);

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
        let disguise = Disguise::new(load_disguise_page(&self.config, custom_page), &settings);

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
}
