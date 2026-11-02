use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::db::settings::AppSettings;

/// Built-in disguise page. It is deliberately generic: nothing in it hints at
/// proxying, subscriptions, or this application.
const BUILTIN_PAGE: &str = include_str!("../404.html");

/// Serves the decoy content used for every failed public lookup.
#[derive(Clone)]
pub struct Disguise {
    page: Arc<String>,
    delay_ms: u64,
}

impl Disguise {
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

impl IntoResponse for Disguise {
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
        let disguise = Disguise::new(Some("<html>custom</html>".to_owned()), &AppSettings::default());
        assert_eq!(disguise.page(), "<html>custom</html>");
    }

    #[test]
    fn settings_control_the_delay() {
        let settings = AppSettings {
            failure_delay_ms: 250,
            ..Default::default()
        };
        let disguise = Disguise::new(None, &settings);
        assert_eq!(disguise.delay_ms, 250);
    }

    #[tokio::test]
    async fn not_found_response_hides_identifying_headers() {
        let disguise = Disguise::new(None, &AppSettings::default());
        let mut response = disguise.not_found().await;
        response.headers_mut().insert("x-powered-by", HeaderValue::from_static("php"));
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
