//! sing-box JSON rendering.
//!
//! Reference: <https://sing-box.sagernet.org/configuration/outbound/>

use serde_json::{json, Map, Value};

use crate::protocol::{
    AnytlsConfig, Hysteria2Config, ProxyKind, QuicConfig, TlsConfig, Transport, TrojanConfig,
    TuicConfig, VlessConfig,
};

use super::context::RenderContext;

/// Renders a node as a bare outbound object.
pub fn render(context: &RenderContext) -> anyhow::Result<String> {
    let outbound = outbound(context)?;
    Ok(serde_json::to_string_pretty(&outbound)?)
}

/// Renders every node into a complete, runnable sing-box configuration.
pub fn render_collection(contexts: &[RenderContext]) -> anyhow::Result<String> {
    let outbounds = build_outbounds(contexts)?;
    let config = json!({
        "log": { "level": "info" },
        "dns": {
            "servers": [
                { "tag": "google", "address": "tls://8.8.8.8" },
                { "tag": "local", "address": "223.5.5.5", "detour": "direct" }
            ],
            "rules": [ { "outbound": "any", "server": "local" } ]
        },
        "inbounds": [
            {
                "type": "mixed",
                "tag": "mixed-in",
                "listen": "127.0.0.1",
                "listen_port": 2080
            }
        ],
        "outbounds": outbounds,
        "route": { "final": "direct" }
    });

    Ok(serde_json::to_string_pretty(&config)?)
}

/// Builds outbounds for a collection, appending the mandatory `direct` entry.
fn build_outbounds(contexts: &[RenderContext]) -> anyhow::Result<Vec<Value>> {
    let mut outbounds: Vec<Value> = contexts
        .iter()
        .map(outbound)
        .collect::<anyhow::Result<_>>()?;

    if outbounds.is_empty() {
        outbounds.push(json!({ "type": "direct", "tag": "direct" }));
        return Ok(outbounds);
    }

    outbounds.push(json!({ "type": "direct", "tag": "direct" }));

    let tags: Vec<String> = contexts.iter().map(|context| context.tag()).collect();
    outbounds.push(json!({
        "type": "selector",
        "tag": "proxy",
        "outbounds": tags,
        "default": tags.first(),
    }));

    Ok(outbounds)
}

/// Builds one outbound object.
pub fn outbound(context: &RenderContext) -> anyhow::Result<Value> {
    let mut object = match &context.params {
        ProxyKind::Vless(config) => vless(context, config),
        ProxyKind::Trojan(config) => trojan(context, config),
        ProxyKind::Hysteria2(config) => hysteria2(context, config),
        ProxyKind::Tuic(config) => tuic(context, config),
        ProxyKind::Anytls(config) => anytls(context, config),
    };

    object.insert("tag".to_owned(), json!(context.tag()));
    Ok(Value::Object(object))
}

fn base(context: &RenderContext, protocol: &str) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("type".to_owned(), json!(protocol));
    object.insert("server".to_owned(), json!(context.server));
    object.insert("server_port".to_owned(), json!(context.port));
    object
}

fn vless(context: &RenderContext, config: &VlessConfig) -> Map<String, Value> {
    let mut object = base(context, "vless");
    object.insert("uuid".to_owned(), json!(context.credentials.uuid));

    if let Some(flow) = config.effective_flow() {
        object.insert("flow".to_owned(), json!(flow));
    }
    if let Some(encoding) = config.packet_encoding_value() {
        object.insert("packet_encoding".to_owned(), json!(encoding));
    }
    if let Some(encryption) = config
        .encryption
        .as_deref()
        .filter(|value| !value.is_empty() && *value != "none")
    {
        object.insert("encryption".to_owned(), json!(encryption));
    }

    insert_tls(&mut object, &config.tls, &context.server);
    insert_transport(&mut object, &config.transport);
    insert_mux(&mut object, &config.mux);
    object
}

fn trojan(context: &RenderContext, config: &TrojanConfig) -> Map<String, Value> {
    let mut object = base(context, "trojan");
    object.insert("password".to_owned(), json!(context.credentials.password));

    insert_tls(&mut object, &config.tls, &context.server);
    insert_transport(&mut object, &config.transport);
    insert_mux(&mut object, &config.mux);
    object
}

fn hysteria2(context: &RenderContext, config: &Hysteria2Config) -> Map<String, Value> {
    let mut object = base(context, "hysteria2");
    object.insert("password".to_owned(), json!(context.credentials.password));

    if config.has_port_hopping() {
        object.insert("server_ports".to_owned(), json!(config.server_ports));
        object.insert(
            "hop_interval".to_owned(),
            json!(config.hop_interval_value().unwrap_or("30s")),
        );
        if let Some(value) = config.hop_interval_max.as_deref().filter(|v| !v.is_empty()) {
            object.insert("hop_interval_max".to_owned(), json!(value));
        }
    }

    if let Some(up) = config.up_mbps {
        object.insert("up_mbps".to_owned(), json!(up));
    }
    if let Some(down) = config.down_mbps {
        object.insert("down_mbps".to_owned(), json!(down));
    }
    if let Some(obfs) = config.obfs_value() {
        let mut obfs_object = Map::new();
        obfs_object.insert("type".to_owned(), json!(obfs.type_name()));
        obfs_object.insert("password".to_owned(), json!(obfs.password));
        if let Some(min) = obfs.min_packet_size {
            obfs_object.insert("min_packet_size".to_owned(), json!(min));
        }
        if let Some(max) = obfs.max_packet_size {
            obfs_object.insert("max_packet_size".to_owned(), json!(max));
        }
        object.insert("obfs".to_owned(), Value::Object(obfs_object));
    }
    if let Some(profile) = config.bbr_profile.as_deref().filter(|v| !v.is_empty()) {
        object.insert("bbr_profile".to_owned(), json!(profile));
    }
    if config.brutal_debug {
        object.insert("brutal_debug".to_owned(), json!(true));
    }
    if config.disable_chrome_parrot {
        object.insert("disable_chrome_parrot".to_owned(), json!(true));
    }
    if let Some(realm) = config.realm.as_ref() {
        let mut realm_object = Map::new();
        realm_object.insert("server_url".to_owned(), json!(realm.server_url));
        if let Some(token) = realm.token.as_deref().filter(|v| !v.is_empty()) {
            realm_object.insert("token".to_owned(), json!(token));
        }
        realm_object.insert("realm_id".to_owned(), json!(realm.realm_id));
        if !realm.stun_servers.is_empty() {
            realm_object.insert("stun_servers".to_owned(), json!(realm.stun_servers));
        }
        if let Some(version) = realm.ip_version {
            realm_object.insert("ip_version".to_owned(), json!(version));
        }
        if let Some(mapping) = realm.port_mapping.as_ref() {
            let mut mapping_object = Map::new();
            mapping_object.insert("enabled".to_owned(), json!(mapping.enabled));
            if let Some(timeout) = mapping.timeout.as_deref().filter(|v| !v.is_empty()) {
                mapping_object.insert("timeout".to_owned(), json!(timeout));
            }
            if let Some(lifetime) = mapping.lifetime.as_deref().filter(|v| !v.is_empty()) {
                mapping_object.insert("lifetime".to_owned(), json!(lifetime));
            }
            realm_object.insert("port_mapping".to_owned(), Value::Object(mapping_object));
        }
        object.insert("realm".to_owned(), Value::Object(realm_object));
    }

    insert_tls(&mut object, &config.tls, &context.server);
    insert_quic(&mut object, &config.quic);
    object
}

fn tuic(context: &RenderContext, config: &TuicConfig) -> Map<String, Value> {
    let mut object = base(context, "tuic");
    object.insert("uuid".to_owned(), json!(context.credentials.uuid));
    object.insert("password".to_owned(), json!(context.credentials.password));
    object.insert(
        "congestion_control".to_owned(),
        json!(config.congestion_control.as_str()),
    );
    object.insert(
        "udp_relay_mode".to_owned(),
        json!(config.udp_relay_mode.as_str()),
    );

    if config.udp_over_stream {
        object.insert("udp_over_stream".to_owned(), json!(true));
    }
    if config.zero_rtt_handshake {
        object.insert("zero_rtt_handshake".to_owned(), json!(true));
    }
    if let Some(heartbeat) = config.heartbeat_value() {
        object.insert("heartbeat".to_owned(), json!(heartbeat));
    }

    insert_tls(&mut object, &config.tls, &context.server);
    insert_quic(&mut object, &config.quic);
    object
}

fn anytls(context: &RenderContext, config: &AnytlsConfig) -> Map<String, Value> {
    let mut object = base(context, "anytls");
    object.insert("password".to_owned(), json!(context.credentials.password));

    if let Some(value) = config.idle_check_interval_value() {
        object.insert("idle_session_check_interval".to_owned(), json!(value));
    }
    if let Some(value) = config.idle_session_timeout_value() {
        object.insert("idle_session_timeout".to_owned(), json!(value));
    }
    if let Some(value) = config.min_idle_session {
        object.insert("min_idle_session".to_owned(), json!(value));
    }
    if let Some(value) = config.client_metadata.as_deref().filter(|v| !v.is_empty()) {
        object.insert("client_metadata".to_owned(), json!(value));
    }

    insert_tls(&mut object, &config.tls, &context.server);
    object
}

/// Emits the `tls` object, including REALITY and uTLS settings.
fn insert_tls(object: &mut Map<String, Value>, tls: &TlsConfig, server: &str) {
    if !tls.is_active() {
        return;
    }

    let mut tls_object = Map::new();
    tls_object.insert("enabled".to_owned(), json!(true));
    tls_object.insert(
        "server_name".to_owned(),
        json!(tls.effective_server_name(server)),
    );

    if tls.insecure {
        tls_object.insert("insecure".to_owned(), json!(true));
    }
    if !tls.alpn.is_empty() {
        tls_object.insert("alpn".to_owned(), json!(tls.alpn));
    }
    if tls.enabled {
        tls_object.insert(
            "utls".to_owned(),
            json!({
                "enabled": true,
                "fingerprint": tls.effective_fingerprint(),
            }),
        );
    }
    if let Some(hash) = tls.certificate_sha256_value() {
        let hashes: Vec<&str> = hash
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .collect();
        tls_object.insert("certificate_sha256".to_owned(), json!(hashes));
    }
    if let Some(name) = tls
        .verify_peer_cert_by_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        tls_object.insert("certificate_public_key_sha256".to_owned(), json!(name));
    }
    if let Some(version) = tls.min_version.as_deref().filter(|value| !value.is_empty()) {
        tls_object.insert("min_version".to_owned(), json!(version));
    }
    if let Some(version) = tls.max_version.as_deref().filter(|value| !value.is_empty()) {
        tls_object.insert("max_version".to_owned(), json!(version));
    }

    if let Some(reality) = tls.reality.as_ref().filter(|reality| reality.is_usable()) {
        let mut reality_object = Map::new();
        reality_object.insert("enabled".to_owned(), json!(true));
        reality_object.insert("public_key".to_owned(), json!(reality.public_key));
        if let Some(short_id) = reality.short_id_value() {
            reality_object.insert("short_id".to_owned(), json!(short_id));
        }
        if let Some(spider) = reality.spider_x_value() {
            reality_object.insert("spider_x".to_owned(), json!(spider));
        }
        if let Some(mldsa) = reality.mldsa65_verify_value() {
            reality_object.insert("mldsa65_verify".to_owned(), json!(mldsa));
        }
        tls_object.insert("reality".to_owned(), Value::Object(reality_object));
    }

    object.insert("tls".to_owned(), Value::Object(tls_object));
}

/// Emits the `transport` object for V2Ray-style stream transports.
fn insert_transport(object: &mut Map<String, Value>, transport: &Transport) {
    match transport {
        Transport::Tcp(_) => {}
        Transport::Ws(options) => {
            let mut ws = Map::new();
            ws.insert("type".to_owned(), json!("ws"));
            ws.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                ws.insert("host".to_owned(), json!(host));
            }
            if !options.headers.is_empty() {
                ws.insert("headers".to_owned(), json!(options.headers));
            }
            if let Some(early) = options.max_early_data {
                ws.insert("max_early_data".to_owned(), json!(early));
                ws.insert(
                    "early_data_header_name".to_owned(),
                    json!(
                        options
                            .early_data_header_name
                            .as_deref()
                            .unwrap_or("Sec-WebSocket-Protocol")
                    ),
                );
            }
            object.insert("transport".to_owned(), Value::Object(ws));
        }
        Transport::Grpc(options) => {
            let mut grpc = Map::new();
            grpc.insert("type".to_owned(), json!("grpc"));
            grpc.insert(
                "service_name".to_owned(),
                json!(options.service_name_or_default()),
            );
            object.insert("transport".to_owned(), Value::Object(grpc));
        }
        Transport::Http(options) => {
            let mut http = Map::new();
            http.insert("type".to_owned(), json!("http"));
            if !options.host.is_empty() {
                http.insert("host".to_owned(), json!(options.host));
            }
            http.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(method) = options.method.as_deref().filter(|value| !value.is_empty()) {
                http.insert("method".to_owned(), json!(method));
            }
            object.insert("transport".to_owned(), Value::Object(http));
        }
        Transport::HttpUpgrade(options) => {
            let mut upgrade = Map::new();
            upgrade.insert("type".to_owned(), json!("httpupgrade"));
            upgrade.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                upgrade.insert("host".to_owned(), json!(host));
            }
            object.insert("transport".to_owned(), Value::Object(upgrade));
        }
        Transport::Xhttp(options) => {
            let mut xhttp = Map::new();
            xhttp.insert("type".to_owned(), json!("xhttp"));
            xhttp.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                xhttp.insert("host".to_owned(), json!(host));
            }
            if options.mode != crate::protocol::shared::transport::XhttpMode::Auto {
                xhttp.insert("mode".to_owned(), json!(options.mode.as_str()));
            }
            if !options.headers.is_empty() {
                xhttp.insert("headers".to_owned(), json!(options.headers));
            }
            object.insert("transport".to_owned(), Value::Object(xhttp));
        }
    }
}

/// Emits the `multiplex` object.
fn insert_mux(object: &mut Map<String, Value>, mux: &crate::protocol::MuxConfig) {
    if !mux.is_active() {
        return;
    }

    let mut multiplex = Map::new();
    multiplex.insert("enabled".to_owned(), json!(true));
    multiplex.insert("protocol".to_owned(), json!(mux.protocol.as_str()));
    if let Some(max_streams) = mux.max_streams {
        multiplex.insert("max_streams".to_owned(), json!(max_streams));
    }
    if mux.padding {
        multiplex.insert("padding".to_owned(), json!(true));
    }
    if let Some(brutal) = mux.brutal.as_ref() {
        multiplex.insert("brutal".to_owned(), json!({
            "enabled": brutal.enabled,
            "up_mbps": brutal.up_mbps,
            "down_mbps": brutal.down_mbps,
        }));
    }

    object.insert("multiplex".to_owned(), Value::Object(multiplex));
}

/// Emits the QUIC window overrides sing-box accepts.
fn insert_quic(object: &mut Map<String, Value>, quic: &QuicConfig) {
    if let Some(value) = quic.initial_stream_receive_window {
        object.insert("initial_stream_receive_window".to_owned(), json!(value));
    }
    if let Some(value) = quic.max_stream_receive_window {
        object.insert("max_stream_receive_window".to_owned(), json!(value));
    }
    if let Some(value) = quic.initial_connection_receive_window {
        object.insert("initial_connection_receive_window".to_owned(), json!(value));
    }
    if let Some(value) = quic.max_connection_receive_window {
        object.insert("max_connection_receive_window".to_owned(), json!(value));
    }
    if let Some(value) = quic.keep_alive_value() {
        object.insert("keep_alive_period".to_owned(), json!(value));
    }
    if quic.disable_path_mtu_discovery {
        object.insert("disable_path_mtu_discovery".to_owned(), json!(true));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::shared::transport::{
        GrpcOptions, WebSocketOptions, XhttpExtra, XhttpMode, XhttpOptions,
    };
    use crate::protocol::{RealityConfig, MuxConfig};
    use crate::render::context::{RenderCredentials, RenderUser};

    fn context(params: ProxyKind) -> RenderContext {
        RenderContext::new(
            RenderUser {
                id: 1,
                remark: String::new(),
            },
            RenderCredentials::new(
                "b0dd64e4-0fbd-4038-9139-d1f32a68a0dc",
                "pw",
                "alice@tokyo",
            ),
            params,
        )
        .with_node("example.com", 443, "Tokyo")
    }

    fn tls() -> TlsConfig {
        TlsConfig {
            enabled: true,
            server_name: Some("cdn.example.com".to_owned()),
            alpn: vec!["h2".to_owned()],
            ..Default::default()
        }
    }

    #[test]
    fn vless_outbound_has_expected_fields() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            ..Default::default()
        })))
        .expect("render");

        assert_eq!(outbound["type"], "vless");
        assert_eq!(outbound["server"], "example.com");
        assert_eq!(outbound["server_port"], 443);
        assert_eq!(outbound["uuid"], "b0dd64e4-0fbd-4038-9139-d1f32a68a0dc");
        assert_eq!(outbound["tls"]["server_name"], "cdn.example.com");
        assert_eq!(outbound["tls"]["utls"]["fingerprint"], "chrome");
        assert_eq!(outbound["tag"], "tokyo-1");
    }

    #[test]
    fn reality_settings_are_nested_under_tls() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            flow: Some("xtls-rprx-vision".to_owned()),
            tls: TlsConfig {
                enabled: false,
                reality: Some(RealityConfig {
                    enabled: true,
                    public_key: "pbk".to_owned(),
                    short_id: Some("abcd".to_owned()),
                    spider_x: Some("/spx".to_owned()),
                    mldsa65_verify: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        })))
        .expect("render");

        assert_eq!(outbound["tls"]["reality"]["public_key"], "pbk");
        assert_eq!(outbound["tls"]["reality"]["short_id"], "abcd");
        assert_eq!(outbound["tls"]["reality"]["spider_x"], "/spx");
        assert_eq!(outbound["flow"], "xtls-rprx-vision");
        assert!(outbound["tls"].get("utls").is_none());
    }

    #[test]
    fn ws_and_grpc_transports_are_rendered() {
        let ws = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Ws(WebSocketOptions {
                path: Some("/ws".to_owned()),
                host: Some("ws.example.com".to_owned()),
                max_early_data: Some(2048),
                early_data_header_name: None,
                ..Default::default()
            }),
            ..Default::default()
        })))
        .expect("render");
        assert_eq!(ws["transport"]["type"], "ws");
        assert_eq!(ws["transport"]["path"], "/ws");
        assert_eq!(ws["transport"]["max_early_data"], 2048);
        assert_eq!(
            ws["transport"]["early_data_header_name"],
            "Sec-WebSocket-Protocol"
        );

        let grpc = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Grpc(GrpcOptions {
                service_name: Some("TunService".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })))
        .expect("render");
        assert_eq!(grpc["transport"]["type"], "grpc");
        assert_eq!(grpc["transport"]["service_name"], "TunService");
    }

    #[test]
    fn xhttp_mode_is_omitted_when_auto() {
        let auto = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Xhttp(XhttpOptions::default()),
            ..Default::default()
        })))
        .expect("render");
        assert!(auto["transport"].get("mode").is_none());

        let packet_up = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Xhttp(XhttpOptions {
                mode: XhttpMode::PacketUp,
                extra: Some(XhttpExtra {
                    x_padding_bytes: Some("100-1000".to_owned()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        })))
        .expect("render");
        assert_eq!(packet_up["transport"]["mode"], "packet-up");
    }

    #[test]
    fn hysteria2_renders_hopping_and_obfs() {
        let outbound = outbound(&context(ProxyKind::Hysteria2(Hysteria2Config {
            server_ports: vec!["2080:3000".to_owned()],
            hop_interval: Some("15s".to_owned()),
            up_mbps: Some(30),
            obfs: Some(crate::protocol::ObfsConfig {
                password: "obfs".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        })))
        .expect("render");

        assert_eq!(outbound["server_ports"][0], "2080:3000");
        assert_eq!(outbound["hop_interval"], "15s");
        assert_eq!(outbound["up_mbps"], 30);
        assert_eq!(outbound["obfs"]["type"], "salamander");
        assert_eq!(outbound["obfs"]["password"], "obfs");
        assert_eq!(outbound["tls"]["enabled"], true);
    }

    #[test]
    fn tuic_renders_relay_settings() {
        let outbound = outbound(&context(ProxyKind::Tuic(TuicConfig {
            congestion_control: crate::protocol::CongestionControl::Bbr,
            udp_relay_mode: crate::protocol::UdpRelayMode::Quic,
            zero_rtt_handshake: true,
            ..Default::default()
        })))
        .expect("render");

        assert_eq!(outbound["uuid"], "b0dd64e4-0fbd-4038-9139-d1f32a68a0dc");
        assert_eq!(outbound["congestion_control"], "bbr");
        assert_eq!(outbound["udp_relay_mode"], "quic");
        assert_eq!(outbound["zero_rtt_handshake"], true);
    }

    #[test]
    fn anytls_renders_session_tuning() {
        let outbound = outbound(&context(ProxyKind::Anytls(AnytlsConfig {
            idle_session_timeout: Some("60s".to_owned()),
            min_idle_session: Some(2),
            ..Default::default()
        })))
        .expect("render");

        assert_eq!(outbound["idle_session_timeout"], "60s");
        assert_eq!(outbound["min_idle_session"], 2);
    }

    #[test]
    fn mux_is_rendered_only_when_enabled() {
        let disabled = outbound(&context(ProxyKind::Trojan(TrojanConfig {
            tls: tls(),
            ..Default::default()
        })))
        .expect("render");
        assert!(disabled.get("multiplex").is_none());

        let enabled = outbound(&context(ProxyKind::Trojan(TrojanConfig {
            tls: tls(),
            mux: MuxConfig {
                enabled: true,
                max_streams: Some(8),
                padding: true,
                ..Default::default()
            },
            ..Default::default()
        })))
        .expect("render");
        assert_eq!(enabled["multiplex"]["enabled"], true);
        assert_eq!(enabled["multiplex"]["max_streams"], 8);
        assert_eq!(enabled["multiplex"]["padding"], true);
    }

    #[test]
    fn collection_is_a_complete_config() {
        let contexts = vec![
            context(ProxyKind::Vless(VlessConfig {
                tls: tls(),
                ..Default::default()
            })),
            context(ProxyKind::Anytls(AnytlsConfig::default())).with_index(1),
        ];

        let rendered = render_collection(&contexts).expect("render");
        let value: Value = serde_json::from_str(&rendered).expect("valid json");

        let tags: Vec<&str> = value["outbounds"]
            .as_array()
            .expect("outbounds")
            .iter()
            .map(|outbound| outbound["tag"].as_str().expect("tag"))
            .collect();

        assert!(tags.contains(&"tokyo-1"));
        assert!(tags.contains(&"tokyo-2"));
        assert!(tags.contains(&"direct"));
        assert!(tags.contains(&"proxy"));
        assert_eq!(value["inbounds"][0]["type"], "mixed");
    }

    #[test]
    fn empty_collection_still_produces_a_valid_config() {
        let rendered = render_collection(&[]).expect("render");
        let value: Value = serde_json::from_str(&rendered).expect("valid json");
        assert_eq!(value["outbounds"].as_array().expect("outbounds").len(), 1);
    }
}
