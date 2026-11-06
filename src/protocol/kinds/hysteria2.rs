// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use serde::{Deserialize, Serialize};

use super::super::shared::{QuicConfig, TlsConfig};

/// Hysteria2 outbound settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/outbound/hysteria2/>
/// and <https://wiki.metacubex.one/en/config/proxies/hysteria2/>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hysteria2Config {
    /// Server port range list enabling port hopping, e.g. `["2080:3000"]`.
    pub server_ports: Vec<String>,
    /// Port hopping interval, e.g. `30s`. Defaults to 30 seconds when hopping is on.
    pub hop_interval: Option<String>,
    /// Upper bound for randomised hopping intervals.
    pub hop_interval_max: Option<String>,
    /// Brutal-style maximum upload bandwidth in Mbps.
    pub up_mbps: Option<u32>,
    /// Brutal-style maximum download bandwidth in Mbps.
    pub down_mbps: Option<u32>,
    /// QUIC traffic obfuscation.
    pub obfs: Option<ObfsConfig>,
    /// BBR congestion profile: `conservative`, `standard`, or `aggressive`.
    pub bbr_profile: Option<String>,
    /// Log Hysteria Brutal congestion-control debugging information.
    pub brutal_debug: bool,
    /// Disable Chrome QUIC fingerprint parroting.
    pub disable_chrome_parrot: bool,
    /// Hysteria Realm rendezvous settings.
    pub realm: Option<RealmConfig>,
    pub tls: TlsConfig,
    pub quic: QuicConfig,
}

impl Default for Hysteria2Config {
    fn default() -> Self {
        Self {
            server_ports: Vec::new(),
            hop_interval: None,
            hop_interval_max: None,
            up_mbps: None,
            down_mbps: None,
            obfs: None,
            bbr_profile: None,
            brutal_debug: false,
            disable_chrome_parrot: false,
            realm: None,
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            quic: QuicConfig::default(),
        }
    }
}

impl Hysteria2Config {
    /// True when a port range list is configured.
    pub fn has_port_hopping(&self) -> bool {
        !self.server_ports.is_empty()
    }

    /// Port range list as the comma separated string share links use.
    pub fn server_ports_joined(&self) -> Option<String> {
        if self.server_ports.is_empty() {
            None
        } else {
            Some(self.server_ports.join(","))
        }
    }

    pub fn hop_interval_value(&self) -> Option<&str> {
        self.hop_interval
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    /// Active obfuscator, if any.
    pub fn obfs_value(&self) -> Option<&ObfsConfig> {
        self.obfs.as_ref().filter(|obfs| obfs.is_active())
    }

    /// Brutal bandwidth in the `"<up> Mbps"` form Mihomo expects.
    pub fn up_bandwidth(&self) -> Option<String> {
        self.up_mbps.map(|value| format!("{value} Mbps"))
    }

    pub fn down_bandwidth(&self) -> Option<String> {
        self.down_mbps.map(|value| format!("{value} Mbps"))
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.tls.enabled {
            return Err("hysteria2 requires TLS to be enabled".to_owned());
        }
        if self.realm.is_some() && self.has_port_hopping() {
            return Err("hysteria2 realm conflicts with server_ports".to_owned());
        }
        if let Some(obfs) = self.obfs.as_ref() {
            obfs.validate()?;
        }
        for range in &self.server_ports {
            if parse_port_range(range).is_none() {
                return Err(format!("invalid hysteria2 port range `{range}`"));
            }
        }
        Ok(())
    }
}

/// QUIC traffic obfuscator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObfsConfig {
    pub obfs_type: ObfsType,
    pub password: String,
    /// Minimum on-wire packet size in bytes; gecko only.
    pub min_packet_size: Option<u32>,
    /// Maximum on-wire packet size in bytes; gecko only.
    pub max_packet_size: Option<u32>,
}

impl Default for ObfsConfig {
    fn default() -> Self {
        Self {
            obfs_type: ObfsType::Salamander,
            password: String::new(),
            min_packet_size: None,
            max_packet_size: None,
        }
    }
}

impl ObfsConfig {
    /// Obfuscation only takes effect once a password is set.
    pub fn is_active(&self) -> bool {
        !self.password.is_empty()
    }

    pub fn type_name(&self) -> &'static str {
        self.obfs_type.as_str()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.password.is_empty() {
            return Err("hysteria2 obfs requires a password".to_owned());
        }
        if let (Some(min), Some(max)) = (self.min_packet_size, self.max_packet_size)
            && min > max
        {
            return Err("hysteria2 obfs min_packet_size exceeds max_packet_size".to_owned());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObfsType {
    #[default]
    Salamander,
    Gecko,
}

impl ObfsType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Salamander => "salamander",
            Self::Gecko => "gecko",
        }
    }
}

/// Hysteria Realm rendezvous settings.
///
/// Reference: <https://sing-box.sagernet.org/configuration/service/hysteria-realm/>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RealmConfig {
    pub server_url: String,
    pub token: Option<String>,
    pub realm_id: String,
    pub stun_servers: Vec<String>,
    /// Restrict traversal to a single IP version: `4` or `6`.
    pub ip_version: Option<u8>,
    pub port_mapping: Option<PortMapping>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PortMapping {
    pub enabled: bool,
    pub timeout: Option<String>,
    pub lifetime: Option<String>,
}

/// Parses `start:end` or `single` port ranges, mirroring sing-box's syntax.
///
/// Reference: <https://sing-box.sagernet.org/configuration/shared/listen/#server_ports>
pub fn parse_port_range(value: &str) -> Option<(u16, u16)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let (start, end) = match value.split_once(':') {
        Some((start, end)) => (start.trim(), end.trim()),
        None => (value, value),
    };

    let start: u16 = start.parse().ok()?;
    let end: u16 = end.parse().ok()?;
    if start == 0 || end == 0 || start > end {
        return None;
    }
    Some((start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_is_enabled_by_default() {
        assert!(Hysteria2Config::default().tls.enabled);
        assert!(Hysteria2Config::default().validate().is_ok());
    }

    #[test]
    fn port_ranges_parse_both_forms() {
        assert_eq!(parse_port_range("2080:3000"), Some((2080, 3000)));
        assert_eq!(parse_port_range("443"), Some((443, 443)));
        assert_eq!(parse_port_range(" 1000 : 2000 "), Some((1000, 2000)));
        assert!(parse_port_range("3000:2080").is_none());
        assert!(parse_port_range("0:100").is_none());
        assert!(parse_port_range("abc").is_none());
        assert!(parse_port_range("").is_none());
    }

    #[test]
    fn invalid_port_range_fails_validation() {
        let config = Hysteria2Config {
            server_ports: vec!["oops".to_owned()],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn hopping_serialises_as_comma_list() {
        let config = Hysteria2Config {
            server_ports: vec!["2080:3000".to_owned(), "4000".to_owned()],
            ..Default::default()
        };
        assert!(config.has_port_hopping());
        assert_eq!(
            config.server_ports_joined().as_deref(),
            Some("2080:3000,4000")
        );
    }

    #[test]
    fn obfs_requires_a_password_to_be_active() {
        let inactive = ObfsConfig::default();
        assert!(!inactive.is_active());

        let active = ObfsConfig {
            password: "secret".to_owned(),
            ..Default::default()
        };
        assert!(active.is_active());
        assert!(active.validate().is_ok());
    }

    #[test]
    fn obfs_without_password_is_invalid() {
        let config = Hysteria2Config {
            obfs: Some(ObfsConfig::default()),
            ..Default::default()
        };
        let error = config.validate().expect_err("should reject");
        assert!(error.contains("obfs requires a password"));
    }

    #[test]
    fn obfs_packet_size_bounds_are_validated() {
        let obfs = ObfsConfig {
            obfs_type: ObfsType::Gecko,
            password: "secret".to_owned(),
            min_packet_size: Some(1200),
            max_packet_size: Some(512),
        };
        assert!(obfs.validate().is_err());
    }

    #[test]
    fn realm_conflicts_with_port_hopping() {
        let config = Hysteria2Config {
            server_ports: vec!["443".to_owned()],
            realm: Some(RealmConfig::default()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn bandwidth_renders_in_mihomo_form() {
        let config = Hysteria2Config {
            up_mbps: Some(30),
            down_mbps: Some(200),
            ..Default::default()
        };
        assert_eq!(config.up_bandwidth().as_deref(), Some("30 Mbps"));
        assert_eq!(config.down_bandwidth().as_deref(), Some("200 Mbps"));
    }
}
