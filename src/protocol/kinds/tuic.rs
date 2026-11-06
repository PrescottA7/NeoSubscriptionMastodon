// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use serde::{Deserialize, Serialize};

use super::super::shared::{QuicConfig, TlsConfig};

/// TUIC outbound settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/outbound/tuic/>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TuicConfig {
    /// QUIC congestion control algorithm.
    pub congestion_control: CongestionControl,
    /// UDP packet relay mode.
    pub udp_relay_mode: UdpRelayMode,
    /// Relay UDP over QUIC streams instead of datagrams.
    pub udp_over_stream: bool,
    /// Attempt a 0-RTT handshake.
    pub zero_rtt_handshake: bool,
    /// Heartbeat interval, e.g. `10s`.
    pub heartbeat: Option<String>,
    pub tls: TlsConfig,
    pub quic: QuicConfig,
}

impl Default for TuicConfig {
    fn default() -> Self {
        Self {
            congestion_control: CongestionControl::Cubic,
            udp_relay_mode: UdpRelayMode::Native,
            udp_over_stream: false,
            zero_rtt_handshake: false,
            heartbeat: None,
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            quic: QuicConfig::default(),
        }
    }
}

impl TuicConfig {
    pub fn heartbeat_value(&self) -> Option<&str> {
        self.heartbeat.as_deref().filter(|value| !value.is_empty())
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.tls.enabled {
            return Err("tuic requires TLS to be enabled".to_owned());
        }
        if self.udp_over_stream && self.udp_relay_mode == UdpRelayMode::Quic {
            return Err("tuic udp_over_stream conflicts with udp_relay_mode=quic".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CongestionControl {
    #[default]
    Cubic,
    NewReno,
    Bbr,
}

impl CongestionControl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cubic => "cubic",
            Self::NewReno => "new_reno",
            Self::Bbr => "bbr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UdpRelayMode {
    #[default]
    Native,
    Quic,
}

impl UdpRelayMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Quic => "quic",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        let config = TuicConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.congestion_control.as_str(), "cubic");
        assert_eq!(config.udp_relay_mode.as_str(), "native");
    }

    #[test]
    fn udp_over_stream_conflicts_with_quic_relay() {
        let config = TuicConfig {
            udp_over_stream: true,
            udp_relay_mode: UdpRelayMode::Quic,
            ..Default::default()
        };
        let error = config.validate().expect_err("should reject");
        assert!(error.contains("udp_over_stream"));
    }

    #[test]
    fn udp_over_stream_is_allowed_with_native_relay() {
        let config = TuicConfig {
            udp_over_stream: true,
            udp_relay_mode: UdpRelayMode::Native,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn congestion_names_match_core_vocabulary() {
        assert_eq!(CongestionControl::NewReno.as_str(), "new_reno");
        assert_eq!(CongestionControl::Bbr.as_str(), "bbr");
    }

    #[test]
    fn blank_heartbeat_is_treated_as_absent() {
        let config = TuicConfig {
            heartbeat: Some(String::new()),
            ..Default::default()
        };
        assert!(config.heartbeat_value().is_none());
    }
}
