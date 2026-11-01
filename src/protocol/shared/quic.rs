use serde::{Deserialize, Serialize};

/// QUIC transport tuning shared by Hysteria2 and TUIC.
///
/// Reference: <https://sing-box.sagernet.org/configuration/shared/quic/>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuicConfig {
    /// Initial stream-level receive window in bytes.
    pub initial_stream_receive_window: Option<u64>,
    /// Maximum stream-level receive window in bytes.
    pub max_stream_receive_window: Option<u64>,
    /// Initial connection-level receive window in bytes.
    pub initial_connection_receive_window: Option<u64>,
    /// Maximum connection-level receive window in bytes.
    pub max_connection_receive_window: Option<u64>,
    /// Keep-alive period, e.g. `10s`.
    pub keep_alive_period: Option<String>,
    /// Disable MTU discovery.
    pub disable_path_mtu_discovery: bool,
}

impl QuicConfig {
    /// True when any window override is set.
    pub fn has_window_overrides(&self) -> bool {
        self.initial_stream_receive_window.is_some()
            || self.max_stream_receive_window.is_some()
            || self.initial_connection_receive_window.is_some()
            || self.max_connection_receive_window.is_some()
    }

    /// True when nothing would be emitted.
    pub fn is_empty(&self) -> bool {
        !self.has_window_overrides()
            && self.keep_alive_period.is_none()
            && !self.disable_path_mtu_discovery
    }

    /// Non-empty keep-alive period.
    pub fn keep_alive_value(&self) -> Option<&str> {
        self.keep_alive_period
            .as_deref()
            .filter(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_empty() {
        assert!(QuicConfig::default().is_empty());
    }

    #[test]
    fn window_override_makes_config_non_empty() {
        let config = QuicConfig {
            max_stream_receive_window: Some(8_388_608),
            ..Default::default()
        };
        assert!(config.has_window_overrides());
        assert!(!config.is_empty());
    }

    #[test]
    fn blank_keep_alive_is_treated_as_absent() {
        let config = QuicConfig {
            keep_alive_period: Some(String::new()),
            ..Default::default()
        };
        assert!(config.keep_alive_value().is_none());
    }
}
