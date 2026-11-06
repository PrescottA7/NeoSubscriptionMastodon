// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use serde::{Deserialize, Serialize};

use super::super::shared::{MuxConfig, TlsConfig, Transport};

/// Trojan outbound settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/outbound/trojan/>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrojanConfig {
    /// Drop UDP support; Trojan servers commonly disable it.
    pub udp: bool,
    pub tls: TlsConfig,
    pub transport: Transport,
    pub mux: MuxConfig,
}

impl TrojanConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.tls.enabled {
            return Err("trojan requires TLS to be enabled".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_is_mandatory() {
        let config = TrojanConfig::default();
        let error = config.validate().expect_err("should reject");
        assert!(error.contains("requires TLS"));
    }

    #[test]
    fn tls_enabled_config_is_valid() {
        let config = TrojanConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = r#"{"tls":{"enabled":true},"surprise":1}"#;
        assert!(serde_json::from_str::<TrojanConfig>(json).is_err());
    }
}
