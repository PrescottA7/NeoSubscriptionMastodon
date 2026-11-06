// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Client configuration rendering.
//!
//! Every renderer consumes the same [`RenderContext`], so a single node
//! definition produces consistent output for all supported clients.

pub mod context;
pub mod hysteria2;
pub mod mihomo;
pub mod server;
pub mod singbox;
pub mod uri;
pub mod xray;

use serde::{Deserialize, Serialize};

use crate::protocol::ProxyKind;

pub use context::{RenderContext, RenderCredentials, RenderUser};

/// Client applications a node can be exported for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Core {
    /// Subscription URI list, as imported by V2RayN and friends.
    Uri,
    SingBox,
    Mihomo,
    Xray,
    Hysteria2,
}

impl Core {
    pub const ALL: [Self; 5] = [
        Self::Uri,
        Self::SingBox,
        Self::Mihomo,
        Self::Xray,
        Self::Hysteria2,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uri => "uri",
            Self::SingBox => "sing-box",
            Self::Mihomo => "mihomo",
            Self::Xray => "xray",
            Self::Hysteria2 => "hysteria2",
        }
    }

    /// Parses a `target` query value or core name.
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "uri" | "uris" | "v2ray" | "v2rayn" | "v2rayng" | "base64" | "plain" => Some(Self::Uri),
            "sing-box" | "singbox" | "sing_box" | "sbox" => Some(Self::SingBox),
            "mihomo" | "clash" | "clash-meta" | "clashmeta" | "clash-verge" => Some(Self::Mihomo),
            "xray" | "xray-core" | "v2ray-core" => Some(Self::Xray),
            "hysteria2" | "hysteria" | "hy2" => Some(Self::Hysteria2),
            _ => None,
        }
    }

    /// Media type used when the rendered document is downloaded.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Uri => "text/plain; charset=utf-8",
            Self::SingBox | Self::Xray => "application/json; charset=utf-8",
            Self::Mihomo | Self::Hysteria2 => "text/yaml; charset=utf-8",
        }
    }

    /// File extension used for downloads.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Uri => "txt",
            Self::SingBox => "singbox.json",
            Self::Xray => "xray.json",
            Self::Mihomo => "mihomo.yaml",
            Self::Hysteria2 => "hysteria2.yaml",
        }
    }

    /// True when the renderer ignores user credentials (no account is needed).
    pub fn is_standalone(self) -> bool {
        matches!(self, Self::Uri)
    }
}

/// Renders one node for the requested core.
///
/// `Qualified` returns the complete document (full config or URI list), while
/// `Fragment` returns only the node-level piece, which the admin UI embeds into
/// an existing file.
pub fn render_node(
    core: Core,
    user: &RenderUser,
    credentials: &RenderCredentials,
    params: &ProxyKind,
) -> anyhow::Result<String> {
    let context = RenderContext::new(user.clone(), credentials.clone(), params.clone());
    render_with(core, &context)
}

/// Renders one node using a context that already carries credentials.
pub fn render_with(core: Core, context: &RenderContext) -> anyhow::Result<String> {
    match core {
        Core::Uri => Ok(uri::render(context)),
        Core::SingBox => singbox::render(context),
        Core::Mihomo => mihomo::render(context),
        Core::Xray => xray::render(context),
        Core::Hysteria2 => hysteria2::render(context),
    }
}

/// Renders every node into a single document for the requested core.
pub fn render_collection(core: Core, contexts: &[RenderContext]) -> anyhow::Result<String> {
    match core {
        Core::Uri => Ok(contexts
            .iter()
            .map(uri::render)
            .collect::<Vec<_>>()
            .join("\r\n")),
        Core::SingBox => singbox::render_collection(contexts),
        Core::Mihomo => mihomo::render_collection(contexts),
        Core::Xray => xray::render_collection(contexts),
        Core::Hysteria2 => hysteria2::render_collection(contexts),
    }
}

/// Client applications this server can render a node for.
pub fn supported_cores() -> &'static [Core] {
    &Core::ALL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_names_round_trip() {
        for core in Core::ALL {
            assert_eq!(Core::parse(core.as_str()), Some(core));
        }
    }

    #[test]
    fn core_aliases_are_accepted() {
        assert_eq!(Core::parse("clash"), Some(Core::Mihomo));
        assert_eq!(Core::parse("V2RayN"), Some(Core::Uri));
        assert_eq!(Core::parse("hy2"), Some(Core::Hysteria2));
        assert_eq!(Core::parse("unknown"), None);
    }

    #[test]
    fn content_types_match_format() {
        assert!(Core::SingBox.content_type().starts_with("application/json"));
        assert!(Core::Uri.content_type().starts_with("text/plain"));
        assert!(Core::Mihomo.content_type().starts_with("text/yaml"));
    }
}
