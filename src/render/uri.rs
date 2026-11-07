// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Share-link rendering.
//!
//! VLESS and Trojan follow the standardised link format described by the Xray
//! share-link specification: <https://github.com/XTLS/Xray-core/discussions/716>
//! Hysteria2, TUIC, and AnyTLS follow the schemes their own clients defined.

use crate::protocol::{
    AnytlsConfig, Hysteria2Config, ProxyKind, Transport, TrojanConfig, TuicConfig, VlessConfig,
};
use crate::utils::{format_host, url_encode};

use super::context::RenderContext;

/// Renders one node as a single share link.
pub fn render(context: &RenderContext) -> String {
    match &context.params {
        ProxyKind::Vless(config) => vless(context, config),
        ProxyKind::Trojan(config) => trojan(context, config),
        ProxyKind::Hysteria2(config) => hysteria2(context, config),
        ProxyKind::Tuic(config) => tuic(context, config),
        ProxyKind::Anytls(config) => anytls(context, config),
    }
}

/// Renders a whole subscription as newline-separated links.
pub fn render_collection(contexts: &[RenderContext]) -> String {
    contexts.iter().map(render).collect::<Vec<_>>().join("\r\n")
}

fn vless(context: &RenderContext, config: &VlessConfig) -> String {
    let mut query = vec![("type", config.transport.uri_name().to_owned())];

    if let Some(flow) = config.effective_flow() {
        query.push(("flow", flow.to_owned()));
    }
    if let Some(encoding) = config.packet_encoding_value() {
        query.push(("packetEncoding", encoding.to_owned()));
    }
    if let Some(encryption) = config
        .encryption
        .as_deref()
        .filter(|value| !value.is_empty() && *value != "none")
    {
        query.push(("encryption", encryption.to_owned()));
    }

    push_transport(&mut query, &config.transport);
    push_tls(&mut query, config.tls.enabled, &config.tls, &context.server);
    push_mux_uri(&mut query, &config.mux);

    build_uri(
        "vless",
        &format!(
            "{}@{}",
            url_encode(&context.credentials.uuid),
            format_host(&context.server)
        ),
        context.port,
        &query,
        &context.label(),
    )
}

fn trojan(context: &RenderContext, config: &TrojanConfig) -> String {
    let mut query = vec![("type", config.transport.uri_name().to_owned())];

    push_transport(&mut query, &config.transport);
    push_tls(&mut query, true, &config.tls, &context.server);
    push_mux_uri(&mut query, &config.mux);

    build_uri(
        "trojan",
        &format!(
            "{}@{}",
            url_encode(&context.credentials.password),
            format_host(&context.server)
        ),
        context.port,
        &query,
        &context.label(),
    )
}

fn hysteria2(context: &RenderContext, config: &Hysteria2Config) -> String {
    let mut query: Vec<(&str, String)> = Vec::new();

    if let Some(ports) = config.server_ports_joined() {
        query.push(("mport", ports));
    }
    if let Some(interval) = config.hop_interval_value() {
        query.push(("hop_interval", interval.to_owned()));
    }
    if let Some(up) = config.up_mbps {
        query.push(("upmbps", up.to_string()));
    }
    if let Some(down) = config.down_mbps {
        query.push(("downmbps", down.to_string()));
    }
    if let Some(obfs) = config.obfs_value() {
        query.push(("obfs", obfs.type_name().to_owned()));
        query.push(("obfs-password", obfs.password.clone()));
    }
    if let Some(profile) = non_empty(&config.bbr_profile) {
        query.push(("bbr-profile", profile.to_owned()));
    }
    if config.brutal_debug {
        query.push(("brutal_debug", "1".to_owned()));
    }
    if config.disable_chrome_parrot {
        query.push(("disable-chrome-parrot", "true".to_owned()));
    }
    if config.tls.insecure {
        query.push(("insecure", "1".to_owned()));
    }
    push_tls_common(&mut query, &config.tls, &context.server);

    let userinfo = format!(
        "{}:{}",
        url_encode(&context.credentials.email),
        url_encode(&context.credentials.password)
    );

    build_uri(
        "hysteria2",
        &format!("{userinfo}@{}", format_host(&context.server)),
        context.port,
        &query,
        &context.label(),
    )
}

fn tuic(context: &RenderContext, config: &TuicConfig) -> String {
    let mut query: Vec<(&str, String)> = vec![
        (
            "congestion_control",
            config.congestion_control.as_str().to_owned(),
        ),
        ("udp_relay_mode", config.udp_relay_mode.as_str().to_owned()),
    ];

    if config.udp_over_stream {
        query.push(("udp_over_stream", "1".to_owned()));
    }
    if config.zero_rtt_handshake {
        query.push(("zero_rtt_handshake", "1".to_owned()));
    }
    if let Some(heartbeat) = config.heartbeat_value() {
        query.push(("heartbeat", heartbeat.to_owned()));
    }
    if config.tls.insecure {
        query.push(("insecure", "1".to_owned()));
    }
    push_tls_common(&mut query, &config.tls, &context.server);

    let userinfo = format!(
        "{}:{}",
        url_encode(&context.credentials.uuid),
        url_encode(&context.credentials.password)
    );

    build_uri(
        "tuic",
        &format!("{userinfo}@{}", format_host(&context.server)),
        context.port,
        &query,
        &context.label(),
    )
}

fn anytls(context: &RenderContext, config: &AnytlsConfig) -> String {
    let mut query: Vec<(&str, String)> =
        vec![("security", "tls".to_owned()), ("type", "tcp".to_owned())];

    if let Some(value) = config
        .idle_session_check_interval
        .as_deref()
        .filter(|v| !v.is_empty())
    {
        query.push(("idle_session_check_interval", value.to_owned()));
    }
    if let Some(value) = config
        .idle_session_timeout
        .as_deref()
        .filter(|v| !v.is_empty())
    {
        query.push(("idle_session_timeout", value.to_owned()));
    }
    if let Some(value) = config.min_idle_session {
        query.push(("min_idle_session", value.to_string()));
    }
    if let Some(value) = config.client_metadata.as_deref().filter(|v| !v.is_empty()) {
        query.push(("client_metadata", value.to_owned()));
    }
    if config.tls.insecure {
        query.push(("insecure", "1".to_owned()));
    }
    push_tls_common(&mut query, &config.tls, &context.server);

    build_uri(
        "anytls",
        &format!(
            "{}@{}",
            url_encode(&context.credentials.password),
            format_host(&context.server)
        ),
        context.port,
        &query,
        &context.label(),
    )
}

/// Adds the transport-specific query fields defined by the share-link standard.
fn push_transport(query: &mut Vec<(&str, String)>, transport: &Transport) {
    match transport {
        Transport::Tcp(_) => {}
        Transport::Ws(options) => {
            query.push(("path", options.path_or_default().to_owned()));
            if let Some(host) = non_empty(&options.host) {
                query.push(("host", host.to_owned()));
            }
            if let Some(early) = options.max_early_data {
                query.push(("ed", early.to_string()));
            }
            if let Some(name) = non_empty(&options.early_data_header_name) {
                query.push(("eh", name.to_owned()));
            }
        }
        Transport::Grpc(options) => {
            if let Some(service) = non_empty(&options.service_name) {
                query.push(("serviceName", service.to_owned()));
            }
            if let Some(mode) = non_empty(&options.mode) {
                query.push(("mode", mode.to_owned()));
            }
            if let Some(authority) = non_empty(&options.authority) {
                query.push(("authority", authority.to_owned()));
            }
        }
        Transport::Http(options) => {
            if !options.host.is_empty() {
                query.push(("host", options.host.join(",")));
            }
            query.push(("path", options.path_or_default().to_owned()));
        }
        Transport::HttpUpgrade(options) => {
            query.push(("path", options.path_or_default().to_owned()));
            if let Some(host) = non_empty(&options.host) {
                query.push(("host", host.to_owned()));
            }
        }
        Transport::Xhttp(options) => {
            query.push(("path", options.path_or_default().to_owned()));
            if let Some(host) = non_empty(&options.host) {
                query.push(("host", host.to_owned()));
            }
            query.push(("mode", options.mode.as_str().to_owned()));
            for (name, value) in &options.headers {
                query.push(("header", format!("{name}:{value}")));
            }
            if let Some(extra) = options.extra_json() {
                query.push(("extra", extra));
            }
        }
    }
}

/// Adds `security=` plus the TLS fields for a link.
fn push_tls(
    query: &mut Vec<(&str, String)>,
    enabled: bool,
    tls: &crate::protocol::TlsConfig,
    server: &str,
) {
    if tls.reality_enabled() {
        query.push(("security", "reality".to_owned()));
    } else if enabled {
        query.push(("security", "tls".to_owned()));
    } else {
        return;
    }

    push_tls_common(query, tls, server);

    if let Some(reality) = tls.reality.as_ref().filter(|reality| reality.is_usable()) {
        query.push(("pbk", reality.public_key.clone()));
        if let Some(short_id) = reality.short_id_value() {
            query.push(("sid", short_id.to_owned()));
        }
        if let Some(spider) = reality.spider_x_value() {
            query.push(("spx", spider.to_owned()));
        }
        if let Some(mldsa) = reality.mldsa65_verify_value() {
            query.push(("pqv", mldsa.to_owned()));
        }
    }
}

/// TLS fields common to every scheme.
fn push_tls_common(
    query: &mut Vec<(&str, String)>,
    tls: &crate::protocol::TlsConfig,
    server: &str,
) {
    if !tls.is_active() {
        return;
    }

    query.push(("sni", tls.effective_server_name(server).to_owned()));

    if let Some(alpn) = tls.alpn_joined() {
        query.push(("alpn", alpn));
    }

    if tls.enabled && !tls.reality_enabled() {
        query.push(("fp", tls.effective_fingerprint().to_owned()));
    }

    if let Some(hash) = tls.certificate_sha256_value() {
        query.push(("pcs", hash.to_owned()));
    }
    if let Some(name) = tls
        .verify_peer_cert_by_name
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        query.push(("vcn", name.to_owned()));
    }
}

fn push_mux_uri(query: &mut Vec<(&str, String)>, mux: &crate::protocol::MuxConfig) {
    if !mux.is_active() {
        return;
    }
    query.push(("mux", mux.protocol.as_str().to_owned()));
    if let Some(max_streams) = mux.max_streams {
        query.push(("muxConcurrency", max_streams.to_string()));
    }
}

/// Assembles `scheme://authority:port?query#label`.
fn build_uri(
    scheme: &str,
    authority: &str,
    port: u16,
    query: &[(&str, String)],
    label: &str,
) -> String {
    let mut url = format!("{scheme}://{authority}:{port}");

    let filtered: Vec<String> = query
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| format!("{key}={}", url_encode(value)))
        .collect();

    if !filtered.is_empty() {
        url.push('?');
        url.push_str(&filtered.join("&"));
    }

    if !label.is_empty() {
        url.push('#');
        url.push_str(&url_encode(label));
    }

    url
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::shared::transport::{
        GrpcOptions, HttpUpgradeOptions, WebSocketOptions, XhttpExtra, XhttpMode, XhttpOptions,
    };
    use crate::protocol::{RealityConfig, TlsConfig};
    use crate::render::context::{RenderContext, RenderCredentials, RenderUser};

    fn user() -> RenderUser {
        RenderUser {
            id: 1,
            remark: "手机".to_owned(),
        }
    }

    fn credentials() -> RenderCredentials {
        RenderCredentials::new(
            "b0dd64e4-0fbd-4038-9139-d1f32a68a0dc",
            "pass/word",
            "user@tokyo",
        )
    }

    fn context(params: ProxyKind) -> RenderContext {
        RenderContext::new(user(), credentials(), params).with_node("2001:db8::1", 443, "Tokyo")
    }

    #[test]
    fn vless_tcp_tls_link_matches_the_spec_shape() {
        let params = ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                server_name: Some("cdn.example.com".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(
            link.starts_with("vless://b0dd64e4-0fbd-4038-9139-d1f32a68a0dc@[2001:db8::1]:443?"),
            "unexpected link: {link}"
        );
        assert!(link.contains("type=tcp"));
        assert!(link.contains("security=tls"));
        assert!(link.contains("sni=cdn.example.com"));
        assert!(!link.contains("flow="));
        assert!(link.ends_with("#Tokyo%20-%20%E6%89%8B%E6%9C%BA"));
    }

    #[test]
    fn vless_reality_link_carries_reality_fields() {
        let params = ProxyKind::Vless(VlessConfig {
            flow: Some("xtls-rprx-vision".to_owned()),
            tls: TlsConfig {
                enabled: false,
                reality: Some(RealityConfig {
                    enabled: true,
                    public_key: "pubkey".to_owned(),
                    short_id: Some("0123abcd".to_owned()),
                    spider_x: Some("/spider".to_owned()),
                    mldsa65_verify: None,
                }),
                ..Default::default()
            },
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.contains("security=reality"));
        assert!(link.contains("pbk=pubkey"));
        assert!(link.contains("sid=0123abcd"));
        assert!(link.contains("spx=%2Fspider"));
        assert!(link.contains("flow=xtls-rprx-vision"));
    }

    #[test]
    fn vless_xhttp_link_includes_mode_and_extra() {
        let params = ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            transport: Transport::Xhttp(XhttpOptions {
                host: Some("cdn.example.com".to_owned()),
                path: Some("/x".to_owned()),
                mode: XhttpMode::PacketUp,
                extra: Some(XhttpExtra {
                    x_padding_bytes: Some("100-1000".to_owned()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.contains("type=xhttp"));
        assert!(link.contains("host=cdn.example.com"));
        assert!(link.contains("path=%2Fx"));
        assert!(link.contains("mode=packet-up"));
        assert!(link.contains("extra="));
    }

    #[test]
    fn websocket_fields_use_share_link_names() {
        let params = ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            transport: Transport::Ws(WebSocketOptions {
                path: Some("/ws".to_owned()),
                host: Some("ws.example.com".to_owned()),
                max_early_data: Some(2048),
                early_data_header_name: Some("Sec-WebSocket-Protocol".to_owned()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.contains("type=ws"));
        assert!(link.contains("path=%2Fws"));
        assert!(link.contains("host=ws.example.com"));
        assert!(link.contains("ed=2048"));
        assert!(link.contains("eh=Sec-WebSocket-Protocol"));
    }

    #[test]
    fn grpc_and_httpupgrade_fields_are_emitted() {
        let grpc = ProxyKind::Vless(VlessConfig {
            transport: Transport::Grpc(GrpcOptions {
                service_name: Some("TunService".to_owned()),
                mode: Some("multi".to_owned()),
                authority: Some("grpc.example.com".to_owned()),
            }),
            ..Default::default()
        });
        let link = render(&context(grpc));
        assert!(link.contains("serviceName=TunService"));
        assert!(link.contains("mode=multi"));
        assert!(link.contains("authority=grpc.example.com"));

        let upgrade = ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            transport: Transport::HttpUpgrade(HttpUpgradeOptions {
                host: Some("up.example.com".to_owned()),
                path: Some("/up".to_owned()),
            }),
            ..Default::default()
        });
        let link = render(&context(upgrade));
        assert!(link.contains("type=httpupgrade"));
        assert!(link.contains("host=up.example.com"));
    }

    #[test]
    fn trojan_encodes_password_in_userinfo() {
        let params = ProxyKind::Trojan(TrojanConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.starts_with("trojan://pass%2Fword@[2001:db8::1]:443?"));
        assert!(link.contains("security=tls"));
    }

    #[test]
    fn hysteria2_link_carries_bandwidth_and_obfs() {
        let params = ProxyKind::Hysteria2(Hysteria2Config {
            server_ports: vec!["2080:3000".to_owned()],
            hop_interval: Some("30s".to_owned()),
            up_mbps: Some(30),
            down_mbps: Some(200),
            obfs: Some(crate::protocol::ObfsConfig {
                password: "obfs-pass".to_owned(),
                ..Default::default()
            }),
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.starts_with(
            "hysteria2://user%40tokyo:pass%2Fword@[2001:db8::1]:443?"
        ));
        assert!(link.contains("mport=2080%3A3000"));
        assert!(link.contains("hop_interval=30s"));
        assert!(link.contains("upmbps=30"));
        assert!(link.contains("downmbps=200"));
        assert!(link.contains("obfs=salamander"));
        assert!(link.contains("obfs-password=obfs-pass"));
    }

    #[test]
    fn tuic_link_carries_uuid_and_relay_mode() {
        let params = ProxyKind::Tuic(TuicConfig {
            congestion_control: crate::protocol::CongestionControl::Bbr,
            udp_relay_mode: crate::protocol::UdpRelayMode::Quic,
            heartbeat: Some("10s".to_owned()),
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.starts_with(
            "tuic://b0dd64e4-0fbd-4038-9139-d1f32a68a0dc:pass%2Fword@[2001:db8::1]:443?"
        ));
        assert!(link.contains("congestion_control=bbr"));
        assert!(link.contains("udp_relay_mode=quic"));
        assert!(link.contains("heartbeat=10s"));
    }

    #[test]
    fn anytls_link_is_rendered() {
        let link = render(&context(ProxyKind::Anytls(AnytlsConfig::default())));
        assert!(link.starts_with("anytls://pass%2Fword@[2001:db8::1]:443?"));
        assert!(link.contains("security=tls"));
    }

    #[test]
    fn collection_joins_with_crlf() {
        let params = ProxyKind::Anytls(AnytlsConfig::default());
        let contexts = vec![context(params.clone()), context(params)];
        let output = render_collection(&contexts);
        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("\r\n"));
    }

    #[test]
    fn certificate_pins_are_emitted() {
        let params = ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                certificate_sha256: Some("aabbcc".to_owned()),
                verify_peer_cert_by_name: Some("pin.example.com".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        });

        let link = render(&context(params));
        assert!(link.contains("pcs=aabbcc"));
        assert!(link.contains("vcn=pin.example.com"));
    }
}
