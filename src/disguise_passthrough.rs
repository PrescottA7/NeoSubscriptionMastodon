// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! Mirrors the disguise page from an operator-supplied sample site.
//!
//! When enabled a failed public lookup is no longer answered with a static page:
//! the request is forwarded to the template URL and the sample response is
//! returned as-is. An optional IP/port pair overrides DNS, so the template's
//! hostname can be pointed at a specific origin while `Host` and SNI keep the
//! template's own hostname.

use std::sync::{Arc, OnceLock};

use axum::{
    body::{Body, Bytes},
    http::{
        header::{self, HeaderMap, HeaderName, HeaderValue},
        Method, StatusCode, Uri,
    },
    response::Response,
};
use http_body_util::BodyExt;
use rustls::{pki_types::ServerName, ClientConfig, RootCertStore};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::debug;

use crate::db::settings::AppSettings;

/// A stalled sample site must not hold a connection open forever.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
/// The disguise page is small; anything larger is treated as a failure.
const MAX_BODY: usize = 2 * 1024 * 1024;

/// Headers describing the previous hop, which must not be copied through.
const HOP_BY_HOP: [&str; 9] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "content-length",
];

/// A resolved upstream: where to dial, and what to ask for.
pub struct Target {
    /// Authority sent as `Host` and used as the TLS server name.
    pub authority: String,
    /// Host without the port, used for SNI.
    pub host: String,
    /// Dial address; honours the configured IP override.
    pub dial_addr: String,
    /// Origin-form request target, including the probe path.
    pub path_and_query: String,
}

impl Target {
    /// Builds the upstream from the settings, or `None` when unusable.
    ///
    /// The probe's path and query are preserved so a request for `/anything`
    /// still reaches the sample site's own 404 handler.
    pub fn from_settings(settings: &AppSettings, path_and_query: &str) -> Option<Self> {
        if !settings.passthrough_configured() {
            return None;
        }

        let template = url::Url::parse(&settings.disguise_template_url).ok()?;
        if template.scheme() != "https" {
            debug!(scheme = %template.scheme(), "disguise passthrough requires https");
            return None;
        }

        let host = template.host_str()?.to_owned();
        let port = if settings.disguise_template_port != 0 {
            settings.disguise_template_port
        } else {
            template.port_or_known_default()?
        };
        let authority = if port == 443 {
            host.clone()
        } else {
            format!("{host}:{port}")
        };

        // An explicit IP replaces DNS but keeps the hostname for Host and SNI.
        let dial_host = if settings.disguise_template_ip.is_empty() {
            host.clone()
        } else {
            settings.disguise_template_ip.clone()
        };
        let dial_addr = if dial_host.contains(':') {
            format!("[{dial_host}]:{port}")
        } else {
            format!("{dial_host}:{port}")
        };

        let uri: Uri = format!("https://{authority}{path_and_query}").parse().ok()?;

        Some(Self {
            authority,
            host,
            dial_addr,
            path_and_query: uri.path_and_query()?.as_str().to_owned(),
        })
    }

    /// Issues the upstream request, returning the sample response.
    pub async fn mirror(
        &self,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<MirrorResponse, String> {
        let request = build_request::<http_body_util::Empty<Bytes>>(
            &self.path_and_query,
            &self.authority,
            method,
            headers,
        )?;

        let response = tokio::time::timeout(TIMEOUT, self.send(request))
            .await
            .map_err(|_| "upstream timed out".to_owned())?
            .map_err(|error| format!("upstream request failed: {error}"))?;

        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .filter(|(name, _)| is_forwardable(name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();

        let body = tokio::time::timeout(TIMEOUT, response.into_body().collect())
            .await
            .map_err(|_| "upstream body timed out".to_owned())?
            .map_err(|error| format!("read upstream body: {error}"))?
            .to_bytes();

        if body.len() > MAX_BODY {
            return Err("upstream response too large".to_owned());
        }

        Ok(MirrorResponse { status, headers, body })
    }

    async fn send<B>(
        &self,
        request: hyper::Request<B>,
    ) -> Result<hyper::Response<hyper::body::Incoming>, String>
    where
        B: hyper::body::Body + Send + 'static,
        B::Data: Send,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        // The connector is keyed on the URI, but our URI carries the template's
        // hostname while we may need to dial a fixed IP. `HttpConnector` cannot
        // be told that, so the connection is opened here and handed to hyper.
        let stream = tokio::time::timeout(TIMEOUT, TcpStream::connect(&self.dial_addr))
            .await
            .map_err(|_| format!("connect {} timed out", self.dial_addr))?
            .map_err(|error| format!("connect {}: {error}", self.dial_addr))?;

        let server_name = ServerName::try_from(self.host.clone())
            .map_err(|_| format!("invalid TLS server name {}", self.host))?;
        let tls = tokio::time::timeout(TIMEOUT, tls_connector().connect(server_name, stream))
            .await
            .map_err(|_| "TLS handshake timed out".to_owned())?
            .map_err(|error| format!("TLS handshake: {error}"))?;

        let io = hyper_util::rt::TokioIo::new(tls);
        let (mut sender, connection) = tokio::time::timeout(
            TIMEOUT,
            hyper::client::conn::http1::handshake::<_, B>(io),
        )
        .await
        .map_err(|_| "pipeline handshake timed out".to_owned())?
        .map_err(|error| format!("pipeline handshake: {error}"))?;

        tokio::spawn(async move {
            if let Err(error) = connection.await {
                debug!(%error, "upstream connection closed with an error");
            }
        });

        // `Origin` form keeps the request-line at the path instead of turning it
        // into an absolute-form proxy request.
        sender
            .send_request(request)
            .await
            .map_err(|error| format!("send: {error}"))
    }
}

/// One proxied answer, reduced to what axum can return.
pub struct MirrorResponse {
    pub status: StatusCode,
    pub headers: Vec<(HeaderName, HeaderValue)>,
    pub body: Bytes,
}

/// Builds the outbound request.
///
/// The URI is deliberately origin-form (`/path?query`): hyper serialises it
/// as-is, and the template's hostname travels in the `Host` header instead, so
/// the probe never turns into an absolute-form proxy request.
fn build_request<B>(
    path_and_query: &str,
    authority: &str,
    method: &Method,
    headers: &HeaderMap,
) -> Result<hyper::Request<B>, String>
where
    B: Default,
{
    let mut builder = hyper::Request::builder()
        .method(method.clone())
        .uri(path_and_query)
        .header(header::HOST, authority);

    for (name, value) in headers.iter() {
        if is_forwardable(name) {
            builder = builder.header(name, value);
        }
    }

    builder
        .body(B::default())
        .map_err(|error| format!("build upstream request: {error}"))
}

/// True for headers that may be copied in either direction.
fn is_forwardable(name: &HeaderName) -> bool {
    let lower = name.as_str();
    !HOP_BY_HOP.contains(&lower) && lower != header::HOST.as_str()
}

/// Shared TLS stack. Roots come from the bundled Mozilla set plus, when
/// `SSL_CERT_FILE` points at a PEM bundle, the certificates in that file, so a
/// private CA in front of the sample site can be trusted.
fn tls_connector() -> TlsConnector {
    static CONFIG: OnceLock<Arc<ClientConfig>> = OnceLock::new();
    let config = CONFIG.get_or_init(|| {
        // sqlx pulls rustls in with `aws-lc-rs` while this crate asks for `ring`,
        // so the provider cannot be auto-detected and must be chosen explicitly.
        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        load_extra_roots(&mut roots);
        Arc::new(
            ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    });
    TlsConnector::from(config.clone())
}

/// Adds the certificates named by `SSL_CERT_FILE`, if the variable is set.
fn load_extra_roots(roots: &mut RootCertStore) {
    let Some(path) = std::env::var_os("SSL_CERT_FILE") else {
        return;
    };
    let Ok(pem) = std::fs::read(&path) else {
        debug!(path = %path.to_string_lossy(), "SSL_CERT_FILE is not readable");
        return;
    };

    let mut added = 0;
    for block in rustls_pemfile_iterator(&pem) {
        if roots.add(block).is_ok() {
            added += 1;
        }
    }
    debug!(path = %path.to_string_lossy(), added, "loaded extra TLS roots");
}

/// Yields every `CERTIFICATE` block in a PEM bundle.
fn rustls_pemfile_iterator(pem: &[u8]) -> Vec<rustls::pki_types::CertificateDer<'static>> {
    static BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    static END: &str = "-----END CERTIFICATE-----";

    let text = String::from_utf8_lossy(pem);
    let mut certificates = Vec::new();
    let mut rest = text.as_ref();
    while let Some(start) = rest.find(BEGIN) {
        let Some(end) = rest[start..].find(END) else {
            break;
        };
        let body = &rest[start + BEGIN.len()..start + end];
        let der = body
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        if let Ok(bytes) = base64_decode(&der) {
            certificates.push(rustls::pki_types::CertificateDer::from(bytes));
        }
        rest = &rest[start + end + END.len()..];
    }
    certificates
}

/// Minimal standard-alphabet base64 decoder for PEM bodies.
fn base64_decode(input: &str) -> Result<Vec<u8>, ()> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let Some(value) = ALPHABET.iter().position(|candidate| *candidate == byte) else {
            return Err(());
        };
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok(out)
}

/// Converts a mirrored answer into a response with fingerprints stripped.
pub fn to_response(mirror: MirrorResponse) -> Response {
    let mut response = Response::new(Body::from(mirror.body));
    *response.status_mut() = mirror.status;
    for (name, value) in mirror.headers {
        response.headers_mut().insert(name, value);
    }
    crate::disguise::strip_identifying_headers(&mut response);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AppSettings {
        AppSettings {
            disguise_passthrough: true,
            disguise_template_url: "https://sample.example.com/".to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn disabled_passthrough_has_no_target() {
        let mut settings = settings();
        settings.disguise_passthrough = false;
        assert!(Target::from_settings(&settings, "/x").is_none());
    }

    #[test]
    fn missing_url_has_no_target() {
        let mut settings = settings();
        settings.disguise_template_url.clear();
        assert!(Target::from_settings(&settings, "/x").is_none());
    }

    #[test]
    fn the_probe_path_is_kept() {
        let target = Target::from_settings(&settings(), "/anything?q=1").expect("target");
        assert_eq!(target.path_and_query, "/anything?q=1");
        assert_eq!(target.authority, "sample.example.com");
        assert_eq!(target.dial_addr, "sample.example.com:443");
    }

    #[test]
    fn a_configured_port_changes_the_dial_port_only() {
        let mut settings = settings();
        settings.disguise_template_port = 8443;
        let target = Target::from_settings(&settings, "/").expect("target");
        assert_eq!(target.authority, "sample.example.com:8443");
        assert_eq!(target.dial_addr, "sample.example.com:8443");
        assert_eq!(target.host, "sample.example.com");
    }

    #[test]
    fn a_configured_ip_replaces_dns_but_not_the_host() {
        let mut settings = settings();
        settings.disguise_template_ip = "203.0.113.7".to_owned();
        let target = Target::from_settings(&settings, "/").expect("target");
        assert_eq!(target.dial_addr, "203.0.113.7:443");
        assert_eq!(target.authority, "sample.example.com");
        assert_eq!(target.host, "sample.example.com");
    }

    #[test]
    fn an_ipv6_override_is_bracketed() {
        let mut settings = settings();
        settings.disguise_template_ip = "2001:db8::1".to_owned();
        let target = Target::from_settings(&settings, "/").expect("target");
        assert_eq!(target.dial_addr, "[2001:db8::1]:443");
    }

    #[test]
    fn plain_http_is_refused() {
        let mut settings = settings();
        settings.disguise_template_url = "http://sample.example.com/".to_owned();
        assert!(Target::from_settings(&settings, "/").is_none());
    }

    #[test]
    fn hop_by_hop_headers_are_dropped() {
        assert!(!is_forwardable(&header::CONNECTION));
        assert!(!is_forwardable(&header::TRANSFER_ENCODING));
        assert!(!is_forwardable(&header::HOST));
        assert!(is_forwardable(&header::ACCEPT));
    }

    #[test]
    fn the_template_host_wins_over_the_client_host() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("probe.example.net"));
        let request: hyper::Request<http_body_util::Empty<Bytes>> =
            build_request("/admin", "sample.example.com", &Method::GET, &headers)
                .expect("request");
        assert_eq!(
            request.headers().get(header::HOST).and_then(|v| v.to_str().ok()),
            Some("sample.example.com")
        );
        // Origin form, not an absolute-form proxy request.
        assert_eq!(request.uri().to_string(), "/admin");
    }
}
