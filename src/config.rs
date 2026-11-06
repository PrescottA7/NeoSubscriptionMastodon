// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use std::{
    fs,
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};

const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3000";
const DEFAULT_DATA_ROOT: &str = "data";
const DEFAULT_CONFIG_FILE: &str = "config.yml";

/// Command line interface for the NSM server.
#[derive(Parser, Debug)]
#[command(name = "nsm", version, about = "NeoSubscriptionMastodon subscription manager")]
struct CliArgs {
    /// Path to the YAML configuration file.
    #[arg(short, long, default_value = DEFAULT_CONFIG_FILE)]
    config: PathBuf,
}

/// Runtime configuration, persisted as YAML so operators can edit it directly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Address the HTTP server binds to.
    pub bind_addr: String,
    /// Directory holding the database, encryption key, and logs.
    pub data_root: PathBuf,
    /// Marks authentication cookies as `Secure`. Enable only when the admin UI
    /// is served over HTTPS, otherwise browsers discard the session cookie.
    #[serde(default)]
    pub cookie_secure: bool,
    /// Overrides the built-in `404.html` disguise page when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disguise_page: Option<PathBuf>,
    /// Host that only serves subscription links. Empty disables host routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_host: Option<String>,
    /// Host that only serves the admin UI and API. Empty disables host routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_host: Option<String>,
    /// Overrides the database subscription prefix when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_prefix: Option<String>,
    /// Path prefix for the admin UI and API. Empty serves them from the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_path_prefix: Option<String>,
    #[serde(skip)]
    pub database_path: PathBuf,
    #[serde(skip)]
    pub encryption_key_path: PathBuf,
    /// Set when the file was generated on this run, used for logging.
    #[serde(skip)]
    pub created: bool,
}

impl AppConfig {
    pub fn default_data_root() -> PathBuf {
        PathBuf::from(DEFAULT_DATA_ROOT)
    }

    pub fn default_from_data_root<P: AsRef<Path>>(data_root: P) -> Self {
        Self::from_parts(
            DEFAULT_BIND_ADDR.to_owned(),
            data_root.as_ref().to_path_buf(),
            false,
        )
    }

    /// Builds a fully-resolved configuration from the user-visible fields.
    pub fn from_parts(bind_addr: String, data_root: PathBuf, cookie_secure: bool) -> Self {
        Self {
            bind_addr,
            database_path: data_root.join("nsm.sqlite3"),
            encryption_key_path: data_root.join("encryption.key"),
            data_root,
            cookie_secure,
            disguise_page: None,
            subscription_host: None,
            admin_host: None,
            subscription_prefix: None,
            admin_path_prefix: None,
            created: false,
        }
    }

    /// Configuration used by tests: isolated directory, ephemeral port.
    pub fn test_config<P: AsRef<Path>>(data_root: P) -> Self {
        Self::from_parts(
            "127.0.0.1:0".to_owned(),
            data_root.as_ref().to_path_buf(),
            false,
        )
    }

    pub fn from_cli() -> anyhow::Result<Self> {
        Self::from_cli_args(std::env::args_os())
    }

    fn from_cli_args<I, T>(args: I) -> anyhow::Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let cli = CliArgs::parse_from(args);
        Self::load_or_create(&cli.config)
    }

    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            return Self::read_from_file(path);
        }

        let config = Self {
            created: true,
            ..Self::default()
        };
        config.write_template(path)?;
        Ok(config)
    }

    /// Writes a commented template rather than a bare serialisation, so a fresh
    /// install is self-documenting.
    pub fn write_template(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).with_context(|| {
                format!("create config parent directory at {}", parent.display())
            })?;
        }

        let template = format!(
            r#"# NSM (NeoSubscriptionMastodon) configuration
#
# Generated on first start. Edit and restart the process to apply changes.

# Address the HTTP server binds to.
bind_addr: {bind_addr}

# Directory holding the SQLite database, the encryption key and logs.
data_root: {data_root}

# Mark the admin session cookie as `Secure`.
# Enable ONLY when the admin UI is served over HTTPS; over plain HTTP the
# browser silently drops the cookie and login appears to fail.
cookie_secure: {cookie_secure}

# Optional: path to a custom 404 page used to disguise the service.
# When omitted the built-in page is used; it can also be replaced from the
# admin settings screen without touching this file.
# disguise_page: ./404.html

# ---------------------------------------------------------------------------
# Domain and path separation (optional)
# ---------------------------------------------------------------------------
#
# Set these when the subscription links and the admin UI live on different
# hostnames, or when the admin UI should not sit at the root path. The proxy in
# front of NSM decides which hostname reaches this process; NSM only reads the
# incoming Host header.
#
# IMPORTANT: behind a reverse proxy, either pass the original name through
# unchanged or set X-Forwarded-Host. If the proxy rewrites Host without setting
# X-Forwarded-Host, the request is classified as "unknown" and answered with the
# disguise page.

# Host that only serves subscription links. Everything else on this host -
# including the admin UI and its API - answers 404.
# subscription_host: sub.example.com

# Host that only serves the admin UI and API. Subscription paths on this host
# answer 404.
# admin_host: panel.example.com

# Subscription path prefix. Takes precedence over the value stored in the
# database when set.
# subscription_prefix: /s

# Path prefix for the admin UI and API, for example /admin or /subsc-admin.
# Empty serves them from the root. Must not overlap the subscription prefix and
# must not be /api.
# admin_path_prefix: /admin
"#,
            bind_addr = self.bind_addr,
            data_root = self.data_root.display(),
            cookie_secure = self.cookie_secure,
        );

        let mut file = fs::File::create(path)
            .with_context(|| format!("create config file at {}", path.display()))?;
        file.write_all(template.as_bytes())
            .with_context(|| format!("write config file at {}", path.display()))?;
        file.sync_all().ok();

        Ok(())
    }

    fn read_from_file(path: &Path) -> anyhow::Result<Self> {
        let file =
            fs::File::open(path).with_context(|| format!("open config file at {}", path.display()))?;
        let reader = BufReader::new(file);
        let parsed: Self = serde_yaml::from_reader(reader)
            .with_context(|| format!("parse yaml config from {}", path.display()))?;

        // Derived paths are never trusted from disk so `data_root` stays authoritative.
        Ok(
            Self::from_parts(parsed.bind_addr, parsed.data_root, parsed.cookie_secure)
                .with_scope(parsed.disguise_page, parsed.subscription_host, parsed.admin_host)
                .with_prefixes(parsed.subscription_prefix, parsed.admin_path_prefix),
        )
    }

    fn with_scope(
        mut self,
        disguise_page: Option<PathBuf>,
        subscription_host: Option<String>,
        admin_host: Option<String>,
    ) -> Self {
        self.disguise_page = disguise_page;
        self.subscription_host = normalize_host(subscription_host);
        self.admin_host = normalize_host(admin_host);
        self
    }

    fn with_prefixes(
        mut self,
        subscription_prefix: Option<String>,
        admin_path_prefix: Option<String>,
    ) -> Self {
        self.subscription_prefix = normalize_prefix(subscription_prefix);
        self.admin_path_prefix = normalize_prefix(admin_path_prefix);
        self
    }

    /// Path prefix that the admin UI and API are mounted under.
    ///
    /// An empty string means "the root", which keeps the historical layout.
    pub fn admin_prefix(&self) -> &str {
        self.admin_path_prefix.as_deref().unwrap_or("")
    }

    /// Subscription prefix from the file, if the operator pinned one.
    pub fn pinned_subscription_prefix(&self) -> Option<&str> {
        self.subscription_prefix.as_deref()
    }

    /// Rejects configurations that cannot be served unambiguously.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let (Some(sub), Some(admin)) = (&self.subscription_host, &self.admin_host)
            && sub == admin
        {
            anyhow::bail!(
                "subscription_host and admin_host are both {sub}; \
                 leave one empty or use two different hostnames"
            );
        }

        let admin = self.admin_prefix();
        if !admin.is_empty() {
            if !admin.starts_with('/') {
                anyhow::bail!("admin_path_prefix must start with a slash, got {admin}");
            }
            if admin == "/api" || admin.starts_with("/api/") {
                anyhow::bail!("admin_path_prefix must not shadow the API path, got {admin}");
            }
            if let Some(sub) = self.pinned_subscription_prefix()
                && (admin == sub
                    || admin.starts_with(&format!("{sub}/"))
                    || sub.starts_with(&format!("{admin}/")))
            {
                anyhow::bail!(
                    "admin_path_prefix ({admin}) must not overlap \
                     subscription_prefix ({sub})"
                );
            }
        }

        Ok(())
    }

    /// Creates every directory the process needs before serving traffic.
    pub fn ensure_directories(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.data_root)
            .with_context(|| format!("create data directory at {}", self.data_root.display()))
    }

    /// Warning shown when `cookie_secure` is combined with a plain-HTTP bind.
    pub fn insecure_transport_warning(&self) -> Option<&'static str> {
        (self.cookie_secure && !self.bind_addr.contains(":443")).then_some(
            "cookie_secure is enabled but the server does not appear to be behind HTTPS; \
             the browser will drop the session cookie and login will fail",
        )
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::default_from_data_root(Self::default_data_root())
    }
}

/// Lower-cases a hostname and drops the port, so `Sub.Example.com:443` and
/// `sub.example.com` compare equal.
fn normalize_host(raw: Option<String>) -> Option<String> {
    let value = raw?.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    // IPv6 literals are bracketed; only strip a port after the closing bracket.
    let host = match value.rfind(']') {
        Some(index) if value.starts_with('[') => &value[..=index],
        _ => value.split(':').next().unwrap_or(&value),
    };
    let host = host.trim_matches(|c| c == '.').to_owned();
    (!host.is_empty()).then_some(host)
}

/// Normalises a path prefix: leading slash, no trailing slash, `None` when blank.
fn normalize_prefix(raw: Option<String>) -> Option<String> {
    let value = raw?.trim().to_owned();
    if value.is_empty() || value == "/" {
        return None;
    }
    let with_slash = if value.starts_with('/') {
        value
    } else {
        format!("/{value}")
    };
    Some(with_slash.trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_create_writes_a_commented_template() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("custom.yml");

        let config = AppConfig::load_or_create(&path).expect("load config");
        let written = fs::read_to_string(&path).expect("read");

        assert!(config.created);
        assert_eq!(config.bind_addr, DEFAULT_BIND_ADDR);
        assert!(written.starts_with("# NSM (NeoSubscriptionMastodon)"));
        assert!(written.contains("bind_addr:"));
        assert!(written.contains("data_root:"));
        assert!(written.contains("cookie_secure:"));
        assert!(written.contains("disguise_page"));
        assert!(written.contains("subscription_host"));
        assert!(written.contains("admin_path_prefix"));
    }

    #[test]
    fn generated_template_round_trips() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("config.yml");

        let created = AppConfig::load_or_create(&path).expect("create");
        let reloaded = AppConfig::load_or_create(&path).expect("reload");

        assert!(!reloaded.created);
        assert_eq!(created.bind_addr, reloaded.bind_addr);
        assert_eq!(created.data_root, reloaded.data_root);
    }

    #[test]
    fn read_from_file_recomputes_derived_paths() {
        let temp = tempdir().expect("temp dir");
        let data_root = temp.path().join("my-data");
        let path = temp.path().join("nested").join("config.yml");

        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(
            &path,
            format!(
                "bind_addr: 127.0.0.1:3999\ndata_root: {}\ncookie_secure: true\n",
                data_root.display()
            ),
        )
        .expect("write yaml");

        let config = AppConfig::load_or_create(&path).expect("load config");

        assert_eq!(config.bind_addr, "127.0.0.1:3999");
        assert_eq!(config.data_root, data_root);
        assert!(config.cookie_secure);
        assert_eq!(config.database_path, config.data_root.join("nsm.sqlite3"));
        assert_eq!(
            config.encryption_key_path,
            config.data_root.join("encryption.key")
        );
    }

    #[test]
    fn from_cli_args_creates_default_config_file() {
        let temp = tempdir().expect("temp dir");
        let previous_cwd = std::env::current_dir().expect("cwd");

        std::env::set_current_dir(temp.path()).expect("set cwd");
        let result = AppConfig::from_cli_args(["nsm"]);
        std::env::set_current_dir(previous_cwd).expect("restore cwd");

        assert!(result.is_ok());
        assert!(temp.path().join("config.yml").exists());
    }

    #[test]
    fn insecure_transport_warning_only_fires_for_plain_binds() {
        let mut config = AppConfig::default();
        assert!(config.insecure_transport_warning().is_none());

        config.cookie_secure = true;
        config.bind_addr = "0.0.0.0:3000".to_owned();
        assert!(config.insecure_transport_warning().is_some());

        config.bind_addr = "0.0.0.0:443".to_owned();
        assert!(config.insecure_transport_warning().is_none());
    }

    #[test]
    fn hosts_are_lowercased_and_deported() {
        assert_eq!(
            normalize_host(Some("  Panel.Example.COM:8443 ".to_owned())).as_deref(),
            Some("panel.example.com")
        );
        assert_eq!(
            normalize_host(Some("[::1]:443".to_owned())).as_deref(),
            Some("[::1]")
        );
        assert_eq!(normalize_host(Some("   ".to_owned())), None);
        assert_eq!(normalize_host(None), None);
    }

    #[test]
    fn prefixes_are_normalised() {
        assert_eq!(
            normalize_prefix(Some("admin".to_owned())).as_deref(),
            Some("/admin")
        );
        assert_eq!(
            normalize_prefix(Some("/admin/".to_owned())).as_deref(),
            Some("/admin")
        );
        assert_eq!(normalize_prefix(Some("/".to_owned())), None);
        assert_eq!(normalize_prefix(Some(String::new())), None);
    }

    fn config_with(admin: &str, subscription: Option<&str>) -> AppConfig {
        AppConfig {
            admin_path_prefix: normalize_prefix(Some(admin.to_owned())),
            subscription_prefix: subscription.map(|value| value.to_owned()),
            ..AppConfig::default()
        }
    }

    #[test]
    fn default_config_uses_the_root() {
        let config = AppConfig::default();
        assert_eq!(config.admin_prefix(), "");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn identical_hosts_are_rejected() {
        let config = AppConfig {
            subscription_host: Some("example.com".to_owned()),
            admin_host: Some("example.com".to_owned()),
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn admin_prefix_may_not_overlap_the_subscription_prefix() {
        assert!(config_with("/s/admin", Some("/s")).validate().is_err());
        assert!(config_with("/admin", Some("/admin")).validate().is_err());
        assert!(config_with("/admin", Some("/s")).validate().is_ok());
    }

    #[test]
    fn admin_prefix_may_not_be_the_api() {
        assert!(config_with("/api", None).validate().is_err());
        assert!(config_with("/api/v2", None).validate().is_err());
    }

    #[test]
    fn scope_settings_survive_a_round_trip() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("config.yml");

        fs::write(
            &path,
            format!(
                "bind_addr: 127.0.0.1:3999\ndata_root: {}\n\
                 subscription_host: Sub.Example.com:443\n\
                 admin_host: panel.example.com\n\
                 subscription_prefix: /link\n\
                 admin_path_prefix: /admin/\n",
                temp.path().display()
            ),
        )
        .expect("write yaml");

        let config = AppConfig::load_or_create(&path).expect("load config");
        assert_eq!(config.subscription_host.as_deref(), Some("sub.example.com"));
        assert_eq!(config.admin_host.as_deref(), Some("panel.example.com"));
        assert_eq!(config.pinned_subscription_prefix(), Some("/link"));
        assert_eq!(config.admin_prefix(), "/admin");
        assert!(config.validate().is_ok());
    }
}
