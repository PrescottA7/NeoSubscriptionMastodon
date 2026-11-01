use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

/// Serves the embedded admin UI. Unknown paths fall back to `index.html` so the
/// SPA router can handle deep links.
pub async fn serve_static(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) if path == "index.html" => index_response(content.data),
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut response = Response::new(Body::from(content.data.into_owned()));
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")),
            );
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            response
        }
        None => match Assets::get("index.html") {
            Some(content) => index_response(content.data),
            None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
        },
    }
}

fn index_response(content: std::borrow::Cow<'static, [u8]>) -> Response {
    let mut response = Response::new(Body::from(content.into_owned()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

/// True when the build produced an admin UI bundle.
pub fn has_assets() -> bool {
    Assets::get("index.html").is_some()
}
