// Copyright (C) 2026 Prescott Anderson
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of NSM (NeoSubscriptionMastodon), licensed under the
// GNU Affero General Public License v3.0 or later. See the LICENSE file
// in the project root for the full text.

//! End-to-end tests over the real router: setup, admin API, subscription
//! rendering, and the disguise page.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use nsm::{AppState, config::AppConfig, db, protocol::VlessConfig};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

struct Harness {
    _temp: TempDir,
    router: axum::Router,
}

async fn harness() -> Harness {
    let temp = TempDir::new().expect("temp dir");
    let config = AppConfig::test_config(temp.path());
    let router = nsm::build_app(config).await.expect("build app");
    Harness {
        _temp: temp,
        router,
    }
}

/// Harness with host routing and an admin path prefix configured.
async fn split_harness() -> Harness {
    let temp = TempDir::new().expect("temp dir");
    let mut config = AppConfig::test_config(temp.path());
    config.subscription_host = Some("sub.example.com".to_owned());
    config.admin_host = Some("panel.example.com".to_owned());
    config.admin_path_prefix = Some("/subsc-admin".to_owned());
    config.subscription_prefix = Some("/link".to_owned());
    let router = nsm::build_app(config).await.expect("build app");
    Harness {
        _temp: temp,
        router,
    }
}

/// Request carrying an explicit `Host`, for host-routing tests.
fn host_request(method: Method, uri: &str, host: &str, body: Option<Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, host);

    match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&value).expect("serialize")))
            .expect("request"),
        None => builder.body(Body::empty()).expect("request"),
    }
}

/// Sends a request and returns status, body, and the session cookie if any.
async fn send(harness: &Harness, request: Request<Body>) -> (StatusCode, String, Option<String>) {
    let response: Response = harness
        .router
        .clone()
        .oneshot(request)
        .await
        .expect("request");
    let status = response.status();
    let headers = response.headers().clone();
    let cookie = headers
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or_default().to_owned());
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("body");
    LAST_HEADERS.with(|slot| *slot.borrow_mut() = Some(headers));
    (status, String::from_utf8_lossy(&body).into_owned(), cookie)
}

thread_local! {
    static LAST_HEADERS: std::cell::RefCell<Option<axum::http::HeaderMap>> =
        const { std::cell::RefCell::new(None) };
}

/// Full response headers from the most recent [`send`] on this thread.
fn last_headers() -> axum::http::HeaderMap {
    LAST_HEADERS.with(|slot| slot.borrow().clone().expect("headers recorded"))
}

fn json_request(method: Method, uri: &str, body: Value, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }

    builder
        .body(Body::from(serde_json::to_vec(&body).expect("serialize")))
        .expect("request")
}

fn get_request(uri: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(Method::GET).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    builder.body(Body::empty()).expect("request")
}

/// Completes the first-run setup and returns the session cookie.
async fn setup(harness: &Harness) -> String {
    let (status, _, cookie) = send(
        harness,
        json_request(
            Method::POST,
            "/api/auth/setup",
            json!({ "username": "admin", "password": "a-very-long-password" }),
            None,
        ),
    )
    .await;

    // Only an `(Extension, Json)` pair changes the status away from 200.
    let cookie = cookie.expect("session cookie");
    assert!(status.is_success(), "unexpected setup status: {status}");
    cookie
}

fn vless_params() -> Value {
    let inner = serde_json::to_value(VlessConfig {
        tls: nsm::protocol::TlsConfig {
            enabled: true,
            server_name: Some("cdn.example.com".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    })
    .expect("serialize");

    let mut object = inner.as_object().cloned().expect("object");
    object.insert("protocol".to_owned(), json!("vless"));
    Value::Object(object)
}

/// Creates a user and returns its id.
async fn create_user(harness: &Harness, cookie: &str, remark: &str) -> i64 {
    let (status, body, _) = send(
        harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": remark, "enabled": true }),
            Some(cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "user creation failed: {body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    created["user"]["id"].as_i64().expect("user id")
}

#[tokio::test]
async fn health_endpoint_is_public() {
    let harness = harness().await;
    let (status, body, _) = send(&harness, get_request("/api/health", None)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("ok"));
}

#[tokio::test]
async fn setup_is_offered_once() {
    let harness = harness().await;

    let (status, body, _) = send(&harness, get_request("/api/auth/setup-status", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"needs_setup\":true"));

    setup(&harness).await;

    let (_, body, _) = send(&harness, get_request("/api/auth/setup-status", None)).await;
    assert!(body.contains("\"needs_setup\":false"));

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/auth/setup",
            json!({ "username": "other", "password": "another-long-password" }),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn admin_routes_require_a_session() {
    let harness = harness().await;
    setup(&harness).await;

    let (status, _, _) = send(&harness, get_request("/api/users", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(
        &harness,
        get_request("/api/nodes", Some("nsm_session=bogus")),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn user_and_node_crud_round_trip() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "Alice" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    let token = created["token"].as_str().expect("token").to_owned();
    let user_id = created["user"]["id"].as_i64().expect("user id");
    assert!(
        created["subscription_path"]
            .as_str()
            .expect("path")
            .starts_with("/s/")
    );

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "Tokyo",
                "server": "example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "node creation failed: {body}");
    let node: Value = serde_json::from_str(&body).expect("json");
    let node_id = node["id"].as_i64().expect("id");

    let (status, body, _) = send(&harness, get_request("/api/nodes", Some(&cookie))).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("vless"));

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::PUT,
            &format!("/api/nodes/{node_id}"),
            json!({
                "name": "Tokyo 2",
                "server": "example.com",
                "port": 8443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 1,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The subscription must now serve a rendered payload for this user.
    let (status, body, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.is_empty());
}

#[tokio::test]
async fn node_enabled_toggle_keeps_protocol_params() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "Alice" }),
            Some(&cookie),
        ),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let token = created["token"].as_str().expect("token").to_owned();
    let user_id = created["user"]["id"].as_i64().expect("user id");

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "Tokyo",
                "server": "example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "node creation failed: {body}");
    let node: Value = serde_json::from_str(&body).expect("json");
    let node_id = node["id"].as_i64().expect("id");

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::PATCH,
            &format!("/api/nodes/{node_id}/enabled"),
            json!({ "enabled": false }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "toggle failed: {body}");
    let toggled: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(toggled["enabled"], json!(false));
    assert_eq!(
        toggled["params"], node["params"],
        "params must be preserved"
    );

    // A disabled node stops being published.
    let (status, _, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::PATCH,
            &format!("/api/nodes/{node_id}/enabled"),
            json!({ "enabled": true }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn node_enabled_toggle_rejects_unknown_node() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::PATCH,
            "/api/nodes/9999/enabled",
            json!({ "enabled": false }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_token_returns_the_disguise_page() {
    let harness = harness().await;
    setup(&harness).await;

    let (status, body, _) = send(&harness, get_request("/s/not-a-real-token", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("404"));
    assert!(!body.contains("\"error\""));
}

#[tokio::test]
async fn unknown_api_path_returns_json_error() {
    let harness = harness().await;
    setup(&harness).await;

    let (status, body, _) = send(&harness, get_request("/api/nope", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("error"));
}

#[tokio::test]
async fn disabled_user_loses_access_immediately() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (_, body, _) = send(
        &harness,
        json_request(Method::POST, "/api/users", json!({}), Some(&cookie)),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let token = created["token"].as_str().expect("token").to_owned();
    let user_id = created["user"]["id"].as_i64().expect("id");

    send(
        &harness,
        json_request(
            Method::PATCH,
            &format!("/api/users/{user_id}"),
            json!({ "enabled": false }),
            Some(&cookie),
        ),
    )
    .await;

    let (status, body, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("404"));
}

#[tokio::test]
async fn token_reset_revokes_the_old_link() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (_, body, _) = send(
        &harness,
        json_request(Method::POST, "/api/users", json!({}), Some(&cookie)),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let old_token = created["token"].as_str().expect("token").to_owned();
    let user_id = created["user"]["id"].as_i64().expect("id");

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            &format!("/api/users/{user_id}/reset-link"),
            json!({}),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reset: Value = serde_json::from_str(&body).expect("json");
    let new_token = reset["token"].as_str().expect("token");
    assert_ne!(old_token, new_token);

    let (status, _, _) = send(&harness, get_request(&format!("/s/{old_token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn settings_control_the_subscription_prefix() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "prefix-user" }),
            Some(&cookie),
        ),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let token = created["token"].as_str().expect("token").to_owned();
    let user_id = created["user"]["id"].as_i64().expect("user id");

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::PUT,
            "/api/settings",
            json!({ "subscription_prefix": "/subscribe" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The user has no nodes yet, so the disguised 404 is the correct answer and
    // proves the new prefix is routed to the subscription handler.
    let (status, _, _) = send(&harness, get_request(&format!("/subscribe/{token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // With a node in place the new prefix serves content while the old one stays dead.
    send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "Tokyo",
                "server": "example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;

    let (status, body, _) = send(&harness, get_request(&format!("/subscribe/{token}"), None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, _, _) = send(&harness, get_request(&format!("/s/{token}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn passthrough_settings_round_trip() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::PUT,
            "/api/settings",
            json!({
                "disguise_passthrough": true,
                "disguise_template_url": "https://sample.example.com/",
                "disguise_template_ip": "203.0.113.7",
                "disguise_template_port": 8443,
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let settings: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(settings["disguise_passthrough"], true);
    assert_eq!(
        settings["disguise_template_url"],
        "https://sample.example.com/"
    );
    assert_eq!(settings["disguise_template_ip"], "203.0.113.7");
    assert_eq!(settings["disguise_template_port"], 8443);

    // A probe still answers 404 when the sample site cannot be reached.
    let (status, body, _) = send(&harness, get_request("/some/probe", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body.contains("404"), "{body}");
}

#[tokio::test]
async fn passthrough_rejects_a_plain_http_template() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::PUT,
            "/api/settings",
            json!({
                "disguise_passthrough": true,
                "disguise_template_url": "http://sample.example.com/",
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("disguise_template_url"), "{body}");
}

#[tokio::test]
async fn subscription_host_hides_the_admin_area() {
    let harness = split_harness().await;

    // The admin prefix and API are unreachable from the subscription host.
    for path in [
        "/subsc-admin",
        "/subsc-admin/login",
        "/subsc-admin/api/health",
        "/api/health",
    ] {
        let (status, body, _) = send(
            &harness,
            host_request(Method::GET, path, "sub.example.com", None),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path {path}: {body}");
        assert!(
            body.contains("404"),
            "path {path} should be disguised: {body}"
        );
    }
}

#[tokio::test]
async fn admin_host_hides_the_subscription_paths() {
    let harness = split_harness().await;

    let (status, body, _) = send(
        &harness,
        host_request(
            Method::GET,
            "/link/unknown-token",
            "panel.example.com",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body.contains("404"), "expected the disguise page: {body}");
}

#[tokio::test]
async fn the_admin_prefix_serves_the_api_and_login() {
    let harness = split_harness().await;

    let (status, body, _) = send(
        &harness,
        host_request(
            Method::GET,
            "/subsc-admin/api/health",
            "panel.example.com",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("\"ok\""), "{body}");

    // An unauthenticated admin call still answers 401 rather than the disguise.
    let (status, body, _) = send(
        &harness,
        host_request(
            Method::GET,
            "/subsc-admin/api/auth/me",
            "panel.example.com",
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
}

#[tokio::test]
async fn setup_works_under_the_admin_prefix() {
    let harness = split_harness().await;

    let (status, body, cookie) = send(
        &harness,
        host_request(
            Method::POST,
            "/subsc-admin/api/auth/setup",
            "panel.example.com",
            Some(json!({ "username": "admin", "password": "a-very-long-password" })),
        ),
    )
    .await;

    assert!(status.is_success(), "setup failed: {body}");
    let cookie = cookie.expect("session cookie");
    // The cookie must be scoped to the admin prefix, not the whole host.
    let set_cookie = last_headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(set_cookie.contains("Path=/subsc-admin"), "{set_cookie}");
    assert!(!cookie.is_empty());
}

#[tokio::test]
async fn the_pinned_subscription_prefix_is_used_in_links() {
    let harness = split_harness().await;

    // Create the admin first; the setup route lives under the admin prefix.
    send(
        &harness,
        host_request(
            Method::POST,
            "/subsc-admin/api/auth/setup",
            "panel.example.com",
            Some(json!({ "username": "admin", "password": "a-very-long-password" })),
        ),
    )
    .await;

    let (_, _, cookie) = send(
        &harness,
        host_request(
            Method::POST,
            "/subsc-admin/api/auth/login",
            "panel.example.com",
            Some(json!({ "username": "admin", "password": "a-very-long-password" })),
        ),
    )
    .await;
    let cookie = cookie.expect("session cookie");

    let (status, body, _) = send(&harness, {
        let mut request = host_request(
            Method::POST,
            "/subsc-admin/api/users",
            "panel.example.com",
            Some(json!({ "remark": "alice" })),
        );
        request
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("cookie"));
        request
    })
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    // The pinned prefix wins over the database default.
    assert_eq!(
        created["subscription_path"],
        format!("/link/{}", created["token"].as_str().expect("token")),
        "{body}"
    );
}

#[tokio::test]
async fn the_pinned_prefix_is_reported_as_locked() {
    let harness = split_harness().await;

    send(
        &harness,
        host_request(
            Method::POST,
            "/subsc-admin/api/auth/setup",
            "panel.example.com",
            Some(json!({ "username": "admin", "password": "a-very-long-password" })),
        ),
    )
    .await;

    let (_, _, cookie) = send(
        &harness,
        host_request(
            Method::POST,
            "/subsc-admin/api/auth/login",
            "panel.example.com",
            Some(json!({ "username": "admin", "password": "a-very-long-password" })),
        ),
    )
    .await;
    let cookie = cookie.expect("session cookie");

    let (status, body, _) = send(&harness, {
        let mut request = host_request(
            Method::GET,
            "/subsc-admin/api/settings",
            "panel.example.com",
            None,
        );
        request
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("cookie"));
        request
    })
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let settings: Value = serde_json::from_str(&body).expect("json");
    // The UI locks the field when the prefix comes from the config file.
    assert_eq!(settings["subscription_prefix_pinned"], true);
    // And it shows the pinned value, never the stale database one.
    let (_, body, _) = send(&harness, {
        let mut request = host_request(
            Method::POST,
            "/subsc-admin/api/users",
            "panel.example.com",
            Some(json!({ "remark": "bob" })),
        );
        request
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("cookie"));
        request
    })
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    assert!(
        created["subscription_path"]
            .as_str()
            .expect("path")
            .starts_with("/link/"),
        "{body}"
    );
}

#[tokio::test]
async fn an_unrecognised_host_gets_the_disguise_page() {
    let harness = split_harness().await;

    // Neither configured host, so nothing is served — not even the health probe.
    for path in ["/subsc-admin/api/health", "/link/anything", "/"] {
        let (status, body, _) = send(
            &harness,
            host_request(Method::GET, path, "other.example.net", None),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path {path}: {body}");
        assert!(body.contains("404"), "path {path}: {body}");
    }
}

#[tokio::test]
async fn mixed_mode_keeps_the_historical_layout() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    // No hosts configured: the API answers at the root and probes stay disguised.
    let (status, body, _) = send(&harness, get_request("/api/health", None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body, _) = send(&harness, get_request("/admin", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert!(body.contains("404"), "{body}");

    assert!(!cookie.is_empty());
}

#[tokio::test]
async fn a_user_can_be_created_with_a_chosen_subscription_path() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "alice", "token": "alice-link" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(created["token"], "alice-link", "{body}");
    assert_eq!(created["user"]["token"], "alice-link", "{body}");
    assert_eq!(created["subscription_path"], "/s/alice-link", "{body}");
}

#[tokio::test]
async fn the_subscription_path_is_visible_and_editable() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "alice" }),
            Some(&cookie),
        ),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let user_id = created["user"]["id"].as_i64().expect("id");
    let generated = created["token"].as_str().expect("token").to_owned();
    assert!(!generated.is_empty());

    // The list endpoint exposes the slug, so the link is always retrievable.
    let (status, body, _) = send(&harness, get_request("/api/users", Some(&cookie))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let listed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(listed[0]["token"], generated, "{body}");

    // Renaming the slug moves the link.
    let (status, body, _) = send(
        &harness,
        json_request(
            Method::PATCH,
            &format!("/api/users/{user_id}"),
            json!({ "token": "my-new-link" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).expect("json")["token"],
        "my-new-link"
    );

    let (status, _, _) = send(&harness, get_request("/s/my-new-link", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "new link should resolve");
    let (status, _, _) = send(&harness, get_request(&format!("/s/{generated}"), None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "old link must be gone");

    let (status, _, _) = send(&harness, get_request("/s/does-not-exist", None)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn duplicate_subscription_paths_are_rejected() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "first", "token": "shared" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "second", "token": "shared" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("already taken"), "{body}");
}

#[tokio::test]
async fn subscription_paths_reject_url_syntax() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    for bad in ["with space", "with/slash", "with?query"] {
        let (status, body, _) = send(
            &harness,
            json_request(
                Method::POST,
                "/api/users",
                json!({ "remark": "x", "token": bad }),
                Some(&cookie),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
        assert!(
            body.contains("token") || body.contains("Subscription path"),
            "{bad}: {body}"
        );
    }
}

#[tokio::test]
async fn admin_api_cannot_be_shadowed_by_the_prefix() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::PUT,
            "/api/settings",
            json!({ "subscription_prefix": "/api" }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn node_render_endpoint_supports_every_core() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "render-user").await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "Hy2",
                "server": "hy2.example.com",
                "port": 443,
                "params": { "protocol": "hysteria2", "tls": { "enabled": true } },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    let node: Value = serde_json::from_str(&body).expect("json");
    let node_id = node["id"].as_i64().expect("id");

    for core in ["uri", "sing-box", "mihomo", "xray", "hysteria2"] {
        let (status, body, _) = send(
            &harness,
            get_request(
                &format!("/api/nodes/{node_id}/render?core={core}"),
                Some(&cookie),
            ),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "core {core} failed: {body}");
        assert!(body.contains("\"content\""), "core {core} payload: {body}");
    }
}

#[tokio::test]
async fn password_change_revokes_sessions() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, _, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/auth/password",
            json!({
                "current_password": "a-very-long-password",
                "new_password": "an-even-longer-password",
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, _) = send(&harness, get_request("/api/auth/me", Some(&cookie))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_clears_the_cookie_and_revokes_the_session() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, _, _) = send(
        &harness,
        json_request(Method::POST, "/api/auth/logout", json!({}), Some(&cookie)),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The clearing cookie must expire immediately, otherwise browsers keep it.
    let headers = last_headers();
    let cleared = headers
        .iter()
        .filter(|(name, _)| *name == header::SET_COOKIE)
        .filter_map(|(_, value)| value.to_str().ok())
        .find(|value| value.starts_with("nsm_session="))
        .expect("clearing cookie");
    let lowered = cleared.to_ascii_lowercase();
    assert!(
        lowered.contains("max-age=0") && lowered.contains("expires="),
        "clearing cookie has no expiry: {cleared}"
    );

    // The token itself must no longer authenticate, which also proves the
    // server-side session row was deleted rather than merely hidden.
    let (status, _, _) = send(&harness, get_request("/api/auth/me", Some(&cookie))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_cookie_persists_and_round_trips() {
    let harness = harness().await;
    setup(&harness).await;

    let (status, _, cookie) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/auth/login",
            json!({ "username": "admin", "password": "a-very-long-password" }),
            None,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A session cookie without Max-Age dies when the browser closes; the login
    // cookie must survive.
    let headers = last_headers();
    let issued = headers
        .iter()
        .filter(|(name, _)| *name == header::SET_COOKIE)
        .filter_map(|(_, value)| value.to_str().ok())
        .find(|value| value.starts_with("nsm_session="))
        .expect("login cookie");
    let lowered = issued.to_ascii_lowercase();
    assert!(lowered.contains("max-age="), "no Max-Age: {issued}");
    assert!(
        !lowered.contains("max-age=0"),
        "cookie expires at once: {issued}"
    );

    // The cookie must actually authenticate on the next request.
    let cookie = cookie.expect("session cookie");
    let (status, _, _) = send(&harness, get_request("/api/auth/me", Some(&cookie))).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn malformed_body_returns_a_readable_error() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "malformed-user").await;

    // The UI can send `port: null` when a numeric field is cleared; that must be
    // reported as a usable message rather than a bare serde blob.
    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "x",
                "server": "e.com",
                "port": null,
                "params": { "protocol": "anytls", "tls": { "enabled": true } },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("error"), "{body}");
    assert!(body.contains("Malformed request"), "{body}");
    assert!(body.contains("port"), "{body}");
    // The serde type names are internal and must not reach the client.
    assert!(!body.contains("u16"), "{body}");
    assert!(!body.contains("Failed to deserialize"), "{body}");
}

#[tokio::test]
async fn field_validation_reports_every_problem() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "validate-user").await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "   ",
                "server": "https://example.com/path",
                "port": 0,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = serde_json::from_str(&body).expect("json body");
    let details = payload["details"].as_array().expect("details array");
    let fields: Vec<&str> = details
        .iter()
        .map(|detail| detail["field"].as_str().expect("field name"))
        .collect();

    assert!(fields.contains(&"name"), "{body}");
    assert!(fields.contains(&"server"), "{body}");
    assert!(fields.contains(&"port"), "{body}");
    // Every entry must carry a message the UI can show next to the input.
    for detail in details {
        assert!(
            detail["message"].as_str().is_some_and(|m| !m.is_empty()),
            "{body}"
        );
    }
}

#[tokio::test]
async fn overlong_remark_is_rejected_with_a_field_name() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/users",
            json!({ "remark": "x".repeat(80), "enabled": true }),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body.contains("\"field\":\"remark\""), "{body}");
    assert!(body.contains("at most 64 characters"), "{body}");
}

#[tokio::test]
async fn internal_errors_hide_the_cause() {
    let harness = harness().await;
    let cookie = setup(&harness).await;

    // A 404 for a missing row is a public message; an unhandled failure is not.
    let (status, body, _) = send(&harness, get_request("/api/nodes/999999", Some(&cookie))).await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::OK,
        "{status}"
    );
    assert!(!body.contains("sea_orm"), "{body}");
}

#[tokio::test]
async fn xhttp_extra_is_accepted_in_snake_case() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "xhttp-user").await;

    // The admin UI speaks snake_case; this is the exact shape it sends. It used
    // to be rejected because the XHTTP blob mirrored Xray's camelCase naming.
    let mut params = vless_params();
    params["transport"] = json!({
        "kind": "xhttp",
        "host": "",
        "path": "/x",
        "mode": "packet-up",
        "headers": { "X-Test": "1" },
        "extra": {
            "x_padding_bytes": "100-1000",
            "x_padding_obfs_mode": true,
            "no_sse_header": true,
            "sc_max_each_post_bytes": "1000000",
            "sc_max_buffered_posts": 100,
            "server_max_header_bytes": 8192,
            "xmux": { "max_concurrency": "16-32", "h_keep_alive_period": 60 },
            "download_settings": {
                "address": "dl.example.com",
                "port": 443,
                "network": "xhttp",
                "security": "tls",
                "tls_settings": { "server_name": "cdn.example.com" },
                "xhttp_settings": { "path": "/dl", "mode": "auto" }
            }
        }
    });

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "xhttp-node",
                "server": "example.com",
                "port": 443,
                "params": params,
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json body");
    assert_eq!(
        created["params"]["transport"]["extra"]["x_padding_bytes"],
        "100-1000"
    );
}

#[tokio::test]
async fn xhttp_extra_is_exported_with_xray_names() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "export-user").await;

    let mut params = vless_params();
    params["transport"] = json!({
        "kind": "xhttp",
        "path": "/x",
        "mode": "packet-up",
        "headers": { "X-Test": "1" },
        "extra": { "x_padding_bytes": "100-1000", "no_sse_header": true }
    });

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "xhttp-node",
                "server": "example.com",
                "port": 443,
                "params": params,
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json body");
    let id = created["id"].as_i64().expect("node id");

    let (status, body, _) = send(
        &harness,
        get_request(&format!("/api/nodes/{id}/render?core=xray"), Some(&cookie)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let document: Value = serde_json::from_str(&body).expect("json body");
    let text = document["content"]
        .as_str()
        .unwrap_or_else(|| document.as_str().expect("render content is a string"));
    let parsed: Value = serde_json::from_str(text).expect("config json");

    let settings = &parsed["outbounds"][0]["streamSettings"]["xhttpSettings"];
    assert_eq!(settings["mode"], "packet-up");
    assert_eq!(settings["headers"]["X-Test"], "1");
    // The `extra` blob is a JSON string carrying Xray's camelCase names.
    let extra = settings["extra"].as_str().expect("extra is a string");
    assert!(extra.contains("xPaddingBytes"), "{extra}");
    assert!(extra.contains("noSSEHeader"), "{extra}");
    assert!(!extra.contains("x_padding_bytes"), "{extra}");
}

#[tokio::test]
async fn each_node_keeps_its_own_credentials() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    // Two nodes, each with a different account for the same person: this is how
    // real proxy servers work, and the old global-credential model could not
    // express it.
    let mut node_ids = Vec::new();
    for (name, uuid, password) in [
        ("HK", "11111111-1111-4111-8111-111111111111", "hk-pass"),
        ("JP", "22222222-2222-4222-8222-222222222222", "jp-pass"),
    ] {
        let (status, body, _) = send(
            &harness,
            json_request(
                Method::POST,
                "/api/nodes",
                json!({
                    "name": name,
                    "server": format!("{name}.example.com"),
                    "port": 443,
                    "params": vless_params(),
                    "enabled": true,
                    "sort_order": 0,
                    "credentials": [{
                        "user_id": user_id,
                        "uuid": uuid,
                        "password": password,
                        "email": format!("alice@{}", name.to_lowercase()),
                    }],
                }),
                Some(&cookie),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let created: Value = serde_json::from_str(&body).expect("json");
        node_ids.push(created["id"].as_i64().expect("id"));
    }

    // Each node renders with its own identifiers. VLESS links carry the UUID and
    // no password, which is what the share-link spec defines.
    for (node_id, uuid) in [
        (node_ids[0], "11111111-1111-4111-8111-111111111111"),
        (node_ids[1], "22222222-2222-4222-8222-222222222222"),
    ] {
        let (status, body, _) = send(
            &harness,
            get_request(
                &format!("/api/nodes/{node_id}/render?core=uri&user_id={user_id}"),
                Some(&cookie),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");

        let document: Value = serde_json::from_str(&body).expect("json");
        let link = document["content"].as_str().expect("content").to_owned();

        assert!(
            link.contains(uuid),
            "node {node_id} must use {uuid}: {link}"
        );
        assert!(
            !link.contains(&other_uuid(uuid)),
            "node {node_id} leaked the other node's uuid: {link}"
        );
    }

    // The stored passwords really are per node.
    let (status, body, _) = send(
        &harness,
        get_request(&format!("/api/users/{user_id}/credentials"), Some(&cookie)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows: Value = serde_json::from_str(&body).expect("json");
    let passwords: Vec<&str> = rows
        .as_array()
        .expect("array")
        .iter()
        .map(|row| row["password"].as_str().expect("password"))
        .collect();
    assert_eq!(passwords, vec!["hk-pass", "jp-pass"], "{body}");
}

fn other_uuid(uuid: &str) -> String {
    if uuid.starts_with('1') {
        "22222222-2222-4222-8222-222222222222".to_owned()
    } else {
        "11111111-1111-4111-8111-111111111111".to_owned()
    }
}

#[tokio::test]
async fn a_node_without_credentials_is_not_published() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "HK",
                "server": "hk.example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    // Rendering for the user must fail: there is no account on that node.
    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/render?core=uri&user_id={user_id}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

#[tokio::test]
async fn generating_credentials_avoids_duplicates() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "HK",
                "server": "hk.example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    // The pair must have received generated values.
    let (status, body, _) = send(
        &harness,
        get_request(&format!("/api/users/{user_id}/credentials"), Some(&cookie)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let rows: Value = serde_json::from_str(&body).expect("json");
    let row = &rows[0];
    assert_eq!(row["enabled"], json!(true));
    let uuid = row["uuid"].as_str().expect("uuid");
    assert_eq!(uuid.len(), 36, "{body}");
    assert!(!row["password"].as_str().expect("password").is_empty());

    // Duplicate user ids in one submission are rejected, not silently merged.
    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "JP",
                "server": "jp.example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }, { "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(body.contains("one credential per node"), "{body}");
}

#[tokio::test]
async fn server_config_renders_a_runnable_inbound() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "HK",
                "server": "hk.example.com",
                "port": 443,
                "params": vless_params(),
                "server_params": {
                    "core": "xray",
                    "inbound": {
                        "protocol": "vless",
                        "listen": "0.0.0.0",
                        "port": 443,
                        "tag": "vless-in",
                        "sniffing": true,
                        "transport": "ws",
                        "path": "/ray"
                    },
                    "tls": {
                        "certificate_file": "/etc/ssl/fullchain.pem",
                        "key_file": "/etc/ssl/key.pem"
                    },
                    "reality": { "enabled": false },
                    "fallbacks": [{ "dest": "80" }]
                },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    let node_id = created["id"].as_i64().expect("id");
    // The stored server half must survive a round-trip.
    assert_eq!(created["server_params"]["inbound"]["port"], 443);
    assert_eq!(created["server_params"]["inbound"]["transport"], "ws");

    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let document: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(document["core"], "xray");
    let config: Value =
        serde_json::from_str(document["content"].as_str().expect("content")).expect("config");

    let inbound = &config["inbounds"][0];
    assert_eq!(inbound["listen"], "0.0.0.0");
    assert_eq!(inbound["port"], 443);
    assert_eq!(inbound["protocol"], "vless");
    assert_eq!(inbound["settings"]["decryption"], "none");
    // The account from `node_credentials` must appear as a server client.
    let client_id = inbound["settings"]["clients"][0]["id"]
        .as_str()
        .expect("id");
    assert_eq!(client_id.len(), 36, "{body}");
    assert_eq!(
        inbound["streamSettings"]["tlsSettings"]["certificates"][0]["certificateFile"],
        "/etc/ssl/fullchain.pem"
    );
    assert_eq!(inbound["streamSettings"]["wsSettings"]["path"], "/ray");
    assert_eq!(inbound["fallbacks"][0]["dest"], "80");
    // A startable config must have an outbound.
    assert_eq!(config["outbounds"][0]["protocol"], "freedom");
}

#[tokio::test]
async fn server_config_keeps_the_reality_private_key_off_the_wire_to_clients() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let pair = nsm::reality::generate();

    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "REALITY",
                "server": "r.example.com",
                "port": 443,
                "params": vless_params(),
                "server_params": {
                    "core": "xray",
                    "reality": {
                        "enabled": true,
                        "private_key": pair.private_key,
                        "server_names": ["www.example.com"],
                        "short_ids": ["ab"],
                        "target": "www.example.com:443"
                    }
                },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let created: Value = serde_json::from_str(&body).expect("json");
    let node_id = created["id"].as_i64().expect("id");

    // The server document carries the private key.
    let (_, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config"),
            Some(&cookie),
        ),
    )
    .await;
    assert!(body.contains("privateKey"), "{body}");
    assert!(
        !body.contains(&pair.public_key),
        "server doc leaked public key"
    );

    // The client document must never contain the private key.
    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/render?core=xray&user_id={user_id}"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !body.contains(&pair.private_key),
        "client doc leaked private key"
    );
}

#[tokio::test]
async fn server_config_is_optional() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "ClientOnly",
                "server": "c.example.com",
                "port": 443,
                "params": vless_params(),
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let node_id = created["id"].as_i64().expect("id");
    assert!(created["server_params"].is_null(), "{body}");

    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("server configuration"), "{body}");
}

#[tokio::test]
async fn server_config_honours_an_explicit_core() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "alice").await;

    let (_, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "HK",
                "server": "hk.example.com",
                "port": 443,
                "params": vless_params(),
                // Stored as xray; the export must still follow `?core=`.
                "server_params": {
                    "core": "xray",
                    "inbound": { "protocol": "vless", "port": 443 }
                },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;
    let created: Value = serde_json::from_str(&body).expect("json");
    let node_id = created["id"].as_i64().expect("id");

    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config?core=xray"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let document: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(document["core"], "xray");

    // Xray cannot serve a Hysteria2 inbound, so the request is refused with a
    // reason instead of an empty document.
    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config?core=hysteria2"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("hysteria2"), "{body}");

    let (status, body, _) = send(
        &harness,
        get_request(
            &format!("/api/nodes/{node_id}/server-config?core=sing-box"),
            Some(&cookie),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.contains("sing-box"), "{body}");
}

#[tokio::test]
async fn trojan_defaults_from_the_ui_are_accepted() {
    let harness = harness().await;
    let cookie = setup(&harness).await;
    let user_id = create_user(&harness, &cookie, "trojan-user").await;

    // The editor now defaults Trojan to TLS on, which the API requires.
    let (status, body, _) = send(
        &harness,
        json_request(
            Method::POST,
            "/api/nodes",
            json!({
                "name": "trojan",
                "server": "e.com",
                "port": 443,
                "params": { "protocol": "trojan", "tls": { "enabled": true } },
                "enabled": true,
                "sort_order": 0,
                "credentials": [{ "user_id": user_id }],
            }),
            Some(&cookie),
        ),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn disguise_covers_common_probe_paths() {
    let harness = harness().await;
    setup(&harness).await;

    for path in [
        "/.env",
        "/.git/config",
        "/admin",
        "/phpmyadmin",
        "/backup.zip",
    ] {
        let (status, body, _) = send(&harness, get_request(path, None)).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "path {path}");
        assert!(body.contains("404"), "path {path}");
    }
}

#[tokio::test]
async fn user_deletion_cleans_up_access_rows() {
    let temp = TempDir::new().expect("temp dir");
    let state = AppState::new(AppConfig::test_config(temp.path()))
        .await
        .expect("state");

    let created = db::users::create(
        &state.db,
        &db::users::NewUser {
            remark: String::new(),
            enabled: true,
            token: None,
        },
    )
    .await
    .expect("user");

    assert!(
        db::users::delete(&state.db, created.user.id)
            .await
            .expect("delete")
    );
    assert!(db::users::list(&state.db).await.expect("list").is_empty());
}
