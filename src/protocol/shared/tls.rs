use serde::{Deserialize, Serialize};

/// TLS settings accepted by Xray, sing-box, and Mihomo outbounds.
///
/// Reference: <https://sing-box.sagernet.org/configuration/shared/tls/#outbound>
/// and <https://xtls.github.io/en/config/transports/stream.html>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TlsConfig {
    /// Whether a TLS layer is used at all.
    pub enabled: bool,
    /// SNI / certificate name; falls back to the node server address when empty.
    pub server_name: Option<String>,
    /// Skip certificate verification. Xray has removed `allowInsecure`, so only
    /// sing-box and Mihomo honour this.
    pub insecure: bool,
    /// ALPN protocols, e.g. `["h2", "http/1.1"]`.
    pub alpn: Vec<String>,
    /// uTLS client-hello fingerprint, e.g. `chrome`.
    pub fingerprint: Option<String>,
    /// Pinned leaf certificate SHA-256 (`pinnedPeerCertSha256`).
    pub certificate_sha256: Option<String>,
    /// Verify the leaf certificate against this name (`verifyPeerCertByName`).
    pub verify_peer_cert_by_name: Option<String>,
    /// Minimum TLS version, e.g. `1.2`.
    pub min_version: Option<String>,
    /// Maximum TLS version.
    pub max_version: Option<String>,
    /// REALITY handshake settings; mutually exclusive with a plain TLS layer.
    pub reality: Option<RealityConfig>,
}

impl TlsConfig {
    /// True when any TLS-ish layer should be emitted.
    pub fn is_active(&self) -> bool {
        self.enabled || self.reality_enabled()
    }

    pub fn reality_enabled(&self) -> bool {
        self.reality.as_ref().is_some_and(|reality| reality.is_usable())
    }

    /// Effective SNI, preferring the explicit value over the node address.
    pub fn effective_server_name<'a>(&'a self, server: &'a str) -> &'a str {
        match self.server_name.as_deref() {
            Some(value) if !value.is_empty() => value,
            _ => server,
        }
    }

    /// Effective fingerprint. Share links default to Chrome, and REALITY requires it.
    pub fn effective_fingerprint(&self) -> &str {
        match self.fingerprint.as_deref() {
            Some(value) if !value.is_empty() => value,
            _ => "chrome",
        }
    }

    /// ALPN list as a comma separated string, the form share links use.
    pub fn alpn_joined(&self) -> Option<String> {
        if self.alpn.is_empty() {
            None
        } else {
            Some(self.alpn.join(","))
        }
    }

    /// Non-empty pinned certificate hash.
    pub fn certificate_sha256_value(&self) -> Option<&str> {
        self.certificate_sha256.as_deref().filter(|value| !value.is_empty())
    }
}

/// REALITY handshake parameters.
///
/// Reference: <https://xtls.github.io/en/config/transports/stream.html#realityobject>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RealityConfig {
    pub enabled: bool,
    /// Server public key (share link `pbk`).
    pub public_key: String,
    /// Short id (share link `sid`), up to 16 hex characters.
    pub short_id: Option<String>,
    /// Path used when probing the destination (`spx`).
    pub spider_x: Option<String>,
    /// ML-DSA-65 verification key (`pqv`).
    pub mldsa65_verify: Option<String>,
}

impl RealityConfig {
    /// REALITY is only usable once a public key is present.
    pub fn is_usable(&self) -> bool {
        self.enabled && !self.public_key.is_empty()
    }

    /// Non-empty short id, trimmed of surrounding whitespace.
    pub fn short_id_value(&self) -> Option<&str> {
        self.short_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    pub fn spider_x_value(&self) -> Option<&str> {
        self.spider_x.as_deref().filter(|value| !value.is_empty())
    }

    pub fn mldsa65_verify_value(&self) -> Option<&str> {
        self.mldsa65_verify
            .as_deref()
            .filter(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_name_falls_back_to_server_address() {
        let tls = TlsConfig {
            enabled: true,
            ..Default::default()
        };
        assert_eq!(tls.effective_server_name("example.com"), "example.com");
    }

    #[test]
    fn blank_server_name_still_falls_back() {
        let tls = TlsConfig {
            enabled: true,
            server_name: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(tls.effective_server_name("example.com"), "example.com");
    }

    #[test]
    fn fingerprint_defaults_to_chrome() {
        assert_eq!(TlsConfig::default().effective_fingerprint(), "chrome");
        let tls = TlsConfig {
            fingerprint: Some("firefox".to_owned()),
            ..Default::default()
        };
        assert_eq!(tls.effective_fingerprint(), "firefox");
    }

    #[test]
    fn reality_requires_public_key() {
        let mut reality = RealityConfig {
            enabled: true,
            public_key: String::new(),
            ..Default::default()
        };
        assert!(!reality.is_usable());

        reality.public_key = "abc".to_owned();
        assert!(reality.is_usable());
    }

    #[test]
    fn is_active_covers_reality_without_plain_tls() {
        let tls = TlsConfig {
            enabled: false,
            reality: Some(RealityConfig {
                enabled: true,
                public_key: "key".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(tls.is_active());
        assert!(tls.reality_enabled());
    }

    #[test]
    fn alpn_join_is_none_when_empty() {
        assert!(TlsConfig::default().alpn_joined().is_none());
        let tls = TlsConfig {
            alpn: vec!["h2".to_owned(), "http/1.1".to_owned()],
            ..Default::default()
        };
        assert_eq!(tls.alpn_joined().as_deref(), Some("h2,http/1.1"));
    }
}
