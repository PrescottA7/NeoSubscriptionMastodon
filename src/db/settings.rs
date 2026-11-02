use sea_orm::{
    ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

use crate::utils::normalize_path;

use super::entities::settings;

pub type Setting = settings::Model;

/// Key of the configurable subscription path prefix.
pub const KEY_SUBSCRIPTION_PREFIX: &str = "subscription_prefix";
/// Key of the configurable disguise page body (HTML).
pub const KEY_DISGUISE_PAGE: &str = "disguise_page";
/// Key of the default subscription format handed to recognised clients.
pub const KEY_DEFAULT_TARGET: &str = "default_target";
/// Key of the response delay applied to failed lookups, in milliseconds.
pub const KEY_FAILURE_DELAY_MS: &str = "failure_delay_ms";
/// Key of the site name shown in the admin UI.
pub const KEY_SITE_NAME: &str = "site_name";
/// Key of the pass-through switch for the disguise page.
pub const KEY_DISGUISE_PASSTHROUGH: &str = "disguise_passthrough";
/// Key of the sample page the disguise pass-through mirrors.
pub const KEY_DISGUISE_TEMPLATE_URL: &str = "disguise_template_url";
/// Key of the dial IP that overrides DNS resolution for the sample page.
pub const KEY_DISGUISE_TEMPLATE_IP: &str = "disguise_template_ip";
/// Key of the dial port that overrides the URL port for the sample page.
pub const KEY_DISGUISE_TEMPLATE_PORT: &str = "disguise_template_port";

pub const DEFAULT_SUBSCRIPTION_PREFIX: &str = "/s";
pub const DEFAULT_SITE_NAME: &str = "NSM";

/// Fully resolved settings with defaults applied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub subscription_prefix: String,
    pub default_target: String,
    pub failure_delay_ms: u64,
    pub site_name: String,
    pub has_custom_disguise_page: bool,
    pub disguise_passthrough: bool,
    pub disguise_template_url: String,
    pub disguise_template_ip: String,
    pub disguise_template_port: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            subscription_prefix: DEFAULT_SUBSCRIPTION_PREFIX.to_owned(),
            default_target: "uri".to_owned(),
            failure_delay_ms: 0,
            site_name: DEFAULT_SITE_NAME.to_owned(),
            has_custom_disguise_page: false,
            disguise_passthrough: false,
            disguise_template_url: String::new(),
            disguise_template_ip: String::new(),
            disguise_template_port: 0,
        }
    }
}

impl AppSettings {
    /// True when the disguise page should be mirrored from the template URL.
    pub fn passthrough_configured(&self) -> bool {
        self.disguise_passthrough && !self.disguise_template_url.is_empty()
    }
}

/// Partial update accepted from the admin API.
#[derive(Debug, Clone, Deserialize)]
pub struct SettingsUpdate {
    pub subscription_prefix: Option<String>,
    pub default_target: Option<String>,
    pub failure_delay_ms: Option<u64>,
    pub site_name: Option<String>,
    pub disguise_passthrough: Option<bool>,
    pub disguise_template_url: Option<String>,
    pub disguise_template_ip: Option<String>,
    pub disguise_template_port: Option<u16>,
}

pub async fn list(conn: &DatabaseConnection) -> Result<Vec<Setting>, DbErr> {
    settings::Entity::find()
        .order_by_asc(settings::Column::Key)
        .all(conn)
        .await
}

pub async fn get(conn: &DatabaseConnection, key: &str) -> Result<Option<String>, DbErr> {
    Ok(settings::Entity::find_by_id(key.to_owned())
        .one(conn)
        .await?
        .map(|row| row.value))
}

pub async fn set(conn: &DatabaseConnection, key: &str, value: &str) -> Result<(), DbErr> {
    match settings::Entity::find_by_id(key.to_owned()).one(conn).await? {
        Some(existing) => {
            let mut active: settings::ActiveModel = existing.into();
            active.value = Set(value.to_owned());
            active.update(conn).await?;
        }
        None => {
            settings::ActiveModel {
                key: Set(key.to_owned()),
                value: Set(value.to_owned()),
            }
            .insert(conn)
            .await?;
        }
    }
    Ok(())
}

pub async fn remove(conn: &DatabaseConnection, key: &str) -> Result<(), DbErr> {
    settings::Entity::delete_by_id(key.to_owned()).exec(conn).await?;
    Ok(())
}

/// Reads every setting, applying defaults for missing keys.
pub async fn load(conn: &DatabaseConnection) -> Result<AppSettings, DbErr> {
    let rows = list(conn).await?;
    let mut resolved = AppSettings::default();

    for row in rows {
        match row.key.as_str() {
            KEY_SUBSCRIPTION_PREFIX => {
                let normalized = normalize_path(&row.value);
                if !normalized.is_empty() {
                    resolved.subscription_prefix = normalized;
                }
            }
            KEY_DEFAULT_TARGET if !row.value.is_empty() => {
                resolved.default_target = row.value;
            }
            KEY_FAILURE_DELAY_MS => {
                resolved.failure_delay_ms = row.value.parse().unwrap_or_default();
            }
            KEY_SITE_NAME if !row.value.is_empty() => {
                resolved.site_name = row.value;
            }
            KEY_DISGUISE_PAGE => {
                resolved.has_custom_disguise_page = !row.value.is_empty();
            }
            KEY_DISGUISE_PASSTHROUGH => {
                resolved.disguise_passthrough = row.value == "true" || row.value == "1";
            }
            KEY_DISGUISE_TEMPLATE_URL => {
                resolved.disguise_template_url = row.value.trim().to_owned();
            }
            KEY_DISGUISE_TEMPLATE_IP => {
                resolved.disguise_template_ip = row.value.trim().to_owned();
            }
            KEY_DISGUISE_TEMPLATE_PORT => {
                resolved.disguise_template_port = row.value.parse().unwrap_or_default();
            }
            _ => {}
        }
    }

    Ok(resolved)
}

/// Applies a partial update, returning the resolved settings afterwards.
pub async fn update(
    conn: &DatabaseConnection,
    input: &SettingsUpdate,
) -> Result<AppSettings, DbErr> {
    if let Some(prefix) = input.subscription_prefix.as_deref() {
        let normalized = normalize_path(prefix);
        if normalized.is_empty() {
            return Err(DbErr::Custom("subscription prefix cannot be empty".to_owned()));
        }
        if normalized == "/api" || normalized.starts_with("/api/") {
            return Err(DbErr::Custom(
                "subscription prefix cannot shadow the admin API".to_owned(),
            ));
        }
        set(conn, KEY_SUBSCRIPTION_PREFIX, &normalized).await?;
    }

    if let Some(target) = input.default_target.as_deref() {
        set(conn, KEY_DEFAULT_TARGET, target).await?;
    }

    if let Some(delay) = input.failure_delay_ms {
        set(conn, KEY_FAILURE_DELAY_MS, &delay.to_string()).await?;
    }

    if let Some(site_name) = input.site_name.as_deref() {
        let site_name = site_name.trim();
        if site_name.is_empty() {
            return Err(DbErr::Custom("site name cannot be empty".to_owned()));
        }
        set(conn, KEY_SITE_NAME, site_name).await?;
    }

    if let Some(enabled) = input.disguise_passthrough {
        set(conn, KEY_DISGUISE_PASSTHROUGH, if enabled { "true" } else { "false" }).await?;
    }

    if let Some(url) = input.disguise_template_url.as_deref() {
        let url = url.trim();
        if url.is_empty() {
            remove(conn, KEY_DISGUISE_TEMPLATE_URL).await?;
        } else {
            set(conn, KEY_DISGUISE_TEMPLATE_URL, url).await?;
        }
    }

    if let Some(ip) = input.disguise_template_ip.as_deref() {
        let ip = ip.trim();
        if ip.is_empty() {
            remove(conn, KEY_DISGUISE_TEMPLATE_IP).await?;
        } else {
            set(conn, KEY_DISGUISE_TEMPLATE_IP, ip).await?;
        }
    }

    if let Some(port) = input.disguise_template_port {
        if port == 0 {
            remove(conn, KEY_DISGUISE_TEMPLATE_PORT).await?;
        } else {
            set(conn, KEY_DISGUISE_TEMPLATE_PORT, &port.to_string()).await?;
        }
    }

    load(conn).await
}

pub async fn set_disguise_page(conn: &DatabaseConnection, html: &str) -> Result<(), DbErr> {
    if html.trim().is_empty() {
        return remove(conn, KEY_DISGUISE_PAGE).await;
    }
    set(conn, KEY_DISGUISE_PAGE, html).await
}

pub async fn get_disguise_page(conn: &DatabaseConnection) -> Result<Option<String>, DbErr> {
    Ok(get(conn, KEY_DISGUISE_PAGE)
        .await?
        .filter(|value| !value.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let settings = AppSettings::default();
        assert_eq!(settings.subscription_prefix, "/s");
        assert_eq!(settings.default_target, "uri");
        assert_eq!(settings.failure_delay_ms, 0);
        assert!(!settings.has_custom_disguise_page);
    }
}
