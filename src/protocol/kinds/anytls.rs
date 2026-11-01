use serde::{Deserialize, Serialize};

use super::super::shared::TlsConfig;

/// AnyTLS outbound settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/outbound/anytls/>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnytlsConfig {
    /// Interval between idle-session checks, e.g. `30s`.
    pub idle_session_check_interval: Option<String>,
    /// Idle sessions older than this are closed, e.g. `30s`.
    pub idle_session_timeout: Option<String>,
    /// Number of idle sessions kept open regardless of the timeout.
    pub min_idle_session: Option<u32>,
    /// Padding scheme metadata forwarded to the server.
    pub client_metadata: Option<String>,
    pub tls: TlsConfig,
}

impl Default for AnytlsConfig {
    fn default() -> Self {
        Self {
            idle_session_check_interval: None,
            idle_session_timeout: None,
            min_idle_session: None,
            client_metadata: None,
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
        }
    }
}

impl AnytlsConfig {
    pub fn idle_check_interval_value(&self) -> Option<&str> {
        self.idle_session_check_interval
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    pub fn idle_session_timeout_value(&self) -> Option<&str> {
        self.idle_session_timeout
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    /// Only sing-box implements these padding/session knobs.
    pub fn has_session_tuning(&self) -> bool {
        self.idle_check_interval_value().is_some()
            || self.idle_session_timeout_value().is_some()
            || self.min_idle_session.is_some()
            || self
                .client_metadata
                .as_deref()
                .is_some_and(|value| !value.is_empty())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.tls.enabled {
            return Err("anytls requires TLS to be enabled".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_is_mandatory() {
        let config = AnytlsConfig {
            tls: TlsConfig::default(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn session_tuning_is_detected() {
        let mut config = AnytlsConfig::default();
        assert!(!config.has_session_tuning());

        config.min_idle_session = Some(0);
        assert!(config.has_session_tuning());
    }

    #[test]
    fn blank_intervals_are_ignored() {
        let config = AnytlsConfig {
            idle_session_timeout: Some(String::new()),
            ..Default::default()
        };
        assert!(config.idle_session_timeout_value().is_none());
        assert!(!config.has_session_tuning());
    }
}
