// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

use std::{borrow::Cow, sync::Arc};

use axum::{
    body::Body, http::{HeaderValue, StatusCode, header, request}, response::{IntoResponse, Response},
};
use hyper::Method;

use crate::db::settings::AppSettings;

pub mod passthrough;

use passthrough::PassthroughTarget;

/// Built-in disguise page. It is deliberately generic: nothing in it hints at
/// proxying, subscriptions, or this application.
const BUILTIN_PAGE: &str = include_str!("404.html");

#[derive(Clone)]
pub enum Disguise {
    Html(DisguiseHtml),
    Passthrough(Arc<PassthroughTarget>),
}

impl Disguise {
    pub fn from_settings(page: Option<String>, settings: &AppSettings) -> Self {
        if let Some(target) = PassthroughTarget::from_settings(settings) {
            Self::Passthrough(Arc::new(target))
        } else {
            Self::Html(DisguiseHtml::new(page, settings))
        }
    }
    
    pub fn page(&self) -> &str {
        match self {
            Self::Html(html) => html.page(),
            Self::Passthrough(_) => "Disabled: passthrough disguise does not have a page",
        }
    }

    pub async fn generate_response(&self, parts: &request::Parts) -> Response {
        match self {
            Self::Html(html) => {
                if parts.method == Method::HEAD {
                    html.not_found_empty()
                } else {
                    html.not_found().await
                }
            },
            Self::Passthrough(target) => {
                match target.mirror(&parts.method, &parts.headers, &parts.uri).await {
                    Ok(resp) => resp.into_response(),
                    Err(err) => {
                        tracing::error!("Passthrough mirror failed, parts={parts:?}, {err:?}");
                        let mut response = if parts.method == Method::HEAD {
                            Response::new(Body::empty())
                        } else {
                            Response::new(Body::from(Cow::Borrowed(BUILTIN_PAGE)))
                        };
                        *response.status_mut() = StatusCode::NOT_FOUND;
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/html; charset=iso-8859-1"),
                        );
                        strip_identifying_headers(&mut response);
                        response
                    }
                }
            }
        }
    }
}

/// Serves the decoy content used for every failed public lookup.
#[derive(Clone)]
pub struct DisguiseHtml {
    page: Arc<String>,
    delay_ms: u64,
}

impl DisguiseHtml {
    pub fn new(page: Option<String>, settings: &AppSettings) -> Self {
        Self {
            page: Arc::new(page.unwrap_or_else(|| BUILTIN_PAGE.to_owned())),
            delay_ms: settings.failure_delay_ms,
        }
    }

    /// The raw page body, exposed for the settings preview endpoint.
    pub fn page(&self) -> &str {
        &self.page
    }

    /// 404 response with this application's fingerprints stripped.
    pub async fn not_found(&self) -> Response {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }
        self.build_response(StatusCode::NOT_FOUND)
    }

    /// 404 used for non-GET verbs, which never carry a body.
    pub fn not_found_empty(&self) -> Response {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_FOUND;
        strip_identifying_headers(&mut response);
        response
    }

    fn build_response(&self, status: StatusCode) -> Response {
        let mut response = Response::new(Body::from(self.page.as_str().to_owned()));
        *response.status_mut() = status;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=iso-8859-1"),
        );
        strip_identifying_headers(&mut response);
        response
    }
}

/// Removes headers that reveal the real stack.
pub fn strip_identifying_headers(response: &mut Response) {
    for name in ["x-powered-by", "server", "x-aspnet-version", "via"] {
        response.headers_mut().remove(name);
    }
}

/// Static file shipped with the binary, used as the default disguise page.
pub fn embedded_page() -> &'static str {
    BUILTIN_PAGE
}

impl IntoResponse for DisguiseHtml {
    fn into_response(self) -> Response {
        self.build_response(StatusCode::NOT_FOUND)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_page_is_html() {
        assert!(BUILTIN_PAGE.contains("<!doctype html>"));
    }

    #[test]
    fn custom_page_overrides_builtin() {
        let disguise = DisguiseHtml::new(
            Some("<html>custom</html>".to_owned()),
            &AppSettings::default(),
        );
        assert_eq!(disguise.page(), "<html>custom</html>");
    }

    #[test]
    fn settings_control_the_delay() {
        let settings = AppSettings {
            failure_delay_ms: 250,
            ..Default::default()
        };
        let disguise = DisguiseHtml::new(None, &settings);
        assert_eq!(disguise.delay_ms, 250);
    }

    #[tokio::test]
    async fn not_found_response_hides_identifying_headers() {
        let disguise = DisguiseHtml::new(None, &AppSettings::default());
        let mut response = disguise.not_found().await;
        response
            .headers_mut()
            .insert("x-powered-by", HeaderValue::from_static("php"));
        strip_identifying_headers(&mut response);

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(response.headers().get("x-powered-by").is_none());
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("text/html; charset=iso-8859-1")
        );
    }
}
