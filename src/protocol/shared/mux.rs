use serde::{Deserialize, Serialize};

/// Multiplexing settings for stream-based protocols.
///
/// Reference: <https://sing-box.sagernet.org/configuration/shared/multiplex/#outbound>
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MuxConfig {
    pub enabled: bool,
    /// Multiplexer implementation.
    pub protocol: MuxProtocol,
    /// Concurrent stream limit advertised to the peer.
    pub max_streams: Option<u32>,
    /// Enable padding to defeat traffic analysis. sing-box only.
    pub padding: bool,
    /// Brutal-style bandwidth hints; sing-box only.
    pub brutal: Option<BrutalOptions>,
}

impl MuxConfig {
    /// True when multiplexing should be emitted into a client config.
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MuxProtocol {
    #[default]
    Smux,
    Yamux,
    H2mux,
}

impl MuxProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smux => "smux",
            Self::Yamux => "yamux",
            Self::H2mux => "h2mux",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrutalOptions {
    pub enabled: bool,
    pub up_mbps: Option<u32>,
    pub down_mbps: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mux_is_inactive_by_default() {
        assert!(!MuxConfig::default().is_active());
    }

    #[test]
    fn mux_protocol_names_match_core_vocabulary() {
        assert_eq!(MuxProtocol::Smux.as_str(), "smux");
        assert_eq!(MuxProtocol::Yamux.as_str(), "yamux");
        assert_eq!(MuxProtocol::H2mux.as_str(), "h2mux");
    }

    #[test]
    fn mux_round_trips_through_json() {
        let mux = MuxConfig {
            enabled: true,
            protocol: MuxProtocol::Yamux,
            max_streams: Some(16),
            padding: true,
            brutal: None,
        };
        let json = serde_json::to_string(&mux).expect("serialize");
        let back: MuxConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(mux, back);
    }
}
