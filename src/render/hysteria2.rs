//! Hysteria2 client YAML rendering.
//!
//! Only Hysteria2 nodes can be represented here; other protocols are skipped so
//! the output stays a runnable configuration.
//!
//! Reference: <https://v2.hysteria.network/docs/advanced/Full-Client-Config/>

use serde_json::{json, Map, Value};

use crate::protocol::{Hysteria2Config, ObfsType, ProxyKind};

use super::context::RenderContext;

/// Renders one Hysteria2 node as the `server` field value plus its parameters.
///
/// The result is a single-server configuration fragment, which is the closest
/// equivalent to "one config per node" that Hysteria2 supports.
pub fn render(context: &RenderContext) -> anyhow::Result<String> {
    let config = render_collection(std::slice::from_ref(context))?;
    Ok(config)
}

/// Renders every Hysteria2 node into one configuration.
///
/// Hysteria2 has a single `server` entry, so the first node wins and the rest
/// are recorded in the `nsm-extra-servers` extension for reference.
pub fn render_collection(contexts: &[RenderContext]) -> anyhow::Result<String> {
    let hysteria_nodes: Vec<(&RenderContext, &Hysteria2Config)> = contexts
        .iter()
        .filter_map(|context| match &context.params {
            ProxyKind::Hysteria2(config) => Some((context, config)),
            _ => None,
        })
        .collect();

    let mut root = Map::new();

    match hysteria_nodes.first() {
        Some((context, config)) => {
            root.insert("server".to_owned(), Value::String(server_address(context, config)));
            root.insert(
                "auth".to_owned(),
                Value::String(context.credentials.password.clone()),
            );

            if let Some(bandwidth) = bandwidth(config) {
                root.insert("bandwidth".to_owned(), bandwidth);
            }

            let tls = tls_section(context, config);
            if !tls.is_empty() {
                root.insert("tls".to_owned(), Value::Object(tls));
            }

            let transport = transport_section(config);
            if !transport.is_empty() {
                root.insert("transport".to_owned(), Value::Object(transport));
            }

            let obfs = obfs_section(config);
            if !obfs.is_empty() {
                root.insert("obfs".to_owned(), Value::Object(obfs));
            }

            let quic = quic_section(config);
            if !quic.is_empty() {
                root.insert("quic".to_owned(), Value::Object(quic));
            }
        }
        None => {
            return Err(anyhow::anyhow!(
                "no hysteria2 node available to render a hysteria2 client config"
            ));
        }
    }

    if hysteria_nodes.len() > 1 {
        let extras: Vec<Value> = hysteria_nodes
            .iter()
            .skip(1)
            .map(|(context, config)| {
                json!({
                    "name": context.tag(),
                    "server": server_address(context, config),
                })
            })
            .collect();
        root.insert("nsm-extra-servers".to_owned(), Value::Array(extras));
    }

    serde_yaml::to_string(&Value::Object(root)).map_err(Into::into)
}

/// Hysteria2 expresses port hopping directly in the `server` address.
fn server_address(context: &RenderContext, config: &Hysteria2Config) -> String {
    if config.has_port_hopping() {
        format!("{}:{}", context.server, config.server_ports.join(","))
    } else {
        format!("{}:{}", context.server, context.port)
    }
}

fn bandwidth(config: &Hysteria2Config) -> Option<Value> {
    if config.up_mbps.is_none() && config.down_mbps.is_none() {
        return None;
    }

    let mut bandwidth = Map::new();
    if let Some(up) = config.up_mbps {
        bandwidth.insert("up".to_owned(), Value::String(format!("{up} mbps")));
    }
    if let Some(down) = config.down_mbps {
        bandwidth.insert("down".to_owned(), Value::String(format!("{down} mbps")));
    }
    Some(Value::Object(bandwidth))
}

fn tls_section(context: &RenderContext, config: &Hysteria2Config) -> Map<String, Value> {
    let mut tls = Map::new();
    if !config.tls.is_active() {
        return tls;
    }

    tls.insert(
        "sni".to_owned(),
        Value::String(
            config
                .tls
                .effective_server_name(&context.server)
                .to_owned(),
        ),
    );

    if config.tls.insecure {
        tls.insert("insecure".to_owned(), Value::Bool(true));
    }
    if let Some(hash) = config.tls.certificate_sha256_value() {
        tls.insert("pinSHA256".to_owned(), Value::String(hash.to_owned()));
    }
    if !config.tls.alpn.is_empty() {
        tls.insert(
            "alpn".to_owned(),
            Value::Array(
                config
                    .tls
                    .alpn
                    .iter()
                    .map(|value| Value::String(value.clone()))
                    .collect(),
            ),
        );
    }

    tls
}

fn transport_section(config: &Hysteria2Config) -> Map<String, Value> {
    let mut transport = Map::new();
    if !config.has_port_hopping() {
        return transport;
    }

    let mut udp = Map::new();
    match (
        config.hop_interval_value(),
        config.hop_interval_max.as_deref().filter(|value| !value.is_empty()),
    ) {
        (Some(min), Some(max)) => {
            udp.insert("minHopInterval".to_owned(), Value::String(min.to_owned()));
            udp.insert("maxHopInterval".to_owned(), Value::String(max.to_owned()));
        }
        (Some(interval), None) => {
            udp.insert("hopInterval".to_owned(), Value::String(interval.to_owned()));
        }
        _ => {
            udp.insert(
                "hopInterval".to_owned(),
                Value::String("30s".to_owned()),
            );
        }
    }

    transport.insert("type".to_owned(), Value::String("udp".to_owned()));
    transport.insert("udp".to_owned(), Value::Object(udp));
    transport
}

fn obfs_section(config: &Hysteria2Config) -> Map<String, Value> {
    let mut obfs = Map::new();
    let Some(active) = config.obfs_value() else {
        return obfs;
    };

    obfs.insert(
        "type".to_owned(),
        Value::String(active.type_name().to_owned()),
    );

    let mut inner = Map::new();
    inner.insert(
        "password".to_owned(),
        Value::String(active.password.clone()),
    );
    if active.obfs_type == ObfsType::Gecko {
        if let Some(min) = active.min_packet_size {
            inner.insert("minPacketSize".to_owned(), Value::from(min));
        }
        if let Some(max) = active.max_packet_size {
            inner.insert("maxPacketSize".to_owned(), Value::from(max));
        }
    }

    obfs.insert(active.type_name().to_owned(), Value::Object(inner));
    obfs
}

fn quic_section(config: &Hysteria2Config) -> Map<String, Value> {
    let mut quic = Map::new();
    if let Some(value) = config.quic.initial_stream_receive_window {
        quic.insert("initStreamReceiveWindow".to_owned(), Value::from(value));
    }
    if let Some(value) = config.quic.max_stream_receive_window {
        quic.insert("maxStreamReceiveWindow".to_owned(), Value::from(value));
    }
    if let Some(value) = config.quic.initial_connection_receive_window {
        quic.insert("initConnReceiveWindow".to_owned(), Value::from(value));
    }
    if let Some(value) = config.quic.max_connection_receive_window {
        quic.insert("maxConnReceiveWindow".to_owned(), Value::from(value));
    }
    if let Some(value) = config.quic.keep_alive_value() {
        quic.insert(
            "keepAlivePeriod".to_owned(),
            Value::String(value.to_owned()),
        );
    }
    if config.quic.disable_path_mtu_discovery {
        quic.insert("disablePathMTUDiscovery".to_owned(), Value::Bool(true));
    }
    if config.disable_chrome_parrot {
        quic.insert("disableChromeParrot".to_owned(), Value::Bool(true));
    }
    quic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ObfsConfig, TlsConfig};
    use crate::render::context::{RenderCredentials, RenderUser};

    fn context(params: ProxyKind) -> RenderContext {
        RenderContext::new(
            RenderUser {
                id: 1,
                remark: String::new(),
            },
            RenderCredentials::new(
                "uuid",
                "hy2-pw",
                "alice@tokyo",
            ),
            params,
        )
        .with_node("example.com", 443, "Tokyo")
    }

    #[test]
    fn minimal_config_has_server_auth_and_tls() {
        let rendered = render(&context(ProxyKind::Hysteria2(Hysteria2Config::default())))
            .expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["server"], "example.com:443");
        assert_eq!(value["auth"], "hy2-pw");
        assert_eq!(value["tls"]["sni"], "example.com");
        assert!(value.get("bandwidth").is_none());
        assert!(value.get("transport").is_none());
        assert!(value.get("obfs").is_none());
    }

    #[test]
    fn port_hopping_moves_into_the_server_address() {
        let config = Hysteria2Config {
            server_ports: vec!["2080:3000".to_owned(), "4000".to_owned()],
            hop_interval: Some("15s".to_owned()),
            ..Default::default()
        };

        let rendered = render(&context(ProxyKind::Hysteria2(config))).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["server"], "example.com:2080:3000,4000");
        assert_eq!(value["transport"]["type"], "udp");
        assert_eq!(value["transport"]["udp"]["hopInterval"], "15s");
    }

    #[test]
    fn randomised_hopping_uses_min_and_max() {
        let config = Hysteria2Config {
            server_ports: vec!["20000:30000".to_owned()],
            hop_interval: Some("15s".to_owned()),
            hop_interval_max: Some("45s".to_owned()),
            ..Default::default()
        };

        let rendered = render(&context(ProxyKind::Hysteria2(config))).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["transport"]["udp"]["minHopInterval"], "15s");
        assert_eq!(value["transport"]["udp"]["maxHopInterval"], "45s");
        assert!(value["transport"]["udp"].get("hopInterval").is_none());
    }

    #[test]
    fn bandwidth_and_obfs_are_rendered() {
        let config = Hysteria2Config {
            up_mbps: Some(30),
            down_mbps: Some(200),
            obfs: Some(ObfsConfig {
                obfs_type: ObfsType::Gecko,
                password: "obfs-pw".to_owned(),
                min_packet_size: Some(512),
                max_packet_size: Some(1200),
            }),
            ..Default::default()
        };

        let rendered = render(&context(ProxyKind::Hysteria2(config))).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["bandwidth"]["up"], "30 mbps");
        assert_eq!(value["bandwidth"]["down"], "200 mbps");
        assert_eq!(value["obfs"]["type"], "gecko");
        assert_eq!(value["obfs"]["gecko"]["password"], "obfs-pw");
        assert_eq!(value["obfs"]["gecko"]["minPacketSize"], 512);
    }

    #[test]
    fn tls_and_quic_overrides_are_rendered() {
        let config = Hysteria2Config {
            tls: TlsConfig {
                enabled: true,
                server_name: Some("sni.example.com".to_owned()),
                insecure: true,
                certificate_sha256: Some("AA:BB".to_owned()),
                ..Default::default()
            },
            quic: crate::protocol::QuicConfig {
                max_stream_receive_window: Some(8_388_608),
                keep_alive_period: Some("10s".to_owned()),
                disable_path_mtu_discovery: true,
                ..Default::default()
            },
            disable_chrome_parrot: true,
            ..Default::default()
        };

        let rendered = render(&context(ProxyKind::Hysteria2(config))).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["tls"]["sni"], "sni.example.com");
        assert_eq!(value["tls"]["insecure"], true);
        assert_eq!(value["tls"]["pinSHA256"], "AA:BB");
        assert_eq!(value["quic"]["maxStreamReceiveWindow"], 8_388_608);
        assert_eq!(value["quic"]["keepAlivePeriod"], "10s");
        assert_eq!(value["quic"]["disablePathMTUDiscovery"], true);
        assert_eq!(value["quic"]["disableChromeParrot"], true);
    }

    #[test]
    fn non_hysteria_nodes_are_rejected() {
        let params = ProxyKind::Anytls(crate::protocol::AnytlsConfig::default());
        assert!(render(&context(params)).is_err());
    }

    #[test]
    fn extra_nodes_are_recorded() {
        let contexts = vec![
            context(ProxyKind::Hysteria2(Hysteria2Config::default())),
            context(ProxyKind::Hysteria2(Hysteria2Config::default())).with_index(1),
        ];

        let rendered = render_collection(&contexts).expect("render");
        let value: Value = serde_yaml::from_str(&rendered).expect("valid yaml");

        assert_eq!(value["nsm-extra-servers"][0]["name"], "tokyo-2");
    }
}
