//! Server-side (inbound) configuration.
//!
//! `ProxyKind` describes how a *client* connects to a node. This module describes
//! how the node itself runs: the inbound listeners the proxy server exposes.
//!
//! The two are deliberately separate. A subscription needs only the outbound
//! half, while the machine operator needs the inbound half — including the
//! REALITY private key, which must never leave the server, and the certificate
//! paths, which mean nothing to a client.

use serde::{Deserialize, Serialize};

/// Protocol the server listens for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InboundProtocol {
    #[default]
    Vless,
    Trojan,
    Hysteria2,
    Tuic,
    Anytls,
}

impl InboundProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vless => "vless",
            Self::Trojan => "trojan",
            Self::Hysteria2 => "hysteria2",
            Self::Tuic => "tuic",
            Self::Anytls => "anytls",
        }
    }
}

/// Everything needed to run the node, as opposed to reach it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub inbound: InboundConfig,
    /// Certificate files for a plain TLS listener. REALITY needs none.
    pub tls: ServerTls,
    pub reality: ServerReality,
    /// Fallback destination for non-proxy traffic on the inbound port.
    pub fallbacks: Vec<Fallback>,
    /// Applied to the inbound socket; mostly used to tune backlog/buffers.
    pub sockopt: Sockopt,
    /// `xray` or `hysteria2`; decides which exporter shape is used.
    pub core: ServerCore,
}

impl ServerConfig {
    /// True when the listener terminates REALITY rather than plain TLS.
    pub fn uses_reality(&self) -> bool {
        self.reality.enabled
    }

    /// Whether `core` can actually run this listener. Xray has no Hysteria2
    /// inbound, and the Hysteria2 exporter only understands Hysteria2 inbounds,
    /// so those two cross-combinations would silently produce a config with no
    /// accounts.
    pub fn supports_core(&self, core: ServerCore) -> bool {
        match core {
            ServerCore::Xray => self.inbound.protocol != InboundProtocol::Hysteria2,
            ServerCore::Hysteria2 => self.inbound.protocol == InboundProtocol::Hysteria2,
        }
    }

    /// Cores that can render this listener, in a stable order.
    pub fn supported_cores(&self) -> Vec<ServerCore> {
        let mut cores = Vec::new();
        if self.supports_core(ServerCore::Xray) {
            cores.push(ServerCore::Xray);
        }
        if self.supports_core(ServerCore::Hysteria2) {
            cores.push(ServerCore::Hysteria2);
        }
        cores
    }
}

/// Which proxy server the configuration targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServerCore {
    #[default]
    Xray,
    Hysteria2,
}

impl ServerCore {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Xray => "xray",
            Self::Hysteria2 => "hysteria2",
        }
    }

    /// Parses the wire name, returning `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "xray" => Some(Self::Xray),
            "hysteria2" => Some(Self::Hysteria2),
            _ => None,
        }
    }
}

/// The inbound listener itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InboundConfig {
    pub protocol: InboundProtocol,
    pub listen: String,
    pub port: u16,
    /// Listener tag used in Xray routing rules.
    pub tag: String,
    /// Xray `sniffing`: inspect the payload to recover the real destination.
    pub sniffing: bool,
    /// Xray `destOverride` when sniffing is on.
    pub dest_override: Vec<String>,
    /// Share-link vocabulary of the transport, e.g. `tcp`, `ws`, `grpc`, `xhttp`.
    pub transport: String,
    /// Path for `ws`, `httpupgrade`, and `xhttp` listeners.
    pub path: String,
    /// `Host` header accepted on the listener; empty means "any".
    pub host: String,
    /// gRPC service name.
    pub service_name: String,
}

impl Default for InboundConfig {
    fn default() -> Self {
        Self {
            protocol: InboundProtocol::Vless,
            listen: "0.0.0.0".to_owned(),
            port: 443,
            tag: "inbound".to_owned(),
            sniffing: true,
            dest_override: vec!["http".to_owned(), "tls".to_owned(), "quic".to_owned()],
            transport: "tcp".to_owned(),
            path: "/".to_owned(),
            host: String::new(),
            service_name: String::new(),
        }
    }
}

/// Certificate material for a plain TLS listener.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ServerTls {
    /// PEM file paths or inline PEM; Xray accepts both.
    pub certificate_file: String,
    pub key_file: String,
    /// `tls` or `xtls`; only meaningful in Xray.
    pub security: String,
    /// ALPN advertised by the listener.
    pub alpn: Vec<String>,
}

/// REALITY listener settings.
///
/// The private key stays on the server; the matching public key is what clients
/// receive. Keeping both in one place lets the exporter derive the client half
/// instead of asking the operator to copy it by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ServerReality {
    pub enabled: bool,
    /// Base64 X25519 private key, as produced by `xray x25519`.
    pub private_key: String,
    /// Domains the server is allowed to present.
    pub server_names: Vec<String>,
    /// Destination probed for unauthenticated clients, e.g. `www.example.com:443`.
    pub target: String,
    /// Accepted short ids; an empty entry allows clients without one.
    pub short_ids: Vec<String>,
    /// ML-DSA-65 seed for post-quantum verification.
    pub mldsa65_seed: String,
    /// Allowed client clock skew in seconds.
    pub max_time_diff: u32,
    /// Reject clients older or newer than this Xray version.
    pub min_client_ver: String,
    pub max_client_ver: String,
}

impl ServerReality {
    /// True when the listener can actually serve REALITY.
    pub fn is_usable(&self) -> bool {
        self.enabled && !self.private_key.is_empty()
    }

    /// First short id, which is what a rendered client uses.
    pub fn primary_short_id(&self) -> Option<&str> {
        self.short_ids
            .iter()
            .map(String::as_str)
            .find(|value| !value.is_empty())
    }
}

/// Xray inbound `fallbacks` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Fallback {
    /// Destination, e.g. `80` for a local port or `127.0.0.1:8080`.
    pub dest: String,
    /// Only fall back for this ALPN.
    pub alpn: String,
    /// Only fall back for this SNI.
    pub name: String,
    /// Only fall back for this path.
    pub path: String,
}

/// Inbound socket options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Sockopt {
    /// `tcpFastOpen` on the listening socket.
    pub tcp_fast_open: bool,
    /// `acceptProxyProtocol` for listeners behind a TCP proxy.
    pub accept_proxy_protocol: bool,
    /// `mark` on Linux.
    pub mark: Option<u32>,
    pub tcp_keep_alive_interval: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_config_targets_xray() {
        let config = ServerConfig::default();
        assert_eq!(config.core, ServerCore::Xray);
        assert!(!config.uses_reality());
        assert_eq!(config.inbound.port, 443);
    }

    #[test]
    fn reality_needs_a_private_key_to_be_usable() {
        let mut reality = ServerReality {
            enabled: true,
            ..Default::default()
        };
        assert!(!reality.is_usable());

        reality.private_key = "PRIVATE".to_owned();
        assert!(reality.is_usable());
    }

    #[test]
    fn primary_short_id_skips_the_empty_entry() {
        let reality = ServerReality {
            short_ids: vec![String::new(), "ab".to_owned()],
            ..Default::default()
        };
        assert_eq!(reality.primary_short_id(), Some("ab"));
    }

    #[test]
    fn server_config_round_trips_through_json() {
        let config = ServerConfig {
            core: ServerCore::Hysteria2,
            reality: ServerReality {
                enabled: true,
                private_key: "PRIVATE".to_owned(),
                short_ids: vec!["ab".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        };

        let json = serde_json::to_string(&config).expect("serialize");
        let back: ServerConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, back);
    }

    #[test]
    fn unknown_fields_are_rejected_loudly() {
        // A typo in a security setting must not be silently ignored.
        let result: Result<ServerConfig, _> = serde_json::from_str(r#"{"inboud":{}}"#);
        assert!(result.is_err());
    }

    #[test]
    fn each_listener_has_exactly_one_runnable_core() {
        // Xray has no Hysteria2 inbound, and the Hysteria2 exporter only knows
        // Hysteria2; offering the other core would emit a config with no accounts.
        let vless = ServerConfig::default();
        assert!(vless.supports_core(ServerCore::Xray));
        assert!(!vless.supports_core(ServerCore::Hysteria2));
        assert_eq!(vless.supported_cores(), vec![ServerCore::Xray]);

        let hysteria2 = ServerConfig {
            inbound: InboundConfig {
                protocol: InboundProtocol::Hysteria2,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!hysteria2.supports_core(ServerCore::Xray));
        assert!(hysteria2.supports_core(ServerCore::Hysteria2));
        assert_eq!(hysteria2.supported_cores(), vec![ServerCore::Hysteria2]);
    }

    #[test]
    fn core_names_parse_both_ways() {
        for core in [ServerCore::Xray, ServerCore::Hysteria2] {
            assert_eq!(ServerCore::parse(core.as_str()), Some(core));
        }
        assert_eq!(ServerCore::parse("sing-box"), None);
    }
}
