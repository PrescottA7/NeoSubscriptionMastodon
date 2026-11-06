// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Server-side configuration rendering.
//!
//! The output of this module is meant to be started by the proxy server itself
//! (`xray -c config.json`, `hysteria2 -c config.yaml`), not imported by a client.
//! It therefore carries the inbound listener, the REALITY private key, and every
//! account that NSM knows about for the node.

use serde_json::{Map, Value, json};

use crate::protocol::{InboundProtocol, ServerConfig, ServerCore};
use crate::reality;

/// One account as the server must know it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAccount {
    /// Pseudo e-mail the clients display; also used as the account id.
    pub email: String,
    pub uuid: String,
    pub password: String,
    /// Per-account flow override; `None` inherits the inbound default.
    pub flow: Option<String>,
}

impl ServerAccount {
    pub fn new(
        email: impl Into<String>,
        uuid: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            email: email.into(),
            uuid: uuid.into(),
            password: password.into(),
            flow: None,
        }
    }

    pub fn with_flow(mut self, flow: Option<String>) -> Self {
        self.flow = flow;
        self
    }
}

/// Renders the complete server document for `config` using `core`.
///
/// `core` is passed in rather than read from `config` so the caller can preview
/// the other exporter without mutating saved state.
pub fn render(
    config: &ServerConfig,
    accounts: &[ServerAccount],
    node_name: &str,
    core: ServerCore,
) -> anyhow::Result<String> {
    match core {
        ServerCore::Xray => Ok(serde_json::to_string_pretty(&xray_config(
            config, accounts, node_name,
        )?)?),
        ServerCore::Hysteria2 => hysteria2_config(config, accounts, node_name),
    }
}

/// Builds a runnable Xray configuration.
pub fn xray_config(
    config: &ServerConfig,
    accounts: &[ServerAccount],
    node_name: &str,
) -> anyhow::Result<Value> {
    let inbound = inbound_object(config, accounts)?;

    Ok(json!({
        "log": { "loglevel": "warning" },
        "inbounds": [inbound],
        "outbounds": [
            { "protocol": "freedom", "tag": "direct" },
            { "protocol": "blackhole", "tag": "block" }
        ],
        "routing": {
            "domainStrategy": "AsIs",
            "rules": [
                { "type": "field", "ip": ["geoip:private"], "outboundTag": "block" }
            ]
        },
        "remarks": node_name,
    }))
}

fn inbound_object(config: &ServerConfig, accounts: &[ServerAccount]) -> anyhow::Result<Value> {
    let inbound = &config.inbound;

    let mut object = Map::new();
    object.insert("listen".to_owned(), json!(inbound.listen));
    object.insert("port".to_owned(), json!(inbound.port));
    object.insert("protocol".to_owned(), json!(inbound.protocol.as_str()));
    object.insert("tag".to_owned(), json!(inbound.tag));

    if inbound.sniffing {
        object.insert(
            "sniffing".to_owned(),
            json!({
                "enabled": true,
                "destOverride": inbound.dest_override,
                "routeOnly": true,
            }),
        );
    }

    object.insert(
        "settings".to_owned(),
        settings_object(inbound.protocol, accounts),
    );

    let stream = stream_settings(config)?;
    if !stream.as_object().is_some_and(|map| map.is_empty()) {
        object.insert("streamSettings".to_owned(), stream);
    }

    if !config.fallbacks.is_empty() {
        object.insert(
            "fallbacks".to_owned(),
            Value::Array(
                config
                    .fallbacks
                    .iter()
                    .filter(|fallback| !fallback.dest.is_empty())
                    .map(|fallback| {
                        let mut entry = Map::new();
                        entry.insert("dest".to_owned(), json!(fallback.dest));
                        if !fallback.alpn.is_empty() {
                            entry.insert("alpn".to_owned(), json!(fallback.alpn));
                        }
                        if !fallback.name.is_empty() {
                            entry.insert("name".to_owned(), json!(fallback.name));
                        }
                        if !fallback.path.is_empty() {
                            entry.insert("path".to_owned(), json!(fallback.path));
                        }
                        Value::Object(entry)
                    })
                    .collect(),
            ),
        );
    }

    let sockopt = sockopt_object(config);
    if !sockopt.is_empty() {
        object.insert("sockopt".to_owned(), Value::Object(sockopt));
    }

    Ok(Value::Object(object))
}

fn settings_object(protocol: InboundProtocol, accounts: &[ServerAccount]) -> Value {
    match protocol {
        InboundProtocol::Vless => json!({
            "clients": accounts
                .iter()
                .map(|account| {
                    let mut client = Map::new();
                    client.insert("id".to_owned(), json!(account.uuid));
                    client.insert("email".to_owned(), json!(account.email));
                    // VLESS accounts carry no password; the uuid is the secret.
                    if let Some(flow) = account.flow.as_deref().filter(|value| !value.is_empty()) {
                        client.insert("flow".to_owned(), json!(flow));
                    }
                    Value::Object(client)
                })
                .collect::<Vec<_>>(),
            "decryption": "none",
        }),
        InboundProtocol::Trojan => json!({
            "clients": accounts
                .iter()
                .map(|account| json!({
                    "password": account.password,
                    "email": account.email,
                }))
                .collect::<Vec<_>>(),
        }),
        // Xray has no inbound for these protocols; the caller should have picked
        // the Hysteria2 exporter instead.
        InboundProtocol::Hysteria2 | InboundProtocol::Tuic | InboundProtocol::Anytls => {
            json!({ "clients": [] })
        }
    }
}

fn stream_settings(config: &ServerConfig) -> anyhow::Result<Value> {
    let inbound = &config.inbound;
    let mut stream = Map::new();

    stream.insert("network".to_owned(), json!(inbound.transport));

    if config.uses_reality() {
        stream.insert("security".to_owned(), json!("reality"));
        stream.insert(
            "realitySettings".to_owned(),
            json!({
                "show": false,
                "target": config.reality.target,
                "xver": 0,
                "serverNames": config.reality.server_names,
                "privateKey": config.reality.private_key,
                "shortIds": config.reality.short_ids,
                "mldsa65Seed": config.reality.mldsa65_seed,
                "maxTimeDiff": config.reality.max_time_diff,
                "minClientVer": config.reality.min_client_ver,
                "maxClientVer": config.reality.max_client_ver,
            }),
        );
    } else if !config.tls.certificate_file.is_empty() {
        let security = if config.tls.security.is_empty() {
            "tls"
        } else {
            &config.tls.security
        };
        stream.insert("security".to_owned(), json!(security));
        stream.insert(
            "tlsSettings".to_owned(),
            json!({
                "certificates": [{
                    "certificateFile": config.tls.certificate_file,
                    "keyFile": config.tls.key_file,
                }],
                "alpn": config.tls.alpn,
            }),
        );
    }

    match inbound.transport.as_str() {
        "ws" => {
            stream.insert(
                "wsSettings".to_owned(),
                json!({ "path": inbound.path, "host": inbound.host }),
            );
        }
        "grpc" => {
            stream.insert(
                "grpcSettings".to_owned(),
                json!({ "serviceName": inbound.service_name }),
            );
        }
        "httpupgrade" => {
            stream.insert(
                "httpupgradeSettings".to_owned(),
                json!({ "path": inbound.path, "host": inbound.host }),
            );
        }
        "xhttp" => {
            stream.insert(
                "xhttpSettings".to_owned(),
                json!({ "path": inbound.path, "host": inbound.host }),
            );
        }
        _ => {}
    }

    Ok(Value::Object(stream))
}

fn sockopt_object(config: &ServerConfig) -> Map<String, Value> {
    let mut sockopt = Map::new();
    if config.sockopt.tcp_fast_open {
        sockopt.insert("tcpFastOpen".to_owned(), json!(true));
    }
    if config.sockopt.accept_proxy_protocol {
        sockopt.insert("acceptProxyProtocol".to_owned(), json!(true));
    }
    if let Some(mark) = config.sockopt.mark {
        sockopt.insert("mark".to_owned(), json!(mark));
    }
    if let Some(interval) = config.sockopt.tcp_keep_alive_interval {
        sockopt.insert("tcpKeepAliveInterval".to_owned(), json!(interval));
    }
    sockopt
}

/// Builds a runnable Hysteria2 server configuration.
fn hysteria2_config(
    config: &ServerConfig,
    accounts: &[ServerAccount],
    _node_name: &str,
) -> anyhow::Result<String> {
    let inbound = &config.inbound;

    let mut root = Map::new();
    root.insert(
        "listen".to_owned(),
        json!(format!("{}:{}", inbound.listen, inbound.port)),
    );

    // Hysteria2 authenticates with a `user: password` map.
    let mut users = Map::new();
    for account in accounts {
        // The email doubles as the username; a bare uuid has no password form.
        users.insert(account.email.clone(), json!(account.password));
    }
    root.insert(
        "auth".to_owned(),
        json!({ "type": "userpass", "userpass": users }),
    );

    let mut tls = Map::new();
    // Hysteria2 has no REALITY support, so a certificate is mandatory either way.
    tls.insert("cert".to_owned(), json!(config.tls.certificate_file));
    tls.insert("key".to_owned(), json!(config.tls.key_file));
    root.insert("tls".to_owned(), Value::Object(tls));

    let yaml = serde_yaml::to_string(&Value::Object(root))?;
    Ok(yaml)
}

/// Derives the client-facing public key from the server private key.
pub fn client_public_key(config: &ServerConfig) -> Option<String> {
    if !config.uses_reality() {
        return None;
    }
    reality::public_from_private(&config.reality.private_key).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{InboundConfig, ServerReality};

    fn vless_server() -> ServerConfig {
        ServerConfig {
            inbound: InboundConfig {
                protocol: InboundProtocol::Vless,
                port: 8443,
                tag: "vless-in".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn account() -> ServerAccount {
        ServerAccount::new(
            "alice@hk",
            "11111111-1111-4111-8111-111111111111",
            "ignored",
        )
    }

    #[test]
    fn xray_config_is_startable() {
        let value = xray_config(&vless_server(), &[account()], "HK").expect("render");

        let inbound = &value["inbounds"][0];
        assert_eq!(inbound["port"], 8443);
        assert_eq!(inbound["protocol"], "vless");
        assert_eq!(inbound["settings"]["decryption"], "none");
        assert_eq!(
            inbound["settings"]["clients"][0]["id"],
            "11111111-1111-4111-8111-111111111111"
        );
        // A runnable config needs an outbound, otherwise xray refuses to start.
        assert_eq!(value["outbounds"][0]["protocol"], "freedom");
    }

    #[test]
    fn reality_private_key_stays_on_the_server() {
        let mut config = vless_server();
        config.reality = ServerReality {
            enabled: true,
            private_key: crate::reality::generate().private_key,
            short_ids: vec!["ab".to_owned()],
            target: "www.example.com:443".to_owned(),
            ..Default::default()
        };

        let value = xray_config(&config, &[account()], "HK").expect("render");
        let stream = &value["inbounds"][0]["streamSettings"];

        assert_eq!(stream["security"], "reality");
        assert!(!stream["realitySettings"]["privateKey"].is_null());
        // The public key must never appear in a server document.
        let text = serde_json::to_string(&value).expect("json");
        assert!(!text.contains("publicKey"));
    }

    #[test]
    fn tls_listener_carries_certificate_paths() {
        let mut config = vless_server();
        config.tls.certificate_file = "/etc/ssl/fullchain.pem".to_owned();
        config.tls.key_file = "/etc/ssl/key.pem".to_owned();

        let value = xray_config(&config, &[account()], "HK").expect("render");
        let stream = &value["inbounds"][0]["streamSettings"];
        let tls = &stream["tlsSettings"];

        assert_eq!(
            tls["certificates"][0]["certificateFile"],
            "/etc/ssl/fullchain.pem"
        );
        assert_eq!(tls["certificates"][0]["keyFile"], "/etc/ssl/key.pem");
        // An empty `security` would give Xray no TLS layer at all.
        assert_eq!(stream["security"], "tls");
    }

    #[test]
    fn blank_security_falls_back_to_tls() {
        let mut config = vless_server();
        config.tls.certificate_file = "/etc/ssl/fullchain.pem".to_owned();
        config.tls.security = String::new();

        let value = xray_config(&config, &[account()], "HK").expect("render");
        assert_eq!(value["inbounds"][0]["streamSettings"]["security"], "tls");
    }

    #[test]
    fn trojan_accounts_use_passwords() {
        let mut config = vless_server();
        config.inbound.protocol = InboundProtocol::Trojan;

        let value = xray_config(
            &config,
            &[ServerAccount::new("bob@hk", "uuid", "bob-secret")],
            "HK",
        )
        .expect("render");

        assert_eq!(
            value["inbounds"][0]["settings"]["clients"][0]["password"],
            "bob-secret"
        );
    }

    #[test]
    fn flow_is_written_per_account() {
        let config = vless_server();
        let accounts = [
            ServerAccount::new("a@hk", "11111111-1111-4111-8111-111111111111", "")
                .with_flow(Some("xtls-rprx-vision".to_owned())),
            ServerAccount::new("b@hk", "22222222-2222-4222-8222-222222222222", ""),
        ];

        let value = xray_config(&config, &accounts, "HK").expect("render");
        let clients = &value["inbounds"][0]["settings"]["clients"];

        assert_eq!(clients[0]["flow"], "xtls-rprx-vision");
        assert!(clients[1].get("flow").is_none());
    }

    #[test]
    fn websocket_transport_is_emitted() {
        let mut config = vless_server();
        config.inbound.transport = "ws".to_owned();
        config.inbound.path = "/ws".to_owned();

        let value = xray_config(&config, &[account()], "HK").expect("render");
        assert_eq!(
            value["inbounds"][0]["streamSettings"]["wsSettings"]["path"],
            "/ws"
        );
    }

    #[test]
    fn fallbacks_are_emitted_when_configured() {
        let mut config = vless_server();
        config.fallbacks = vec![crate::protocol::Fallback {
            dest: "80".to_owned(),
            ..Default::default()
        }];

        let value = xray_config(&config, &[account()], "HK").expect("render");
        assert_eq!(value["inbounds"][0]["fallbacks"][0]["dest"], "80");
    }

    #[test]
    fn hysteria2_config_lists_userpass_accounts() {
        let mut config = vless_server();
        config.core = ServerCore::Hysteria2;
        config.inbound.protocol = InboundProtocol::Hysteria2;
        config.inbound.port = 443;
        config.tls.certificate_file = "/etc/ssl/fullchain.pem".to_owned();
        config.tls.key_file = "/etc/ssl/key.pem".to_owned();

        let yaml = render(
            &config,
            &[ServerAccount::new("bob@jp", "", "bob-secret")],
            "JP",
            ServerCore::Hysteria2,
        )
        .expect("render");

        // Hysteria2 expects `auth.userpass` as a map, not a list of strings.
        assert!(yaml.contains("userpass"), "{yaml}");
        assert!(yaml.contains("bob@jp: bob-secret"), "{yaml}");
        assert!(!yaml.contains("users:"), "{yaml}");
        assert!(yaml.contains("listen"), "{yaml}");
    }

    #[test]
    fn public_key_is_derived_for_clients() {
        let pair = crate::reality::generate();
        let mut config = vless_server();
        config.reality = ServerReality {
            enabled: true,
            private_key: pair.private_key.clone(),
            ..Default::default()
        };

        assert_eq!(client_public_key(&config), Some(pair.public_key));
        assert_eq!(client_public_key(&vless_server()), None);
    }
}
