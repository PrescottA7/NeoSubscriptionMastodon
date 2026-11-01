//! Strongly typed proxy node configuration.
//!
//! A node row stores its protocol parameters as JSON. Deserialising that JSON
//! into [`ProxyKind`] guarantees every renderer sees a complete, validated
//! configuration instead of a hand-written URL template.

pub mod kinds;
pub mod server;
pub mod shared;

use serde::{Deserialize, Serialize};

pub use kinds::{
    AnytlsConfig, CongestionControl, Hysteria2Config, ObfsConfig, ObfsType, RealmConfig,
    TrojanConfig, TuicConfig, UdpRelayMode, VlessConfig, VlessFlow,
};
pub use server::{
    Fallback, InboundConfig, InboundProtocol, ServerConfig, ServerCore, ServerReality, ServerTls,
    Sockopt,
};
pub use shared::{MuxConfig, QuicConfig, RealityConfig, TlsConfig, Transport, TransportKind};

/// All protocols this server can publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "protocol", rename_all = "snake_case")]
pub enum ProxyKind {
    Vless(VlessConfig),
    Trojan(TrojanConfig),
    Hysteria2(Hysteria2Config),
    Tuic(TuicConfig),
    Anytls(AnytlsConfig),
}

/// Enumeration of protocol names, used by the admin API for form discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolName {
    Vless,
    Trojan,
    Hysteria2,
    Tuic,
    Anytls,
}

impl ProtocolName {
    pub const ALL: [Self; 5] = [
        Self::Vless,
        Self::Trojan,
        Self::Hysteria2,
        Self::Tuic,
        Self::Anytls,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vless => "vless",
            Self::Trojan => "trojan",
            Self::Hysteria2 => "hysteria2",
            Self::Tuic => "tuic",
            Self::Anytls => "anytls",
        }
    }

    /// Port a client should assume when the node does not override it.
    pub fn default_port(self) -> u16 {
        match self {
            Self::Vless | Self::Trojan | Self::Anytls => 443,
            Self::Hysteria2 | Self::Tuic => 443,
        }
    }

    /// True when the protocol always runs over TLS.
    pub fn requires_tls(self) -> bool {
        matches!(self, Self::Hysteria2 | Self::Tuic | Self::Anytls)
    }

    /// True when the protocol can be carried by a V2Ray stream transport.
    pub fn supports_transport(self) -> bool {
        matches!(self, Self::Vless | Self::Trojan)
    }
}

impl ProxyKind {
    pub fn protocol(&self) -> ProtocolName {
        match self {
            Self::Vless(_) => ProtocolName::Vless,
            Self::Trojan(_) => ProtocolName::Trojan,
            Self::Hysteria2(_) => ProtocolName::Hysteria2,
            Self::Tuic(_) => ProtocolName::Tuic,
            Self::Anytls(_) => ProtocolName::Anytls,
        }
    }

    pub fn protocol_str(&self) -> &'static str {
        self.protocol().as_str()
    }

    pub fn tls(&self) -> &TlsConfig {
        match self {
            Self::Vless(config) => &config.tls,
            Self::Trojan(config) => &config.tls,
            Self::Hysteria2(config) => &config.tls,
            Self::Tuic(config) => &config.tls,
            Self::Anytls(config) => &config.tls,
        }
    }

    /// Stream transport, or `None` for QUIC based protocols.
    pub fn transport(&self) -> Option<&Transport> {
        match self {
            Self::Vless(config) => Some(&config.transport),
            Self::Trojan(config) => Some(&config.transport),
            Self::Hysteria2(_) | Self::Tuic(_) | Self::Anytls(_) => None,
        }
    }

    pub fn mux(&self) -> Option<&MuxConfig> {
        match self {
            Self::Vless(config) => Some(&config.mux),
            Self::Trojan(config) => Some(&config.mux),
            Self::Hysteria2(_) | Self::Tuic(_) | Self::Anytls(_) => None,
        }
    }

    pub fn quic(&self) -> Option<&QuicConfig> {
        match self {
            Self::Hysteria2(config) => Some(&config.quic),
            Self::Tuic(config) => Some(&config.quic),
            Self::Vless(_) | Self::Trojan(_) | Self::Anytls(_) => None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let protocol = self.protocol();
        if protocol.requires_tls() && !self.tls().enabled {
            return Err(format!("{} requires TLS to be enabled", protocol.as_str()));
        }
        if !protocol.supports_transport() && self.transport().is_some() {
            return Err(format!(
                "{} does not support V2Ray stream transports",
                protocol.as_str()
            ));
        }
        match self {
            Self::Vless(config) => config.validate(),
            Self::Trojan(config) => config.validate(),
            Self::Hysteria2(config) => config.validate(),
            Self::Tuic(config) => config.validate(),
            Self::Anytls(config) => config.validate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_tls() -> TlsConfig {
        TlsConfig {
            enabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn tagged_json_round_trips_every_protocol() {
        let kinds = vec![
            ProxyKind::Vless(VlessConfig {
                tls: enabled_tls(),
                ..Default::default()
            }),
            ProxyKind::Trojan(TrojanConfig {
                tls: enabled_tls(),
                ..Default::default()
            }),
            ProxyKind::Hysteria2(Hysteria2Config {
                tls: enabled_tls(),
                ..Default::default()
            }),
            ProxyKind::Tuic(TuicConfig {
                tls: enabled_tls(),
                ..Default::default()
            }),
            ProxyKind::Anytls(AnytlsConfig {
                tls: enabled_tls(),
                ..Default::default()
            }),
        ];

        for kind in kinds {
            let json = serde_json::to_string(&kind).expect("serialize");
            let back: ProxyKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(kind, back);
            assert!(kind.validate().is_ok(), "{:?}", kind.validate());
        }
    }

    #[test]
    fn protocol_name_is_reflected_in_json() {
        let kind = ProxyKind::Hysteria2(Hysteria2Config {
            tls: enabled_tls(),
            ..Default::default()
        });
        let json = serde_json::to_value(&kind).expect("serialize");
        assert_eq!(json["protocol"], "hysteria2");
    }

    #[test]
    fn tls_requiring_protocol_is_rejected_without_tls() {
        let kind = ProxyKind::Tuic(TuicConfig {
            tls: TlsConfig::default(),
            ..Default::default()
        });
        let error = kind.validate().expect_err("should reject");
        assert!(error.contains("requires TLS"));
    }

    #[test]
    fn vless_requests_custom_tls_validation() {
        let kind = ProxyKind::Vless(VlessConfig {
            flow: Some("bogus".to_owned()),
            tls: enabled_tls(),
            ..Default::default()
        });
        assert!(kind.validate().is_err());
    }

    #[test]
    fn protocol_metadata_is_consistent() {
        assert!(!ProtocolName::Vless.requires_tls());
        assert!(ProtocolName::Hysteria2.requires_tls());
        assert!(ProtocolName::Vless.supports_transport());
        assert!(!ProtocolName::Tuic.supports_transport());
        assert_eq!(ProtocolName::Vless.as_str(), "vless");
        assert_eq!(ProtocolName::ALL.len(), 5);
    }
}
