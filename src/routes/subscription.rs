use axum::{
    body::Body,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
};
use serde::Deserialize;

use crate::db::users;
use crate::render::{context::RenderUser, render_collection, Core, RenderContext};
use crate::state::AppState;
use crate::utils::hash_secret;

#[derive(Deserialize, Default)]
struct SubscriptionQuery {
    /// Explicit client format, overriding user-agent detection.
    target: Option<String>,
    /// Alias for `target` used by some clients.
    flag: Option<String>,
    /// Request a downloadable attachment.
    download: Option<u8>,
}

/// Maps a user agent to the client format it expects.
///
/// Reference: the `User-Agent` values listed by each client's documentation.
fn detect_target(user_agent: &str) -> Option<Core> {
    let ua = user_agent.to_ascii_lowercase();

    if ua.contains("sing-box") || ua.contains("singbox") {
        return Some(Core::SingBox);
    }
    if ua.contains("clash") || ua.contains("mihomo") || ua.contains("stash") {
        return Some(Core::Mihomo);
    }
    if ua.contains("hysteria") {
        return Some(Core::Hysteria2);
    }
    if ua.contains("xray") {
        return Some(Core::Xray);
    }
    if ua.contains("v2ray") || ua.contains("shadowrocket") || ua.contains("quantumult") {
        return Some(Core::Uri);
    }
    None
}

/// Handles a request under the configured subscription prefix.
///
/// Every failure mode — unknown token, disabled user, no nodes, or an internal
/// error — produces the disguise page instead of a diagnostic message.
pub async fn respond(
    state: &AppState,
    headers: HeaderMap,
    query: Option<&str>,
    token: &str,
) -> Response {
    let disguise = state.disguise();

    if token.is_empty() || token.contains('/') {
        return disguise.not_found().await;
    }

    let user = match users::find_by_token(&state.db, &hash_secret(token)).await {
        Ok(Some(user)) => user,
        Ok(None) => return disguise.not_found().await,
        Err(error) => {
            tracing::warn!(%error, "subscription lookup failed");
            return disguise.not_found().await;
        }
    };

    if !user.enabled {
        return disguise.not_found().await;
    }

    let params = parse_query(query);
    let settings = state.settings_cached();

    let target = params
        .target
        .as_deref()
        .or(params.flag.as_deref())
        .and_then(Core::parse)
        .or_else(|| {
            headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .and_then(detect_target)
        })
        .unwrap_or_else(|| Core::parse(&settings.default_target).unwrap_or(Core::Uri));

    let resolved = match crate::db::credentials::resolved_for_user(&state.db, &state.key, user.id)
        .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            tracing::warn!(%error, "credential lookup failed");
            return disguise.not_found().await;
        }
    };

    if resolved.is_empty() {
        return disguise.not_found().await;
    }

    let render_user = RenderUser {
        id: user.id,
        remark: user.remark.clone(),
    };

    let mut contexts: Vec<RenderContext> = Vec::with_capacity(resolved.len());
    for (index, (node, credential)) in resolved.into_iter().enumerate() {
        let params = match serde_json::from_str(&node.params) {
            Ok(params) => params,
            Err(error) => {
                tracing::warn!(%error, node_id = node.id, "node params are invalid");
                continue;
            }
        };

        contexts.push(
            RenderContext::new(
                render_user.clone(),
                crate::render::RenderCredentials::new(
                    credential.uuid,
                    credential.password,
                    credential.email,
                ),
                params,
            )
            .with_node(node.server, node.port as u16, node.name)
            .with_index(index),
        );
    }

    if contexts.is_empty() {
        return disguise.not_found().await;
    }

    let content = match render_collection(target, &contexts) {
        Ok(content) => content,
        Err(error) => {
            tracing::warn!(%error, "rendering failed");
            return disguise.not_found().await;
        }
    };

    if let Err(error) = users::record_access(&state.db, user.id).await {
        tracing::warn!(%error, "failed to record subscription access");
    }

    let body = match target {
        // Subscription URI lists are conventionally base64 encoded, which is what
        // most importers expect by default.
        Core::Uri if params.download.is_none() => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(content.as_bytes())
        }
        _ => content,
    };

    build_response(target, body, params.download.unwrap_or(0) == 1)
}

/// Hysteria2 clients can only consume Hysteria2 nodes.
#[allow(dead_code)]
fn supports_hysteria2_placeholder() {}

fn build_response(core: Core, body: String, download: bool) -> Response {
    let mut response = Response::new(Body::from(body));

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    // Clients use this hint to decide how often to refresh the subscription.
    response.headers_mut().insert(
        header::HeaderName::from_static("profile-update-interval"),
        HeaderValue::from_static("12"),
    );

    if download {
        let filename = format!("subscription.{}", core.extension());
        if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\"")) {
            response
                .headers_mut()
                .insert(header::CONTENT_DISPOSITION, value);
        }
    }

    *response.status_mut() = StatusCode::OK;
    response
}

/// Minimal query parser: enough for the flat parameters clients send.
fn parse_query(query: Option<&str>) -> SubscriptionQuery {
    let Some(query) = query else {
        return SubscriptionQuery::default();
    };

    let mut parsed = SubscriptionQuery::default();
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value);
        match key {
            "target" => parsed.target = Some(value),
            "flag" => parsed.flag = Some(value),
            "download" => parsed.download = value.parse().ok(),
            _ => {}
        }
    }
    parsed
}

/// Decodes `%XX` escapes and `+` as space.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(bytes[index]);
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agents_map_to_clients() {
        assert_eq!(detect_target("sing-box 1.9.0"), Some(Core::SingBox));
        assert_eq!(detect_target("Clash.Meta/1.17"), Some(Core::Mihomo));
        assert_eq!(detect_target("v2rayN/6.23"), Some(Core::Uri));
        assert_eq!(detect_target("Hysteria/2.0"), Some(Core::Hysteria2));
        assert_eq!(detect_target("Mozilla/5.0"), None);
    }

    #[test]
    fn query_parameters_are_parsed() {
        let parsed = parse_query(Some("target=sing-box&download=1"));
        assert_eq!(parsed.target.as_deref(), Some("sing-box"));
        assert_eq!(parsed.download, Some(1));
    }

    #[test]
    fn flag_is_parsed_as_a_target_alias() {
        let parsed = parse_query(Some("flag=clash"));
        assert_eq!(parsed.flag.as_deref(), Some("clash"));
    }

    #[test]
    fn percent_escapes_are_decoded() {
        assert_eq!(percent_decode("sing%2Dbox"), "sing-box");
        assert_eq!(percent_decode("a+b"), "a b");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn empty_query_yields_defaults() {
        let parsed = parse_query(None);
        assert!(parsed.target.is_none());
        assert!(parsed.download.is_none());
    }
}
