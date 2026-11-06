// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

/// Placeholder Vite emits for the deployment-time admin path prefix.
const BASE_PLACEHOLDER: &str = "__NSM_BASE__";

/// Serves the embedded admin UI. Unknown paths fall back to `index.html` so the
/// SPA router can handle deep links.
///
/// `asset_path` is the request path with the admin prefix already removed, and
/// `prefix` is that prefix (empty at the root), used to rewrite asset URLs.
pub async fn serve_static(prefix: &str, asset_path: &str) -> Response {
    let path = asset_path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) if path == "index.html" => text_response(content.data, prefix, "text/html; charset=utf-8", false),
        Some(content) => asset_response(content.data, path, prefix),
        None => match Assets::get("index.html") {
            Some(content) => text_response(content.data, prefix, "text/html; charset=utf-8", false),
            None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        },
    }
}

fn asset_response(
    data: std::borrow::Cow<'static, [u8]>,
    path: &str,
    prefix: &str,
) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    // CSS and JS reference fonts and chunks through the same placeholder, so the
    // rewrite has to happen wherever it appears, not only in `index.html`.
    let text = matches!(
        mime.type_().as_str(),
        "text" | "application"
    ) && matches!(
        mime.subtype().as_str(),
        "css" | "javascript" | "json"
    );

    if text {
        return text_response(data, prefix, mime.as_ref(), true);
    }

    let mut response = Response::new(Body::from(data.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    response
}

fn text_response(
    data: std::borrow::Cow<'static, [u8]>,
    prefix: &str,
    content_type: &str,
    immutable: bool,
) -> Response {
    let text = String::from_utf8_lossy(&data);

    // Two placeholder shapes: the quoted JS value and the URL prefix.
    let body = text
        .replace(&format!("\"{BASE_PLACEHOLDER}\""), &format!("\"{prefix}\""))
        .replace(&format!("/{BASE_PLACEHOLDER}"), prefix);

    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type).unwrap_or(HeaderValue::from_static("text/plain; charset=utf-8")),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    response
}

/// True when the build produced an admin UI bundle.
pub fn has_assets() -> bool {
    Assets::get("index.html").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_placeholder_expands_to_the_root_when_no_prefix_is_set() {
        let html = br#"<script src="/__NSM_BASE__/assets/app.js"></script>
<script>window.__NSM_BASE__ = "__NSM_BASE__";</script>"#;
        let rendered = render(html, "");
        assert!(rendered.contains(r#"src="/assets/app.js""#), "{rendered}");
        assert!(
            rendered.contains(r#"window.__NSM_BASE__ = "";"#),
            "{rendered}"
        );
    }

    #[test]
    fn the_placeholder_expands_to_the_configured_prefix() {
        let html = br#"<script src="/__NSM_BASE__/assets/app.js"></script>
<script>window.__NSM_BASE__ = "__NSM_BASE__";</script>"#;
        let rendered = render(html, "/subsc-admin");
        assert!(
            rendered.contains(r#"src="/subsc-admin/assets/app.js""#),
            "{rendered}"
        );
        assert!(
            rendered.contains(r#"window.__NSM_BASE__ = "/subsc-admin";"#),
            "{rendered}"
        );
    }

    /// Mirrors `index_response` without building an axum response.
    fn render(raw: &[u8], prefix: &str) -> String {
        let html = String::from_utf8_lossy(raw);
        html.replace(&format!("\"{BASE_PLACEHOLDER}\""), &format!("\"{prefix}\""))
            .replace(&format!("/{BASE_PLACEHOLDER}"), prefix)
    }
}
