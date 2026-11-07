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
//! returned as-is. An optional address/port pair overrides DNS, so the template's
//! hostname can be pointed at a specific origin while `Host` and SNI keep the
//! template's own hostname.

use std::sync::{Arc, OnceLock};

use anyhow::anyhow;
use axum::{
    body::{Body, Bytes}, http::{
        Method, StatusCode,
        header::{self, HeaderMap, HeaderName, HeaderValue},
    }, response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use hyper::Uri;
use rustls::{ClientConfig, RootCertStore, pki_types::ServerName};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tracing::{error, info, warn};
use url::Url;

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
pub struct PassthroughTarget {
    /// Authority sent as `Host` and used as the TLS server name.
    pub authority: String,
    /// Host without the port, used for SNI.
    pub host: String,
    /// Dial address; honours the configured address override.
    pub dial_addr: String,
    pub path_template: String,
}

impl PassthroughTarget {
    /// Builds the upstream from the settings, or `None` when unusable.
    ///
    /// The probe's path and query are preserved so a request for `/anything`
    /// still reaches the sample site's own 404 handler.
    pub fn from_settings(settings: &AppSettings) -> Option<Self> {
        if !settings.passthrough_configured() {
            return None;
        }

        let template = Url::parse(&settings.disguise_template_url).ok()?;
        if template.scheme() != "https" {
            error!(scheme = %template.scheme(), "disguise passthrough requires https");
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

        // An explicit address replaces DNS but keeps the hostname for Host and
        // SNI. It may be an IP or a name such as `host.docker.internal`.
        let override_host = settings.disguise_template_ip.trim();
        let dial_host = if override_host.is_empty() {
            host.clone()
        } else {
            override_host
                .trim_matches(|c| c == '[' || c == ']')
                .to_owned()
        };
        let dial_addr = if let Some(stripped) = dial_host.strip_prefix('[') {
            let inner = stripped.strip_suffix(']').unwrap_or(stripped);
            format!("[{inner}]:{port}")
        } else if dial_host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{dial_host}]:{port}")
        } else {
            format!("{dial_host}:{port}")
        };

        Some(Self {
            authority,
            host,
            dial_addr,
            path_template: settings.disguise_template_url.parse::<Uri>().ok()?
                .path_and_query().map_or_else(|| "/".to_owned(), |pq| pq.as_str().to_owned()),
        })
    }

    fn parse_template(&self, origin_uri: &Uri) -> String {
        let path = origin_uri.path();
        let query = origin_uri.query().map_or_else(|| "".to_owned(), |q| "?".to_owned() + q);
        let pathpad = if path == "/" { "/" } else { "" };
        let pathtrimstart = path.trim_start_matches('/');

        let mut result = self.path_template.clone();
        result = result.replace("{path}", path);
        result = result.replace("{query}", &query);
        result = result.replace("{pathpad}", pathpad);
        result = result.replace("{pathtrimstart}", pathtrimstart);

        result
    }

    /// Issues the upstream request, returning the sample response.
    pub async fn mirror(
        &self,
        method: &Method,
        headers: &HeaderMap,
        origin_uri: &Uri
    ) -> anyhow::Result<MirrorResponse> {
        let access_uri = self.parse_template(origin_uri).parse::<Uri>()?;
        let request = build_request::<http_body_util::Empty<Bytes>>(
            &(access_uri.path().to_owned() + access_uri.query().unwrap_or("")),
            &self.authority,
            method,
            headers,
        )?;

        let response = tokio::time::timeout(TIMEOUT, self.send(request))
            .await
            .map_err(|_| anyhow!("upstream timed out"))?
            .map_err(|error| anyhow!("upstream request failed: {error}"))?;

        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .filter(|(name, _)| is_forwardable(name))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();

        let body = tokio::time::timeout(TIMEOUT, response.into_body().collect())
            .await
            .map_err(|_| anyhow!("upstream body timed out"))?
            .map_err(|error| anyhow!("read upstream body: {error}"))?
            .to_bytes();

        if body.len() > MAX_BODY {
            return Err(anyhow!("upstream response too large"));
        }

        Ok(MirrorResponse {
            status,
            headers,
            body,
        })
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
        // hostname while we may need to dial a fixed address. `HttpConnector`
        // cannot be told that, so the connection is opened here and handed to
        // hyper. `TcpStream::connect` resolves names, so the override may be a
        // hostname such as `host.docker.internal` as well as an IP.
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
        let (mut sender, connection) =
            tokio::time::timeout(TIMEOUT, hyper::client::conn::http1::handshake::<_, B>(io))
                .await
                .map_err(|_| "pipeline handshake timed out".to_owned())?
                .map_err(|error| format!("pipeline handshake: {error}"))?;

        tokio::spawn(async move {
            if let Err(error) = connection.await {
                warn!(%error, "upstream connection closed with an error");
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

impl IntoResponse for MirrorResponse {
    /// Converts a mirrored answer into a response with fingerprints stripped.
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = self.status;
        for (name, value) in self.headers {
            response.headers_mut().insert(name, value);
        }
        crate::disguise::strip_identifying_headers(&mut response);
        response
    }
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
) -> anyhow::Result<hyper::Request<B>>
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
        .map_err(|error| anyhow!("build upstream request: {error}"))
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
        warn!(path = %path.to_string_lossy(), "SSL_CERT_FILE is not readable");
        return;
    };

    let mut added = 0;
    for block in pem_certificates(&pem) {
        if roots.add(block).is_ok() {
            added += 1;
        }
    }
    info!(path = %path.to_string_lossy(), added, "loaded extra TLS roots");
}

/// Yields every `CERTIFICATE` block in a PEM bundle, skipping other sections.
/// Malformed blocks are dropped rather than failing the whole bundle, and
/// `RootCertStore::add` rejects the ones that are not valid DER anyway.
fn pem_certificates(pem: &[u8]) -> Vec<rustls::pki_types::CertificateDer<'static>> {
    rustls_pemfile::certs(&mut &pem[..])
        .filter_map(Result::ok)
        .collect()
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
        assert!(PassthroughTarget::from_settings(&settings).is_none());
    }

    #[test]
    fn a_configured_port_changes_the_dial_port_only() {
        let mut settings = settings();
        settings.disguise_template_port = 8443;
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.authority, "sample.example.com:8443");
        assert_eq!(target.dial_addr, "sample.example.com:8443");
        assert_eq!(target.host, "sample.example.com");
    }

    #[test]
    fn a_configured_ip_replaces_dns_but_not_the_host() {
        let mut settings = settings();
        settings.disguise_template_ip = "203.0.113.7".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.dial_addr, "203.0.113.7:443");
        assert_eq!(target.authority, "sample.example.com");
        assert_eq!(target.host, "sample.example.com");
    }

    #[test]
    fn a_configured_hostname_replaces_dns_but_not_the_host() {
        let mut settings = settings();
        settings.disguise_template_ip = "host.docker.internal".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.dial_addr, "host.docker.internal:443");
        assert_eq!(target.authority, "sample.example.com");
        assert_eq!(target.host, "sample.example.com");
    }

    #[test]
    fn an_ipv6_override_is_bracketed() {
        let mut settings = settings();
        settings.disguise_template_ip = "2001:db8::1".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.dial_addr, "[2001:db8::1]:443");
    }

    #[test]
    fn url_path_query_placeholders_are_replaced() {
        let mut settings = settings();
        settings.disguise_template_url = "https://sample.example.com/test{path}/hello{query}".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.parse_template(&Uri::from_static("https://sample.example.com/foo/bar?x=1&y=2")), "/test/foo/bar/hello?x=1&y=2");
    }

    #[test]
    fn url_query_placeholder_query_absent() {
        let mut settings = settings();
        settings.disguise_template_url = "https://sample.example.com/test{path}/hello{query}".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.parse_template(&Uri::from_static("https://sample.example.com/foo/bar")), "/test/foo/bar/hello");
    }

    #[test]
    fn url_query_placeholder_path_pad() {
        let mut settings = settings();
        settings.disguise_template_url = "https://sample.example.com/test{pathpad}hello{query}".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.parse_template(&Uri::from_static("https://sample.example.com/")), "/test/hello");
    }

    #[test]
    fn url_query_placeholder_path_trim_start() {
        let mut settings = settings();
        settings.disguise_template_url = "https://sample.example.com/test/{pathtrimstart}hello{query}".to_owned();
        let target = PassthroughTarget::from_settings(&settings).expect("target");
        assert_eq!(target.parse_template(&Uri::from_static("https://sample.example.com/foo/bar")), "/test/foo/barhello");
    }

    #[test]
    fn plain_http_is_refused() {
        let mut settings = settings();
        settings.disguise_template_url = "http://sample.example.com/".to_owned();
        assert!(PassthroughTarget::from_settings(&settings).is_none());
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
            build_request("/admin", "sample.example.com", &Method::GET, &headers).expect("request");
        assert_eq!(
            request
                .headers()
                .get(header::HOST)
                .and_then(|v| v.to_str().ok()),
            Some("sample.example.com")
        );
        // Origin form, not an absolute-form proxy request.
        assert_eq!(request.uri().to_string(), "/admin");
    }

    /// Builds a PEM bundle from `(label, body)` pairs, encoding each body as base64.
    fn pem_bundle(sections: &[(&str, &[u8])]) -> String {
        use base64::Engine;

        let mut out = String::new();
        for (label, bytes) in sections {
            out.push_str(&format!("-----BEGIN {label}-----\n"));
            out.push_str(&base64::engine::general_purpose::STANDARD.encode(bytes));
            out.push_str(&format!("\n-----END {label}-----\n"));
        }
        out
    }

    #[test]
    fn every_certificate_block_is_collected_and_other_sections_are_skipped() {
        let pem = pem_bundle(&[
            ("CERTIFICATE", &[1, 2, 3]),
            ("PRIVATE KEY", &[9, 9, 9]),
            ("CERTIFICATE", &[4, 5, 6, 7]),
        ]);

        let certificates = pem_certificates(pem.as_bytes());
        assert_eq!(certificates.len(), 2);
        assert_eq!(certificates[0].as_ref(), [1, 2, 3]);
        assert_eq!(certificates[1].as_ref(), [4, 5, 6, 7]);
    }

    #[test]
    fn a_bundle_without_certificates_yields_nothing() {
        assert!(pem_certificates(b"").is_empty());
        assert!(pem_certificates(b"this is not a pem file").is_empty());

        let keys_only = pem_bundle(&[("PRIVATE KEY", &[1, 2, 3])]);
        assert!(pem_certificates(keys_only.as_bytes()).is_empty());
    }

    #[test]
    fn a_malformed_block_does_not_discard_the_rest_of_the_bundle() {
        let mut pem =
            String::from("-----BEGIN CERTIFICATE-----\n!!!not base64!!!\n-----END CERTIFICATE-----\n");
        pem.push_str(&pem_bundle(&[("CERTIFICATE", &[8, 9])]));

        let certificates = pem_certificates(pem.as_bytes());
        assert_eq!(certificates.len(), 1);
        assert_eq!(certificates[0].as_ref(), [8, 9]);
    }

    #[test]
    fn folded_bodies_are_decoded() {
        // Real bundles wrap base64 bodies at 64 characters; the decoded bytes must match exactly.
        use base64::Engine;

        let bytes: Vec<u8> = (0..=255u8).collect();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let folded: String = encoded
            .as_bytes()
            .chunks(64)
            .map(|chunk| format!("{}\n", std::str::from_utf8(chunk).expect("ascii")))
            .collect();
        let pem = format!("-----BEGIN CERTIFICATE-----\n{folded}-----END CERTIFICATE-----\n");

        let certificates = pem_certificates(pem.as_bytes());
        assert_eq!(certificates.len(), 1);
        assert_eq!(certificates[0].as_ref(), bytes.as_slice());
    }
}
