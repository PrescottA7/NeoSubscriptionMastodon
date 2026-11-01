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
                .with_disguise_page(parsed.disguise_page),
        )
    }

    fn with_disguise_page(mut self, disguise_page: Option<PathBuf>) -> Self {
        self.disguise_page = disguise_page;
        self
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
}
