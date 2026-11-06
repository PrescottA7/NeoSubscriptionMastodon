// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Mihomo (Clash Meta) YAML rendering.
//!
//! Reference: <https://wiki.metacubex.one/en/config/proxies/>

use serde_json::Map;
use serde_json::Value;

use crate::protocol::{
    AnytlsConfig, Hysteria2Config, ProxyKind, Transport, TrojanConfig, TuicConfig, VlessConfig,
};

use super::context::RenderContext;

/// Renders a node as a bare proxy mapping.
pub fn render(context: &RenderContext) -> anyhow::Result<String> {
    let proxy = proxy(context);
    serde_yaml::to_string(&vec![proxy]).map_err(Into::into)
}

/// Renders every node into a complete mihomo configuration.
pub fn render_collection(contexts: &[RenderContext]) -> anyhow::Result<String> {
    let proxies: Vec<Value> = contexts.iter().map(proxy).collect();
    let names: Vec<String> = contexts.iter().map(|context| context.tag()).collect();

    let mut root = Map::new();
    root.insert("mixed-port".to_owned(), Value::from(7890));
    root.insert("allow-lan".to_owned(), Value::Bool(false));
    root.insert("mode".to_owned(), Value::String("rule".to_owned()));
    root.insert("log-level".to_owned(), Value::String("info".to_owned()));
    root.insert("proxies".to_owned(), Value::Array(proxies));

    let groups = if names.is_empty() {
        Vec::new()
    } else {
        vec![Value::Object({
            let mut group = Map::new();
            group.insert("name".to_owned(), Value::String("PROXY".to_owned()));
            group.insert("type".to_owned(), Value::String("select".to_owned()));
            group.insert(
                "proxies".to_owned(),
                Value::Array(names.into_iter().map(Value::String).collect()),
            );
            group
        })]
    };
    root.insert("proxy-groups".to_owned(), Value::Array(groups));
    root.insert(
        "rules".to_owned(),
        Value::Array(vec![Value::String("MATCH,PROXY".to_owned())]),
    );

    serde_yaml::to_string(&Value::Object(root)).map_err(Into::into)
}

/// Builds a `proxies` entry.
pub fn proxy(context: &RenderContext) -> Value {
    let mut object = match &context.params {
        ProxyKind::Vless(config) => vless(context, config),
        ProxyKind::Trojan(config) => trojan(context, config),
        ProxyKind::Hysteria2(config) => hysteria2(context, config),
        ProxyKind::Tuic(config) => tuic(context, config),
        ProxyKind::Anytls(config) => anytls(context, config),
    };

    object.insert("name".to_owned(), Value::String(context.tag()));
    Value::Object(object)
}

fn base(context: &RenderContext, kind: &str) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("type".to_owned(), Value::String(kind.to_owned()));
    object.insert("server".to_owned(), Value::String(context.server.clone()));
    object.insert("port".to_owned(), Value::from(context.port));
    object
}

fn vless(context: &RenderContext, config: &VlessConfig) -> Map<String, Value> {
    let mut object = base(context, "vless");
    object.insert(
        "uuid".to_owned(),
        Value::String(context.credentials.uuid.clone()),
    );
    object.insert("udp".to_owned(), Value::Bool(true));

    if let Some(flow) = config.effective_flow() {
        object.insert("flow".to_owned(), Value::String(flow.to_owned()));
    }
    if let Some(encoding) = config.packet_encoding_value() {
        object.insert(
            "packet-encoding".to_owned(),
            Value::String(encoding.to_owned()),
        );
    }
    if let Some(encryption) = config
        .encryption
        .as_deref()
        .filter(|value| !value.is_empty() && *value != "none")
    {
        object.insert(
            "encryption".to_owned(),
            Value::String(encryption.to_owned()),
        );
    }

    insert_mihomo_tls(&mut object, &config.tls, &context.server);
    insert_network(&mut object, &config.transport);
    object
}

fn trojan(context: &RenderContext, config: &TrojanConfig) -> Map<String, Value> {
    let mut object = base(context, "trojan");
    object.insert(
        "password".to_owned(),
        Value::String(context.credentials.password.clone()),
    );
    object.insert("udp".to_owned(), Value::Bool(config.udp));

    insert_mihomo_tls(&mut object, &config.tls, &context.server);
    insert_network(&mut object, &config.transport);
    object
}

fn hysteria2(context: &RenderContext, config: &Hysteria2Config) -> Map<String, Value> {
    let mut object = base(context, "hysteria2");
    object.insert(
        "password".to_owned(),
        Value::String(context.credentials.password.clone()),
    );

    if config.has_port_hopping() {
        object.insert(
            "ports".to_owned(),
            Value::String(config.server_ports.join(",")),
        );
        if let Some(interval) = config.hop_interval_value() {
            object.insert(
                "hop-interval".to_owned(),
                Value::String(interval.to_owned()),
            );
        }
    }

    if let Some(up) = config.up_bandwidth() {
        object.insert("up".to_owned(), Value::String(up));
    }
    if let Some(down) = config.down_bandwidth() {
        object.insert("down".to_owned(), Value::String(down));
    }
    if let Some(obfs) = config.obfs_value() {
        object.insert(
            "obfs".to_owned(),
            Value::String(obfs.type_name().to_owned()),
        );
        object.insert(
            "obfs-password".to_owned(),
            Value::String(obfs.password.clone()),
        );
        if let Some(min) = obfs.min_packet_size {
            object.insert("obfs-min-packet-size".to_owned(), Value::from(min));
        }
        if let Some(max) = obfs.max_packet_size {
            object.insert("obfs-max-packet-size".to_owned(), Value::from(max));
        }
    }
    if let Some(profile) = config
        .bbr_profile
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        object.insert("bbr-profile".to_owned(), Value::String(profile.to_owned()));
    }

    insert_mihomo_tls(&mut object, &config.tls, &context.server);
    insert_mihomo_quic(&mut object, &config.quic);
    object
}

fn tuic(context: &RenderContext, config: &TuicConfig) -> Map<String, Value> {
    let mut object = base(context, "tuic");
    object.insert(
        "uuid".to_owned(),
        Value::String(context.credentials.uuid.clone()),
    );
    object.insert(
        "password".to_owned(),
        Value::String(context.credentials.password.clone()),
    );
    object.insert(
        "congestion-controller".to_owned(),
        Value::String(config.congestion_control.as_str().replace('_', "-")),
    );
    object.insert(
        "udp-relay-mode".to_owned(),
        Value::String(config.udp_relay_mode.as_str().to_owned()),
    );

    if config.udp_over_stream {
        object.insert("udp-over-stream".to_owned(), Value::Bool(true));
    }
    if config.zero_rtt_handshake {
        object.insert("reduce-rtt".to_owned(), Value::Bool(true));
    }
    if let Some(heartbeat) = config.heartbeat_value() {
        object.insert(
            "heartbeat-interval".to_owned(),
            Value::String(heartbeat.to_owned()),
        );
    }

    insert_mihomo_tls(&mut object, &config.tls, &context.server);
    insert_mihomo_quic(&mut object, &config.quic);
    object
}

fn anytls(context: &RenderContext, config: &AnytlsConfig) -> Map<String, Value> {
    let mut object = base(context, "anytls");
    object.insert(
        "password".to_owned(),
        Value::String(context.credentials.password.clone()),
    );
    object.insert("udp".to_owned(), Value::Bool(true));

    if let Some(value) = config.idle_check_interval_value() {
        object.insert(
            "idle-session-check-interval".to_owned(),
            Value::from(parse_seconds(value)),
        );
    }
    if let Some(value) = config.idle_session_timeout_value() {
        object.insert(
            "idle-session-timeout".to_owned(),
            Value::from(parse_seconds(value)),
        );
    }
    if let Some(value) = config.min_idle_session {
        object.insert("min-idle-session".to_owned(), Value::from(value));
    }
    if let Some(value) = config.client_metadata.as_deref().filter(|v| !v.is_empty()) {
        object.insert(
            "client-metadata".to_owned(),
            Value::String(value.to_owned()),
        );
    }

    insert_mihomo_tls(&mut object, &config.tls, &context.server);
    object
}

/// Mihomo uses flat, kebab-case TLS keys rather than a nested object.
fn insert_mihomo_tls(
    object: &mut Map<String, Value>,
    tls: &crate::protocol::TlsConfig,
    server: &str,
) {
    if !tls.is_active() {
        object.insert("tls".to_owned(), Value::Bool(false));
        return;
    }

    object.insert("tls".to_owned(), Value::Bool(true));
    object.insert(
        "sni".to_owned(),
        Value::String(tls.effective_server_name(server).to_owned()),
    );

    if tls.insecure {
        object.insert("skip-cert-verify".to_owned(), Value::Bool(true));
    }
    if !tls.alpn.is_empty() {
        object.insert(
            "alpn".to_owned(),
            Value::Array(
                tls.alpn
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
    }
    if tls.enabled {
        object.insert(
            "client-fingerprint".to_owned(),
            Value::String(tls.effective_fingerprint().to_owned()),
        );
    }
    if let Some(hash) = tls.certificate_sha256_value() {
        object.insert("fingerprint".to_owned(), Value::String(hash.to_owned()));
    }
    if let Some(name) = tls
        .verify_peer_cert_by_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        object.insert(
            "name-cert-verify".to_owned(),
            Value::String(name.to_owned()),
        );
    }

    if let Some(reality) = tls.reality.as_ref().filter(|reality| reality.is_usable()) {
        let mut reality_object = Map::new();
        reality_object.insert(
            "public-key".to_owned(),
            Value::String(reality.public_key.clone()),
        );
        if let Some(short_id) = reality.short_id_value() {
            reality_object.insert("short-id".to_owned(), Value::String(short_id.to_owned()));
        }
        object.insert("reality-opts".to_owned(), Value::Object(reality_object));
    }
}

/// Mihomo accepts QUIC window tuning as flat keys, in bytes.
fn insert_mihomo_quic(object: &mut Map<String, Value>, quic: &crate::protocol::QuicConfig) {
    if let Some(value) = quic.initial_stream_receive_window {
        object.insert(
            "initial-stream-receive-window".to_owned(),
            Value::from(value),
        );
    }
    if let Some(value) = quic.max_stream_receive_window {
        object.insert("max-stream-receive-window".to_owned(), Value::from(value));
    }
    if let Some(value) = quic.initial_connection_receive_window {
        object.insert(
            "initial-connection-receive-window".to_owned(),
            Value::from(value),
        );
    }
    if let Some(value) = quic.max_connection_receive_window {
        object.insert(
            "max-connection-receive-window".to_owned(),
            Value::from(value),
        );
    }
}

/// Mihomo flattens the transport into `network` plus per-transport keys.
fn insert_network(object: &mut Map<String, Value>, transport: &Transport) {
    match transport {
        Transport::Tcp(_) => {}
        Transport::Ws(options) => {
            object.insert("network".to_owned(), Value::String("ws".to_owned()));
            object.insert(
                "ws-opts".to_owned(),
                Value::Object({
                    let mut ws = Map::new();
                    ws.insert(
                        "path".to_owned(),
                        Value::String(options.path_or_default().to_owned()),
                    );
                    if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                        ws.insert(
                            "headers".to_owned(),
                            Value::Object({
                                let mut headers = Map::new();
                                headers.insert("Host".to_owned(), Value::String(host.to_owned()));
                                headers
                            }),
                        );
                    }
                    if let Some(early) = options.max_early_data {
                        ws.insert("max-early-data".to_owned(), Value::from(early));
                        ws.insert(
                            "early-data-header-name".to_owned(),
                            Value::String(
                                options
                                    .early_data_header_name
                                    .clone()
                                    .unwrap_or_else(|| "Sec-WebSocket-Protocol".to_owned()),
                            ),
                        );
                    }
                    ws
                }),
            );
        }
        Transport::Grpc(options) => {
            object.insert("network".to_owned(), Value::String("grpc".to_owned()));
            object.insert(
                "grpc-opts".to_owned(),
                Value::Object({
                    let mut grpc = Map::new();
                    grpc.insert(
                        "grpc-service-name".to_owned(),
                        Value::String(options.service_name_or_default().to_owned()),
                    );
                    grpc
                }),
            );
        }
        Transport::Http(options) => {
            object.insert("network".to_owned(), Value::String("http".to_owned()));
            object.insert(
                "http-opts".to_owned(),
                Value::Object({
                    let mut http = Map::new();
                    if !options.host.is_empty() {
                        http.insert(
                            "headers".to_owned(),
                            Value::Object({
                                let mut headers = Map::new();
                                headers.insert(
                                    "Host".to_owned(),
                                    Value::Array(
                                        options
                                            .host
                                            .iter()
                                            .map(|value| Value::String(value.clone()))
                                            .collect(),
                                    ),
                                );
                                headers
                            }),
                        );
                    }
                    http.insert(
                        "path".to_owned(),
                        Value::Array(vec![Value::String(options.path_or_default().to_owned())]),
                    );
                    http
                }),
            );
        }
        Transport::HttpUpgrade(options) => {
            object.insert(
                "network".to_owned(),
                Value::String("httpupgrade".to_owned()),
            );
            object.insert(
                "httpupgrade-opts".to_owned(),
                Value::Object({
                    let mut upgrade = Map::new();
                    upgrade.insert(
                        "path".to_owned(),
                        Value::String(options.path_or_default().to_owned()),
                    );
                    if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                        upgrade.insert("host".to_owned(), Value::String(host.to_owned()));
                    }
                    upgrade
                }),
            );
        }
        Transport::Xhttp(options) => {
            object.insert("network".to_owned(), Value::String("xhttp".to_owned()));
            object.insert(
                "xhttp-opts".to_owned(),
                Value::Object({
                    let mut xhttp = Map::new();
                    xhttp.insert(
                        "path".to_owned(),
                        Value::String(options.path_or_default().to_owned()),
                    );
                    if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                        xhttp.insert("host".to_owned(), Value::String(host.to_owned()));
                    }
                    if options.mode != crate::protocol::shared::transport::XhttpMode::Auto {
                        xhttp.insert(
                            "mode".to_owned(),
                            Value::String(options.mode.as_str().to_owned()),
                        );
                    }
                    if !options.headers.is_empty() {
                        xhttp.insert(
                            "headers".to_owned(),
                            serde_json::to_value(&options.headers)
                                .unwrap_or(Value::Object(Map::new())),
                        );
                    }
                    if let Some(extra) = options.extra_json() {
                        xhttp.insert("extra".to_owned(), Value::String(extra));
                    }
                    xhttp
                }),
            );
        }
    }
}

/// Converts `"30s"` to `30` for Mihomo's integer interval fields.
fn parse_seconds(value: &str) -> i64 {
    let trimmed = value.trim();
    if let Some(number) = trimmed.strip_suffix("ms") {
        return number
            .parse::<i64>()
            .map(|ms| ms / 1000)
            .unwrap_or_default();
    }
    trimmed
        .trim_end_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::shared::transport::{GrpcOptions, WebSocketOptions, XhttpOptions};
    use crate::protocol::{RealityConfig, TlsConfig};
    use crate::render::context::{RenderCredentials, RenderUser};

    fn context(params: ProxyKind) -> RenderContext {
        RenderContext::new(
            RenderUser {
                id: 1,
                remark: String::new(),
            },
            RenderCredentials::new("uuid-1", "pw", "alice@tokyo"),
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
    fn vless_proxy_uses_kebab_case_tls_keys() {
        let proxy = proxy(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            ..Default::default()
        })));

        assert_eq!(proxy["type"], "vless");
        assert_eq!(proxy["tls"], true);
        assert_eq!(proxy["sni"], "cdn.example.com");
        assert_eq!(proxy["client-fingerprint"], "chrome");
        assert_eq!(proxy["name"], "tokyo-1");
    }

    #[test]
    fn reality_options_are_nested() {
        let proxy = proxy(&context(ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: false,
                reality: Some(RealityConfig {
                    enabled: true,
                    public_key: "pbk".to_owned(),
                    short_id: Some("1234".to_owned()),
                    spider_x: None,
                    mldsa65_verify: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        })));

        assert_eq!(proxy["reality-opts"]["public-key"], "pbk");
        assert_eq!(proxy["reality-opts"]["short-id"], "1234");
    }

    #[test]
    fn ws_and_grpc_options_are_rendered() {
        let ws = proxy(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Ws(WebSocketOptions {
                path: Some("/ws".to_owned()),
                host: Some("ws.example.com".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })));
        assert_eq!(ws["network"], "ws");
        assert_eq!(ws["ws-opts"]["path"], "/ws");
        assert_eq!(ws["ws-opts"]["headers"]["Host"], "ws.example.com");

        let grpc = proxy(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Grpc(GrpcOptions {
                service_name: Some("Svc".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })));
        assert_eq!(grpc["network"], "grpc");
        assert_eq!(grpc["grpc-opts"]["grpc-service-name"], "Svc");
    }

    #[test]
    fn xhttp_options_are_rendered() {
        let proxy = proxy(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Xhttp(XhttpOptions {
                host: Some("cdn.example.com".to_owned()),
                path: Some("/x".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        })));

        assert_eq!(proxy["network"], "xhttp");
        assert_eq!(proxy["xhttp-opts"]["path"], "/x");
        assert_eq!(proxy["xhttp-opts"]["host"], "cdn.example.com");
    }

    #[test]
    fn hysteria2_uses_bandwidth_strings_and_obfs_keys() {
        let proxy = proxy(&context(ProxyKind::Hysteria2(Hysteria2Config {
            up_mbps: Some(30),
            down_mbps: Some(200),
            obfs: Some(crate::protocol::ObfsConfig {
                obfs_type: crate::protocol::ObfsType::Gecko,
                password: "obfs-pw".to_owned(),
                min_packet_size: Some(512),
                max_packet_size: Some(1200),
            }),
            server_ports: vec!["2080:3000".to_owned()],
            ..Default::default()
        })));

        assert_eq!(proxy["up"], "30 Mbps");
        assert_eq!(proxy["down"], "200 Mbps");
        assert_eq!(proxy["obfs"], "gecko");
        assert_eq!(proxy["obfs-password"], "obfs-pw");
        assert_eq!(proxy["obfs-min-packet-size"], 512);
        assert_eq!(proxy["ports"], "2080:3000");
    }

    #[test]
    fn tuic_uses_congestion_controller_key() {
        let proxy = proxy(&context(ProxyKind::Tuic(TuicConfig {
            congestion_control: crate::protocol::CongestionControl::NewReno,
            udp_over_stream: true,
            ..Default::default()
        })));

        assert_eq!(proxy["congestion-controller"], "new-reno");
        assert_eq!(proxy["udp-over-stream"], true);
        assert_eq!(proxy["udp-relay-mode"], "native");
    }

    #[test]
    fn anytls_intervals_become_seconds() {
        let proxy = proxy(&context(ProxyKind::Anytls(AnytlsConfig {
            idle_session_timeout: Some("60s".to_owned()),
            idle_session_check_interval: Some("1500ms".to_owned()),
            min_idle_session: Some(3),
            ..Default::default()
        })));

        assert_eq!(proxy["idle-session-timeout"], 60);
        assert_eq!(proxy["idle-session-check-interval"], 1);
        assert_eq!(proxy["min-idle-session"], 3);
    }

    #[test]
    fn collection_is_a_complete_mihomo_config() {
        let contexts = vec![context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            ..Default::default()
        }))];

        let rendered = render_collection(&contexts).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["mixed-port"], 7890);
        assert_eq!(value["proxies"][0]["type"], "vless");
        assert_eq!(value["proxy-groups"][0]["name"], "PROXY");
        assert_eq!(value["rules"][0], "MATCH,PROXY");
    }

    #[test]
    fn empty_collection_has_no_proxy_group() {
        let rendered = render_collection(&[]).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");
        assert_eq!(value["proxy-groups"].as_array().expect("groups").len(), 0);
    }

    #[test]
    fn parse_seconds_handles_units() {
        assert_eq!(parse_seconds("30s"), 30);
        assert_eq!(parse_seconds("5m"), 5);
        assert_eq!(parse_seconds("1500ms"), 1);
        assert_eq!(parse_seconds("bogus"), 0);
    }
}
