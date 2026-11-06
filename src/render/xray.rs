// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Xray JSON rendering.
//!
//! Reference: <https://xtls.github.io/en/config/outbounds/vless.html> and
//! <https://xtls.github.io/en/config/transports/xhttp.html>

use serde_json::{json, Map, Value};

use crate::protocol::shared::transport::TcpHeaderType;
use crate::protocol::{ProxyKind, Transport, TrojanConfig, VlessConfig};

use super::context::RenderContext;

/// Renders a node as a bare outbound object.
pub fn render(context: &RenderContext) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(&outbound(context))?)
}

/// Renders every node into a complete, runnable Xray configuration.
pub fn render_collection(contexts: &[RenderContext]) -> anyhow::Result<String> {
    let outbounds: Vec<Value> = contexts.iter().map(outbound).collect();

    let config = json!({
        "log": { "loglevel": "warning" },
        "inbounds": [
            {
                "listen": "127.0.0.1",
                "port": 2080,
                "protocol": "socks",
                "settings": { "udp": true, "auth": "noauth" },
                "tag": "socks-in"
            },
            {
                "listen": "127.0.0.1",
                "port": 2081,
                "protocol": "http",
                "tag": "http-in"
            }
        ],
        "outbounds": outbounds,
    });

    Ok(serde_json::to_string_pretty(&config)?)
}

/// Builds one Xray outbound object.
pub fn outbound(context: &RenderContext) -> Value {
    let mut object = match &context.params {
        ProxyKind::Vless(config) => vless(context, config),
        ProxyKind::Trojan(config) => trojan(context, config),
        // Xray has no Hysteria2, TUIC, or AnyTLS outbound; produce a placeholder
        // so the collection stays structurally valid.
        ProxyKind::Hysteria2(_) | ProxyKind::Tuic(_) | ProxyKind::Anytls(_) => {
            json!({
                "protocol": "blackhole",
                "tag": context.tag(),
                "settings": {},
            })
        }
    };

    if let Value::Object(ref mut map) = object {
        map.insert("tag".to_owned(), json!(context.tag()));
    }
    object
}

fn vless(context: &RenderContext, config: &VlessConfig) -> Value {
    let mut settings = Map::new();
    settings.insert("address".to_owned(), json!(context.server));
    settings.insert("port".to_owned(), json!(context.port));
    settings.insert("id".to_owned(), json!(context.credentials.uuid));
    settings.insert("encryption".to_owned(), json!(config.encryption_value()));
    if let Some(flow) = config.effective_flow() {
        settings.insert("flow".to_owned(), json!(flow));
    }

    let mut object = Map::new();
    object.insert("protocol".to_owned(), json!("vless"));
    object.insert("settings".to_owned(), Value::Object(settings));
    object.insert(
        "streamSettings".to_owned(),
        stream_settings(&config.transport, &config.tls, &context.server),
    );
    Value::Object(object)
}

fn trojan(context: &RenderContext, config: &TrojanConfig) -> Value {
    let settings = json!({
        "servers": [
            {
                "address": context.server,
                "port": context.port,
                "password": context.credentials.password,
            }
        ]
    });

    let mut object = Map::new();
    object.insert("protocol".to_owned(), json!("trojan"));
    object.insert("settings".to_owned(), settings);
    object.insert(
        "streamSettings".to_owned(),
        stream_settings(&config.transport, &config.tls, &context.server),
    );
    Value::Object(object)
}

/// Builds Xray's `streamSettings`, which nests transport, TLS, and mux.
fn stream_settings(
    transport: &Transport,
    tls: &crate::protocol::TlsConfig,
    server: &str,
) -> Value {
    let mut settings = Map::new();
    settings.insert("network".to_owned(), json!(transport.uri_name()));

    if let Some(value) = transport_settings(transport) {
        settings.insert(format!("{}Settings", transport.uri_name()), value);
    }

    if tls.reality_enabled() {
        settings.insert("security".to_owned(), json!("reality"));
        settings.insert("realitySettings".to_owned(), reality_settings(tls, server));
    } else if tls.enabled {
        settings.insert("security".to_owned(), json!("tls"));
        settings.insert("tlsSettings".to_owned(), tls_settings(tls, server));
    } else {
        settings.insert("security".to_owned(), json!("none"));
    }

    Value::Object(settings)
}

/// Xray stores the TCP camouflage header inside `tcpSettings`.
fn tcp_settings(header: TcpHeaderType) -> Value {
    match header {
        TcpHeaderType::None => json!({ "header": { "type": "none" } }),
        TcpHeaderType::Http => json!({
            "header": {
                "type": "http",
                "request": {
                    "path": ["/"],
                    "headers": { "Host": [] }
                }
            }
        }),
    }
}

fn transport_settings(transport: &Transport) -> Option<Value> {
    match transport {
        Transport::Tcp(options) => Some(tcp_settings(options.header_type)),
        Transport::Ws(options) => {
            let mut ws = Map::new();
            ws.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                ws.insert("host".to_owned(), json!(host));
            }
            if !options.headers.is_empty() {
                ws.insert("headers".to_owned(), json!(options.headers));
            }
            if let Some(early) = options.max_early_data {
                ws.insert("maxEarlyData".to_owned(), json!(early));
                ws.insert(
                    "earlyDataHeaderName".to_owned(),
                    json!(
                        options
                            .early_data_header_name
                            .as_deref()
                            .unwrap_or("Sec-WebSocket-Protocol")
                    ),
                );
            }
            Some(Value::Object(ws))
        }
        Transport::Grpc(options) => {
            let mut grpc = Map::new();
            grpc.insert(
                "serviceName".to_owned(),
                json!(options.service_name_or_default()),
            );
            if let Some(mode) = options.mode.as_deref().filter(|value| !value.is_empty()) {
                grpc.insert("multiMode".to_owned(), json!(mode == "multi"));
            }
            if let Some(authority) = options.authority.as_deref().filter(|value| !value.is_empty()) {
                grpc.insert("authority".to_owned(), json!(authority));
            }
            Some(Value::Object(grpc))
        }
        Transport::Http(options) => {
            let mut http = Map::new();
            if !options.host.is_empty() {
                http.insert("host".to_owned(), json!(options.host));
            }
            http.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(method) = options.method.as_deref().filter(|value| !value.is_empty()) {
                http.insert("method".to_owned(), json!(method));
            }
            Some(Value::Object(http))
        }
        Transport::HttpUpgrade(options) => {
            let mut upgrade = Map::new();
            upgrade.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                upgrade.insert("host".to_owned(), json!(host));
            }
            Some(Value::Object(upgrade))
        }
        Transport::Xhttp(options) => {
            let mut xhttp = Map::new();
            xhttp.insert("path".to_owned(), json!(options.path_or_default()));
            if let Some(host) = options.host.as_deref().filter(|value| !value.is_empty()) {
                xhttp.insert("host".to_owned(), json!(host));
            }
            xhttp.insert("mode".to_owned(), json!(options.mode.as_str()));
            if !options.headers.is_empty() {
                xhttp.insert("headers".to_owned(), json!(options.headers));
            }
            if let Some(extra) = options.extra_json() {
                // Xray accepts `extra` as a JSON-encoded string.
                xhttp.insert("extra".to_owned(), json!(extra));
            }
            Some(Value::Object(xhttp))
        }
    }
}

fn tls_settings(tls: &crate::protocol::TlsConfig, server: &str) -> Value {
    let mut settings = Map::new();
    settings.insert(
        "serverName".to_owned(),
        json!(tls.effective_server_name(server)),
    );

    if !tls.alpn.is_empty() {
        settings.insert("alpn".to_owned(), json!(tls.alpn));
    }
    settings.insert(
        "fingerprint".to_owned(),
        json!(tls.effective_fingerprint()),
    );

    if let Some(hash) = tls.certificate_sha256_value() {
        settings.insert("pinnedPeerCertificateChainSha256".to_owned(), json!(hash));
    }
    if let Some(name) = tls
        .verify_peer_cert_by_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        settings.insert("verifyPeerCertByName".to_owned(), json!(name));
    }
    if let Some(version) = tls.min_version.as_deref().filter(|value| !value.is_empty()) {
        settings.insert("minVersion".to_owned(), json!(version));
    }
    if let Some(version) = tls.max_version.as_deref().filter(|value| !value.is_empty()) {
        settings.insert("maxVersion".to_owned(), json!(version));
    }

    Value::Object(settings)
}

fn reality_settings(tls: &crate::protocol::TlsConfig, server: &str) -> Value {
    let mut settings = Map::new();

    let Some(reality) = tls.reality.as_ref().filter(|reality| reality.is_usable()) else {
        return Value::Object(settings);
    };

    settings.insert(
        "serverName".to_owned(),
        json!(tls.effective_server_name(server)),
    );
    settings.insert("fingerprint".to_owned(), json!(tls.effective_fingerprint()));
    settings.insert("publicKey".to_owned(), json!(reality.public_key));

    if let Some(short_id) = reality.short_id_value() {
        settings.insert("shortId".to_owned(), json!(short_id));
    }
    if let Some(spider) = reality.spider_x_value() {
        settings.insert("spiderX".to_owned(), json!(spider));
    }
    if let Some(mldsa) = reality.mldsa65_verify_value() {
        settings.insert("mldsa65Verify".to_owned(), json!(mldsa));
    }

    Value::Object(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::shared::transport::{TcpOptions, WebSocketOptions, XhttpExtra, XhttpMode, XhttpOptions};
    use crate::protocol::{AnytlsConfig, Hysteria2Config, RealityConfig, TlsConfig};
    use crate::render::context::{RenderCredentials, RenderUser};

    fn context(params: ProxyKind) -> RenderContext {
        RenderContext::new(
            RenderUser {
                id: 1,
                remark: String::new(),
            },
            RenderCredentials::new(
                "5783a3e7-e373-51cd-8642-c83782b807c5",
                "trojan-pw",
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
            alpn: vec!["h2".to_owned(), "http/1.1".to_owned()],
            ..Default::default()
        }
    }

    #[test]
    fn vless_outbound_matches_xray_shape() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            flow: Some("xtls-rprx-vision".to_owned()),
            tls: tls(),
            ..Default::default()
        })));

        assert_eq!(outbound["protocol"], "vless");
        assert_eq!(outbound["settings"]["address"], "example.com");
        assert_eq!(outbound["settings"]["port"], 443);
        assert_eq!(outbound["settings"]["id"], "5783a3e7-e373-51cd-8642-c83782b807c5");
        assert_eq!(outbound["settings"]["encryption"], "none");
        assert_eq!(outbound["settings"]["flow"], "xtls-rprx-vision");
        assert_eq!(outbound["streamSettings"]["security"], "tls");
        assert_eq!(
            outbound["streamSettings"]["tlsSettings"]["serverName"],
            "cdn.example.com"
        );
        assert_eq!(outbound["streamSettings"]["network"], "tcp");
    }

    #[test]
    fn reality_settings_use_camel_case_keys() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: false,
                reality: Some(RealityConfig {
                    enabled: true,
                    public_key: "pbk".to_owned(),
                    short_id: Some("abcd".to_owned()),
                    spider_x: Some("/spx".to_owned()),
                    mldsa65_verify: Some("mldsa".to_owned()),
                }),
                ..Default::default()
            },
            ..Default::default()
        })));

        assert_eq!(outbound["streamSettings"]["security"], "reality");
        assert_eq!(outbound["streamSettings"]["realitySettings"]["publicKey"], "pbk");
        assert_eq!(outbound["streamSettings"]["realitySettings"]["shortId"], "abcd");
        assert_eq!(outbound["streamSettings"]["realitySettings"]["spiderX"], "/spx");
        assert_eq!(
            outbound["streamSettings"]["realitySettings"]["mldsa65Verify"],
            "mldsa"
        );
    }

    #[test]
    fn xhttp_extra_is_emitted_as_a_json_string() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Xhttp(XhttpOptions {
                host: Some("cdn.example.com".to_owned()),
                path: Some("/x".to_owned()),
                mode: XhttpMode::StreamOne,
                extra: Some(XhttpExtra {
                    x_padding_bytes: Some("100-1000".to_owned()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        })));

        assert_eq!(outbound["streamSettings"]["network"], "xhttp");
        let settings = &outbound["streamSettings"]["xhttpSettings"];
        assert_eq!(settings["mode"], "stream-one");
        let extra = settings["extra"].as_str().expect("extra is a string");
        assert!(extra.contains("xPaddingBytes"));
    }

    #[test]
    fn tcp_header_camouflage_is_rendered() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Tcp(TcpOptions {
                header_type: TcpHeaderType::Http,
            }),
            ..Default::default()
        })));

        assert_eq!(outbound["streamSettings"]["tcpSettings"]["header"]["type"], "http");
    }

    #[test]
    fn ws_early_data_uses_xray_names() {
        let outbound = outbound(&context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            transport: Transport::Ws(WebSocketOptions {
                path: Some("/ws".to_owned()),
                max_early_data: Some(2048),
                ..Default::default()
            }),
            ..Default::default()
        })));

        assert_eq!(outbound["streamSettings"]["wsSettings"]["maxEarlyData"], 2048);
        assert_eq!(
            outbound["streamSettings"]["wsSettings"]["earlyDataHeaderName"],
            "Sec-WebSocket-Protocol"
        );
    }

    #[test]
    fn trojan_uses_server_list_settings() {
        let outbound = outbound(&context(ProxyKind::Trojan(TrojanConfig {
            tls: tls(),
            ..Default::default()
        })));

        assert_eq!(outbound["protocol"], "trojan");
        assert_eq!(outbound["settings"]["servers"][0]["password"], "trojan-pw");
        assert_eq!(outbound["settings"]["servers"][0]["port"], 443);
    }

    #[test]
    fn unsupported_protocols_become_blackholes() {
        for params in [
            ProxyKind::Hysteria2(Hysteria2Config::default()),
            ProxyKind::Anytls(AnytlsConfig::default()),
        ] {
            let outbound = outbound(&context(params));
            assert_eq!(outbound["protocol"], "blackhole");
            assert!(outbound["tag"].is_string());
        }
    }

    #[test]
    fn collection_is_a_complete_xray_config() {
        let contexts = vec![context(ProxyKind::Vless(VlessConfig {
            tls: tls(),
            ..Default::default()
        }))];

        let rendered = render_collection(&contexts).expect("render");
        let value: Value = serde_json::from_str(&rendered).expect("valid json");
        assert_eq!(value["inbounds"][0]["protocol"], "socks");
        assert_eq!(value["outbounds"][0]["protocol"], "vless");
    }
}
