// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use serde::Serialize;

use crate::protocol::ProxyKind;

/// The credentials one user holds on one specific node.
///
/// Every proxy server keeps its own account list, so these values belong to the
/// node: the same person may be `alice` on one machine and `a-01` on another.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RenderCredentials {
    /// Identifier used by VLESS and TUIC.
    pub uuid: String,
    /// Secret used by Trojan, Hysteria2, TUIC, and AnyTLS.
    pub password: String,
    /// Account label shown by clients, typically `user@node`.
    pub email: String,
}

impl RenderCredentials {
    pub fn new(
        uuid: impl Into<String>,
        password: impl Into<String>,
        email: impl Into<String>,
    ) -> Self {
        Self {
            uuid: uuid.into(),
            password: password.into(),
            email: email.into(),
        }
    }
}

/// The subscription user, used only for labelling the rendered node.
#[derive(Debug, Clone, Serialize)]
pub struct RenderUser {
    pub id: i64,
    pub remark: String,
}

impl RenderUser {
    /// Display name used when a remark is not set.
    pub fn display_name(&self) -> &str {
        &self.remark
    }
}

/// Everything a renderer needs: credentials plus the node parameters.
#[derive(Debug, Clone)]
pub struct RenderContext {
    pub user: RenderUser,
    /// Credentials valid on **this** node.
    pub credentials: RenderCredentials,
    pub params: ProxyKind,
    /// Node address, filled in by the caller that owns the node row.
    pub server: String,
    pub port: u16,
    /// Node name used as the outbound tag / proxy name.
    pub node_name: String,
    /// Index of this node within a collection, used to keep tags unique.
    pub index: usize,
}

impl RenderContext {
    /// Builds a context for a single node.
    pub fn new(
        user: RenderUser,
        credentials: RenderCredentials,
        params: ProxyKind,
    ) -> Self {
        Self {
            user,
            credentials,
            params,
            server: String::new(),
            port: 0,
            node_name: String::new(),
            index: 0,
        }
    }

    /// Attaches node identity, which is required by every full-config renderer.
    pub fn with_node(
        mut self,
        server: impl Into<String>,
        port: u16,
        node_name: impl Into<String>,
    ) -> Self {
        self.server = server.into();
        self.port = port;
        self.node_name = node_name.into();
        self
    }

    pub fn with_index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    /// Human readable label: `<node name> - <user>`, or the node name alone.
    pub fn label(&self) -> String {
        let node = if self.node_name.is_empty() {
            self.params.protocol_str().to_owned()
        } else {
            self.node_name.clone()
        };

        let name = self.user.display_name();
        if name.is_empty() {
            node
        } else {
            format!("{node} - {name}")
        }
    }

    /// Outbound tag, guaranteed unique within a collection.
    pub fn tag(&self) -> String {
        format!("{}-{}", slugify(&self.node_name, self.params.protocol_str()), self.index + 1)
    }

    /// Effective TLS configuration of the underlying protocol.
    pub fn tls(&self) -> &crate::protocol::TlsConfig {
        self.params.tls()
    }
}

/// Produces a stable, filesystem- and JSON-safe slug.
pub fn slugify(value: &str, fallback: &str) -> String {
    let slug: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{TlsConfig, VlessConfig};

    fn user(remark: &str) -> RenderUser {
        RenderUser {
            id: 1,
            remark: remark.to_owned(),
        }
    }

    fn credentials() -> RenderCredentials {
        RenderCredentials::new(
            "11111111-2222-3333-4444-555555555555",
            "s3cret",
            "alice@tokyo",
        )
    }

    fn vless() -> ProxyKind {
        ProxyKind::Vless(VlessConfig {
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn label_appends_the_user_remark() {
        let context = RenderContext::new(user("Alice"), credentials(), vless())
            .with_node("example.com", 443, "Tokyo");
        assert_eq!(context.label(), "Tokyo - Alice");
    }

    #[test]
    fn label_omits_empty_remark() {
        let context = RenderContext::new(user(""), credentials(), vless())
            .with_node("example.com", 443, "Tokyo");
        assert_eq!(context.label(), "Tokyo");
    }

    #[test]
    fn label_falls_back_to_protocol_name() {
        let context = RenderContext::new(user(""), credentials(), vless());
        assert_eq!(context.label(), "vless");
    }

    #[test]
    fn tags_are_unique_across_a_collection() {
        let first = RenderContext::new(user(""), credentials(), vless())
            .with_node("a.example.com", 443, "Tokyo")
            .with_index(0);
        let second = RenderContext::new(user(""), credentials(), vless())
            .with_node("b.example.com", 443, "Tokyo")
            .with_index(1);

        assert_ne!(first.tag(), second.tag());
        assert_eq!(first.tag(), "tokyo-1");
        assert_eq!(second.tag(), "tokyo-2");
    }

    #[test]
    fn slugify_normalises_hostile_names() {
        assert_eq!(slugify("Tokyo 01", "node"), "tokyo-01");
        assert_eq!(slugify("东京", "node"), "node");
        assert_eq!(slugify("---", "node"), "node");
        assert_eq!(slugify("A/B", "node"), "a-b");
    }
}
