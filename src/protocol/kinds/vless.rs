// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use serde::{Deserialize, Serialize};

use super::super::shared::{MuxConfig, TlsConfig, Transport};

/// VLESS outbound settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/outbound/vless/> and
/// <https://xtls.github.io/en/config/outbounds/vless.html>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VlessConfig {
    /// Sub-protocol shaping. Only `xtls-rprx-vision` is defined.
    pub flow: Option<String>,
    /// UDP packet encoding; `xudp` is what Xray-based clients use.
    pub packet_encoding: Option<String>,
    /// VLESS Encryption string; `none` disables it explicitly.
    pub encryption: Option<String>,
    pub tls: TlsConfig,
    pub transport: Transport,
    pub mux: MuxConfig,
}

impl Default for VlessConfig {
    fn default() -> Self {
        Self {
            flow: None,
            packet_encoding: Some("xudp".to_owned()),
            encryption: None,
            tls: TlsConfig::default(),
            transport: Transport::default(),
            mux: MuxConfig::default(),
        }
    }
}

impl VlessConfig {
    /// Non-empty, validated flow value.
    pub fn flow_value(&self) -> Option<&str> {
        self.flow
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn packet_encoding_value(&self) -> Option<&str> {
        self.packet_encoding
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// `encryption` as sent in share links. VLESS requires the field, defaulting to `none`.
    pub fn encryption_value(&self) -> &str {
        match self.encryption.as_deref() {
            Some(value) if !value.is_empty() => value,
            _ => "none",
        }
    }

    /// Vision only works over a raw TCP stream protected by TLS or REALITY.
    pub fn effective_flow(&self) -> Option<&str> {
        let flow = self.flow_value()?;
        let usable = self.transport.is_tcp() && self.tls.is_active();
        usable.then_some(flow)
    }

    pub fn validate(&self) -> Result<(), String> {
        if let Some(flow) = self.flow_value() {
            if !matches!(flow, "xtls-rprx-vision") {
                return Err(format!("unsupported vless flow `{flow}`"));
            }
            if !self.transport.is_tcp() {
                return Err(format!("vless flow `{flow}` requires the tcp transport"));
            }
            if !self.tls.is_active() {
                return Err(format!("vless flow `{flow}` requires TLS or REALITY"));
            }
        }
        Ok(())
    }
}

/// Allowed values for [`VlessConfig::flow`], exposed to the admin UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VlessFlow {
    #[serde(rename = "xtls-rprx-vision")]
    Vision,
}

impl VlessFlow {
    pub const ALL: [&'static str; 1] = ["xtls-rprx-vision"];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::shared::transport::TcpOptions;

    fn vision_config() -> VlessConfig {
        VlessConfig {
            flow: Some("xtls-rprx-vision".to_owned()),
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn vision_is_kept_over_tcp_tls() {
        assert_eq!(vision_config().effective_flow(), Some("xtls-rprx-vision"));
        assert!(vision_config().validate().is_ok());
    }

    #[test]
    fn vision_is_dropped_without_tls() {
        let mut config = vision_config();
        config.tls.enabled = false;
        assert!(config.effective_flow().is_none());
        assert!(config.validate().is_err());
    }

    #[test]
    fn vision_is_dropped_on_non_tcp_transport() {
        let mut config = vision_config();
        config.transport = Transport::Ws(Default::default());
        assert!(config.effective_flow().is_none());
        assert!(config.validate().is_err());
    }

    #[test]
    fn unknown_flow_is_rejected() {
        let mut config = vision_config();
        config.flow = Some("reality-only".to_owned());
        let error = config.validate().expect_err("should reject");
        assert!(error.contains("unsupported vless flow"));
    }

    #[test]
    fn blank_flow_means_no_shaping() {
        let mut config = vision_config();
        config.flow = Some("  ".to_owned());
        assert!(config.flow_value().is_none());
        assert!(config.effective_flow().is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn packet_encoding_defaults_to_xudp() {
        assert_eq!(VlessConfig::default().packet_encoding_value(), Some("xudp"));
    }

    #[test]
    fn encryption_defaults_to_none() {
        assert_eq!(VlessConfig::default().encryption_value(), "none");
    }

    #[test]
    fn tcp_options_are_accepted() {
        let config = VlessConfig {
            transport: Transport::Tcp(TcpOptions::default()),
            ..Default::default()
        };
        assert!(config.transport.is_tcp());
    }
}
