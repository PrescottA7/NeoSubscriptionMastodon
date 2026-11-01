use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// Stream transport used below the proxy protocol.
///
/// Reference: <https://sing-box.sagernet.org/configuration/shared/v2ray-transport/>
/// and <https://xtls.github.io/en/config/transports/xhttp.html>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
// The variants differ in size, but the enum is built once per node and never
// stored in large collections, so boxing would cost more than it saves.
#[allow(clippy::large_enum_variant)]
pub enum Transport {
    #[serde(rename = "tcp")]
    Tcp(TcpOptions),
    #[serde(rename = "ws")]
    Ws(WebSocketOptions),
    #[serde(rename = "grpc")]
    Grpc(GrpcOptions),
    #[serde(rename = "http")]
    Http(HttpOptions),
    #[serde(rename = "httpupgrade")]
    HttpUpgrade(HttpUpgradeOptions),
    #[serde(rename = "xhttp")]
    Xhttp(XhttpOptions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct TcpOptions {
    /// TCP header camouflage; Xray only.
    #[serde(default)]
    pub header_type: TcpHeaderType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TcpHeaderType {
    #[default]
    None,
    Http,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct WebSocketOptions {
    pub path: Option<String>,
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    /// Early data size in bytes; `2048` is the value V2RayN sends.
    pub max_early_data: Option<u32>,
    /// Header carrying early data. Xray uses `Sec-WebSocket-Protocol`.
    pub early_data_header_name: Option<String>,
}

impl WebSocketOptions {
    pub fn path_or_default(&self) -> &str {
        non_empty(self.path.as_deref()).unwrap_or("/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct GrpcOptions {
    pub service_name: Option<String>,
    /// Xray transport mode: `gun`, `multi`, or `guna`.
    pub mode: Option<String>,
    /// `:authority` override; Xray only.
    pub authority: Option<String>,
}

impl GrpcOptions {
    pub fn service_name_or_default(&self) -> &str {
        non_empty(self.service_name.as_deref()).unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct HttpOptions {
    /// Host values used for the `Host` header; the first one wins in most cores.
    pub host: Vec<String>,
    pub path: Option<String>,
    pub method: Option<String>,
}

impl HttpOptions {
    pub fn host_or_empty(&self) -> &str {
        self.host.first().map(String::as_str).unwrap_or("")
    }

    pub fn path_or_default(&self) -> &str {
        non_empty(self.path.as_deref()).unwrap_or("/")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct HttpUpgradeOptions {
    pub host: Option<String>,
    pub path: Option<String>,
}

impl HttpUpgradeOptions {
    pub fn path_or_default(&self) -> &str {
        non_empty(self.path.as_deref()).unwrap_or("/")
    }
}

/// XHTTP (formerly SplitHTTP) options.
///
/// The admin API stores these in `snake_case` like every other field; Xray's
/// own `camelCase` names are produced only at render time by [`XhttpExtra::to_wire`].
///
/// Reference: <https://github.com/XTLS/Xray-core/blob/main/infra/conf/transport_method.go>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct XhttpOptions {
    pub host: Option<String>,
    pub path: Option<String>,
    pub mode: XhttpMode,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    /// `extra` blob. Xray accepts either a JSON string or an object.
    pub extra: Option<XhttpExtra>,
}

impl XhttpOptions {
    pub fn path_or_default(&self) -> &str {
        non_empty(self.path.as_deref()).unwrap_or("/")
    }

    /// `extra` rendered as the JSON string Xray expects inside `xhttpSettings`.
    pub fn extra_json(&self) -> Option<String> {
        let extra = self.extra.as_ref()?;
        let wire = extra.to_wire();
        if wire.is_empty() {
            return None;
        }
        serde_json::to_string(&Value::Object(wire)).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum XhttpMode {
    /// Let the core choose (Xray's own default is `auto`).
    #[default]
    Auto,
    /// Upload and download both streamed over a single request.
    StreamOne,
    /// Upload streamed, download streamed, two requests.
    StreamUp,
    /// Download streamed, upload split into many POSTs.
    PacketUp,
}

impl XhttpMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::StreamOne => "stream-one",
            Self::StreamUp => "stream-up",
            Self::PacketUp => "packet-up",
        }
    }
}

/// The `extra` object understood by Xray's XHTTP transport.
///
/// Stored in `snake_case` like the rest of the admin API; [`Self::to_wire`]
/// produces the `camelCase` names Xray actually parses.
///
/// Reference: <https://github.com/XTLS/Xray-core/blob/main/infra/conf/transport_method.go>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct XhttpExtra {
    /// Padding length range applied to the request, e.g. `100-1000`.
    pub x_padding_bytes: Option<String>,
    /// Apply the padding as an obfuscated blob instead of literal characters.
    pub x_padding_obfs_mode: Option<bool>,
    pub x_padding_key: Option<String>,
    pub x_padding_header: Option<String>,
    /// `cookie`, `header`, `query`, or `queryInHeader`.
    pub x_padding_placement: Option<String>,
    /// `repeat-x` or `tokenish`.
    pub x_padding_method: Option<String>,
    /// HTTP method used for uplink POSTs.
    pub uplink_http_method: Option<String>,
    pub session_id_placement: Option<String>,
    pub session_id_key: Option<String>,
    pub session_id_table: Option<String>,
    pub session_id_length: Option<String>,
    pub seq_placement: Option<String>,
    pub seq_key: Option<String>,
    pub uplink_data_placement: Option<String>,
    pub uplink_data_key: Option<String>,
    pub uplink_chunk_size: Option<String>,
    pub no_grpc_header: Option<bool>,
    pub no_sse_header: Option<bool>,
    pub sc_max_each_post_bytes: Option<String>,
    pub sc_min_posts_interval_ms: Option<String>,
    pub sc_max_buffered_posts: Option<i64>,
    pub sc_stream_up_server_secs: Option<String>,
    pub server_max_header_bytes: Option<i32>,
    pub xmux: Option<XmuxOptions>,
    /// Download settings override; used for CDN/edge shaped deployments.
    pub download_settings: Option<DownloadSettings>,
}

impl XhttpExtra {
    /// True when nothing would be emitted.
    pub fn is_empty(&self) -> bool {
        self.to_wire().is_empty()
    }

    /// Builds the `camelCase` object Xray parses, omitting unset fields.
    pub fn to_wire(&self) -> Map<String, Value> {
        let mut map = Map::new();

        fn text(map: &mut Map<String, Value>, key: &str, value: &Option<String>) {
            if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                map.insert(key.to_owned(), json!(value));
            }
        }

        text(&mut map, "xPaddingBytes", &self.x_padding_bytes);
        text(&mut map, "xPaddingKey", &self.x_padding_key);
        text(&mut map, "xPaddingHeader", &self.x_padding_header);
        text(&mut map, "xPaddingPlacement", &self.x_padding_placement);
        text(&mut map, "xPaddingMethod", &self.x_padding_method);
        text(&mut map, "uplinkHTTPMethod", &self.uplink_http_method);
        text(&mut map, "sessionIDPlacement", &self.session_id_placement);
        text(&mut map, "sessionIDKey", &self.session_id_key);
        text(&mut map, "sessionIDTable", &self.session_id_table);
        text(&mut map, "sessionIDLength", &self.session_id_length);
        text(&mut map, "seqPlacement", &self.seq_placement);
        text(&mut map, "seqKey", &self.seq_key);
        text(&mut map, "uplinkDataPlacement", &self.uplink_data_placement);
        text(&mut map, "uplinkDataKey", &self.uplink_data_key);
        text(&mut map, "uplinkChunkSize", &self.uplink_chunk_size);
        text(&mut map, "scMaxEachPostBytes", &self.sc_max_each_post_bytes);
        text(&mut map, "scMinPostsIntervalMs", &self.sc_min_posts_interval_ms);
        text(&mut map, "scStreamUpServerSecs", &self.sc_stream_up_server_secs);

        if let Some(value) = self.x_padding_obfs_mode {
            map.insert("xPaddingObfsMode".to_owned(), json!(value));
        }
        if let Some(value) = self.no_grpc_header {
            map.insert("noGRPCHeader".to_owned(), json!(value));
        }
        if let Some(value) = self.no_sse_header {
            map.insert("noSSEHeader".to_owned(), json!(value));
        }
        if let Some(value) = self.sc_max_buffered_posts.filter(|value| *value > 0) {
            map.insert("scMaxBufferedPosts".to_owned(), json!(value));
        }
        if let Some(value) = self.server_max_header_bytes.filter(|value| *value > 0) {
            map.insert("serverMaxHeaderBytes".to_owned(), json!(value));
        }
        if let Some(xmux) = self.xmux.as_ref().map(XmuxOptions::to_wire).filter(|wire| !wire.is_empty()) {
            map.insert("xmux".to_owned(), Value::Object(xmux));
        }
        if let Some(download) = self.download_settings.as_ref().map(DownloadSettings::to_wire).filter(|wire| !wire.is_empty()) {
            map.insert("downloadSettings".to_owned(), Value::Object(download));
        }

        map
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct XmuxOptions {
    pub max_concurrency: Option<String>,
    pub max_connections: Option<String>,
    pub c_max_reuse_times: Option<String>,
    pub c_max_lifetime_ms: Option<String>,
    pub h_max_request_times: Option<String>,
    pub h_max_reusable_secs: Option<String>,
    pub h_keep_alive_period: Option<i64>,
}

impl XmuxOptions {
    /// True when nothing would be emitted.
    pub fn is_empty(&self) -> bool {
        self.to_wire().is_empty()
    }

    /// Builds the `camelCase` object Xray parses.
    pub fn to_wire(&self) -> Map<String, Value> {
        let mut map = Map::new();

        for (key, value) in [
            ("maxConcurrency", &self.max_concurrency),
            ("maxConnections", &self.max_connections),
            ("cMaxReuseTimes", &self.c_max_reuse_times),
            ("cMaxLifetimeMs", &self.c_max_lifetime_ms),
            ("hMaxRequestTimes", &self.h_max_request_times),
            ("hMaxReusableSecs", &self.h_max_reusable_secs),
        ] {
            if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                map.insert(key.to_owned(), json!(value));
            }
        }
        if let Some(value) = self.h_keep_alive_period.filter(|value| *value > 0) {
            map.insert("hKeepAlivePeriod".to_owned(), json!(value));
        }

        map
    }
}

/// Xray's `downloadSettings`: a full outbound override for the download stream.
///
/// Reference: <https://github.com/XTLS/Xray-core/blob/main/infra/conf/transport_method.go>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadSettings {
    /// `address` override; the node server is used when empty.
    pub address: Option<String>,
    pub port: Option<u16>,
    /// `xhttp`, `ws`, `grpc`, `httpupgrade`, or `http`.
    pub network: Option<String>,
    /// `tls` or `reality`.
    pub security: Option<String>,
    pub tls_settings: Option<Box<DownloadTlsSettings>>,
    /// Transport options of the same shape as the outbound's own.
    pub xhttp_settings: Option<Box<DownloadXhttpSettings>>,
}

impl DownloadSettings {
    /// Builds the `camelCase` object Xray parses.
    pub fn to_wire(&self) -> Map<String, Value> {
        let mut map = Map::new();

        if let Some(address) = self.address.as_deref().filter(|value| !value.is_empty()) {
            map.insert("address".to_owned(), json!(address));
        }
        if let Some(port) = self.port {
            map.insert("port".to_owned(), json!(port));
        }
        if let Some(network) = self.network.as_deref().filter(|value| !value.is_empty()) {
            map.insert("network".to_owned(), json!(network));
        }
        if let Some(security) = self.security.as_deref().filter(|value| !value.is_empty()) {
            map.insert("security".to_owned(), json!(security));
        }
        if let Some(settings) = self.tls_settings.as_ref() {
            let wire = settings.to_wire();
            if !wire.is_empty() {
                let key = if self.security.as_deref() == Some("reality") {
                    "realitySettings"
                } else {
                    "tlsSettings"
                };
                map.insert(key.to_owned(), Value::Object(wire));
            }
        }
        if let Some(settings) = self.xhttp_settings.as_ref() {
            let wire = settings.to_wire();
            if !wire.is_empty() {
                map.insert("xhttpSettings".to_owned(), Value::Object(wire));
            }
        }

        map
    }
}

/// Minimal TLS/REALITY override for `downloadSettings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadTlsSettings {
    pub server_name: Option<String>,
    pub fingerprint: Option<String>,
    pub public_key: Option<String>,
    pub short_id: Option<String>,
}

impl DownloadTlsSettings {
    fn to_wire(&self) -> Map<String, Value> {
        let mut map = Map::new();
        for (key, value) in [
            ("serverName", &self.server_name),
            ("fingerprint", &self.fingerprint),
            ("publicKey", &self.public_key),
            ("shortId", &self.short_id),
        ] {
            if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                map.insert(key.to_owned(), json!(value));
            }
        }
        map
    }
}

/// Transport override for `downloadSettings`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadXhttpSettings {
    pub host: Option<String>,
    pub path: Option<String>,
    pub mode: Option<String>,
}

impl DownloadXhttpSettings {
    fn to_wire(&self) -> Map<String, Value> {
        let mut map = Map::new();
        for (key, value) in [
            ("host", &self.host),
            ("path", &self.path),
            ("mode", &self.mode),
        ] {
            if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                map.insert(key.to_owned(), json!(value));
            }
        }
        map
    }
}

/// Canonical wire name used by share links and core configs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    #[default]
    Tcp,
    Ws,
    Grpc,
    Http,
    HttpUpgrade,
    Xhttp,
}

impl Transport {
    pub fn kind(&self) -> TransportKind {
        match self {
            Self::Tcp(_) => TransportKind::Tcp,
            Self::Ws(_) => TransportKind::Ws,
            Self::Grpc(_) => TransportKind::Grpc,
            Self::Http(_) => TransportKind::Http,
            Self::HttpUpgrade(_) => TransportKind::HttpUpgrade,
            Self::Xhttp(_) => TransportKind::Xhttp,
        }
    }

    /// Name used by share links (`type=`).
    pub fn uri_name(&self) -> &'static str {
        match self.kind() {
            TransportKind::Tcp => "tcp",
            TransportKind::Ws => "ws",
            TransportKind::Grpc => "grpc",
            TransportKind::Http => "http",
            TransportKind::HttpUpgrade => "httpupgrade",
            TransportKind::Xhttp => "xhttp",
        }
    }

    /// Host advertised by the transport, used for TLS SNI fallbacks.
    pub fn host_hint(&self) -> Option<&str> {
        match self {
            Self::Tcp(_) => None,
            Self::Ws(options) => non_empty(options.host.as_deref()),
            Self::Grpc(options) => non_empty(options.authority.as_deref()),
            Self::Http(options) => non_empty(options.host.first().map(String::as_str)),
            Self::HttpUpgrade(options) => non_empty(options.host.as_deref()),
            Self::Xhttp(options) => non_empty(options.host.as_deref()),
        }
    }

    /// Path advertised by the transport, if any.
    pub fn path_hint(&self) -> Option<&str> {
        match self {
            Self::Tcp(_) | Self::Grpc(_) => None,
            Self::Ws(options) => non_empty(options.path.as_deref()),
            Self::Http(options) => non_empty(options.path.as_deref()),
            Self::HttpUpgrade(options) => non_empty(options.path.as_deref()),
            Self::Xhttp(options) => non_empty(options.path.as_deref()),
        }
    }

    /// True when the transport carries a plain TCP stream (affects `flow=xtls-rprx-vision`).
    pub fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp(_))
    }
}

impl Default for Transport {
    fn default() -> Self {
        Self::Tcp(TcpOptions::default())
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_kinds_round_trip_through_json() {
        let variants = [
            Transport::Tcp(TcpOptions::default()),
            Transport::Ws(WebSocketOptions {
                path: Some("/ws".to_owned()),
                host: Some("cdn.example.com".to_owned()),
                ..Default::default()
            }),
            Transport::Grpc(GrpcOptions {
                service_name: Some("TunService".to_owned()),
                ..Default::default()
            }),
            Transport::Http(HttpOptions {
                host: vec!["a.example.com".to_owned()],
                path: Some("/h".to_owned()),
                method: None,
            }),
            Transport::HttpUpgrade(HttpUpgradeOptions {
                host: Some("u.example.com".to_owned()),
                path: Some("/up".to_owned()),
            }),
            Transport::Xhttp(XhttpOptions {
                host: Some("x.example.com".to_owned()),
                path: Some("/x".to_owned()),
                mode: XhttpMode::PacketUp,
                headers: BTreeMap::new(),
                extra: None,
            }),
        ];

        for transport in variants {
            let json = serde_json::to_string(&transport).expect("serialize");
            let back: Transport = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(transport, back);
        }
    }

    #[test]
    fn uri_names_match_share_link_vocabulary() {
        assert_eq!(Transport::default().uri_name(), "tcp");
        assert_eq!(
            Transport::Xhttp(XhttpOptions::default()).uri_name(),
            "xhttp"
        );
        assert_eq!(
            Transport::HttpUpgrade(HttpUpgradeOptions::default()).uri_name(),
            "httpupgrade"
        );
    }

    #[test]
    fn ws_defaults_path_to_root() {
        let options = WebSocketOptions::default();
        assert_eq!(options.path_or_default(), "/");
    }

    #[test]
    fn xhttp_extra_serializes_only_configured_fields() {
        let options = XhttpOptions {
            mode: XhttpMode::StreamOne,
            extra: Some(XhttpExtra {
                x_padding_bytes: Some("100-1000".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = options.extra_json().expect("extra json");
        assert!(json.contains("xPaddingBytes"));
        assert!(!json.contains("xmux"));
    }

    #[test]
    fn xhttp_extra_is_stored_in_snake_case() {
        // The admin API speaks `snake_case` for every other field, so the
        // XHTTP blob must not leak Xray's `camelCase` naming into the model.
        let extra: XhttpExtra = serde_json::from_value(json!({
            "x_padding_bytes": "100-1000",
            "sc_max_each_post_bytes": "1000000",
            "no_sse_header": true,
            "xmux": { "max_concurrency": "16-32" },
        }))
        .expect("deserialize with snake_case keys");

        assert_eq!(extra.x_padding_bytes.as_deref(), Some("100-1000"));
        assert_eq!(extra.sc_max_each_post_bytes.as_deref(), Some("1000000"));
        assert_eq!(extra.no_sse_header, Some(true));
        assert_eq!(
            extra.xmux.as_ref().and_then(|xmux| xmux.max_concurrency.as_deref()),
            Some("16-32")
        );

        let serialized = serde_json::to_value(&extra).expect("serialize");
        assert!(serialized.get("x_padding_bytes").is_some(), "{serialized}");
        assert!(serialized.get("xPaddingBytes").is_none(), "{serialized}");
    }

    #[test]
    fn xhttp_wire_uses_xray_camel_case() {
        let extra = XhttpExtra {
            x_padding_bytes: Some("100-1000".to_owned()),
            no_sse_header: Some(true),
            sc_max_each_post_bytes: Some("1000000".to_owned()),
            xmux: Some(XmuxOptions {
                max_concurrency: Some("16-32".to_owned()),
                h_keep_alive_period: Some(60),
                ..Default::default()
            }),
            ..Default::default()
        };
        let wire = extra.to_wire();

        assert_eq!(wire["xPaddingBytes"], json!("100-1000"));
        assert_eq!(wire["noSSEHeader"], json!(true));
        assert_eq!(wire["scMaxEachPostBytes"], json!("1000000"));
        assert_eq!(wire["xmux"]["maxConcurrency"], json!("16-32"));
        assert_eq!(wire["xmux"]["hKeepAlivePeriod"], json!(60));
        // Only configured keys are emitted.
        assert!(wire.get("sessionIDKey").is_none(), "{wire:?}");
        assert!(wire["xmux"].get("maxConnections").is_none(), "{wire:?}");
    }

    #[test]
    fn xhttp_extra_accepts_the_full_xray_field_set() {
        // Guards against a future typo silently rejecting a valid field: every
        // name below is read from Xray's SplitHTTPConfig.
        let extra: XhttpExtra = serde_json::from_value(json!({
            "x_padding_bytes": "100-1000",
            "x_padding_obfs_mode": true,
            "x_padding_key": "x_padding",
            "x_padding_header": "X-Padding",
            "x_padding_placement": "queryInHeader",
            "x_padding_method": "repeat-x",
            "uplink_http_method": "POST",
            "session_id_placement": "header",
            "session_id_key": "X-Session",
            "session_id_table": "base62",
            "session_id_length": "10-20",
            "seq_placement": "query",
            "seq_key": "seq",
            "uplink_data_placement": "body",
            "uplink_data_key": "data",
            "uplink_chunk_size": "1024",
            "no_grpc_header": true,
            "no_sse_header": true,
            "sc_max_each_post_bytes": "1000000",
            "sc_min_posts_interval_ms": "30",
            "sc_max_buffered_posts": 100,
            "sc_stream_up_server_secs": "30-60",
            "server_max_header_bytes": 8192,
            "download_settings": {
                "address": "dl.example.com",
                "port": 443,
                "network": "xhttp",
                "security": "tls",
                "tls_settings": { "server_name": "cdn.example.com" },
                "xhttp_settings": { "path": "/dl", "mode": "auto" }
            }
        }))
        .expect("deserialize the full Xray field set");

        let wire = extra.to_wire();
        assert_eq!(wire["xPaddingObfsMode"], json!(true));
        assert_eq!(wire["sessionIDTable"], json!("base62"));
        assert_eq!(wire["scMaxBufferedPosts"], json!(100));
        assert_eq!(wire["serverMaxHeaderBytes"], json!(8192));
        assert_eq!(wire["downloadSettings"]["port"], json!(443));
        assert_eq!(
            wire["downloadSettings"]["tlsSettings"]["serverName"],
            json!("cdn.example.com")
        );
        assert_eq!(
            wire["downloadSettings"]["xhttpSettings"]["path"],
            json!("/dl")
        );
    }

    #[test]
    fn empty_extra_yields_no_json() {
        let options = XhttpOptions {
            extra: Some(XhttpExtra::default()),
            ..Default::default()
        };
        assert!(options.extra_json().is_none());
    }

    #[test]
    fn host_hint_reads_each_variant() {
        let ws = Transport::Ws(WebSocketOptions {
            host: Some("cdn.example.com".to_owned()),
            ..Default::default()
        });
        assert_eq!(ws.host_hint(), Some("cdn.example.com"));
        assert_eq!(Transport::default().host_hint(), None);
    }
}
