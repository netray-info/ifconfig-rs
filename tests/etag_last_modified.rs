// Integration tests for ETag / Last-Modified caching headers.
//
// The etag_last_modified middleware in src/middleware.rs only emits these headers
// when the GeoIP City database is loaded (geoip_city_build_epoch is Some).
// In environments without data/GeoLite2-City.mmdb the middleware skips and the
// headers are absent, so all tests below are marked #[ignore].
//
// To run against a real database:
//   cargo test --test etag_last_modified -- --ignored
//
// Note: the ETag value is stable across requests within the same binary version
// and GeoIP database build — it is NOT tied to per-request content.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

fn test_config() -> Config {
    Config::load(Some("ifconfig.dev.toml")).expect("test config")
}

async fn send_request(req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let config = test_config();
    let app = ifconfig_rs::build_app(&config).await.app;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async {
                rx.await.ok();
            })
            .await
            .unwrap();
    });

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();

    let uri = format!(
        "http://{}{}",
        addr,
        req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/")
    );
    let (parts, body) = req.into_parts();
    let mut builder = Request::builder().method(parts.method).uri(&uri);
    for (key, value) in &parts.headers {
        builder = builder.header(key, value);
    }
    let request = builder
        .body(Body::from(body.collect().await.unwrap().to_bytes()))
        .unwrap();
    let response = client.request(request).await.unwrap();

    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();

    let _ = tx.send(());

    (status, headers, body_str)
}

fn get(path: &str) -> Request<Body> {
    Request::builder().uri(path).body(Body::empty()).unwrap()
}

fn get_with_headers(path: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder().uri(path);
    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }
    builder.body(Body::empty()).unwrap()
}

// 1. A successful GET /json response carries both ETag and Last-Modified headers.
#[tokio::test]
#[ignore = "requires data/GeoLite2-City.mmdb to be present"]
async fn etag_present_on_200() {
    let req = get("/json");
    let (status, headers, _) = send_request(req).await;

    assert_eq!(status, StatusCode::OK);

    let etag = headers.get("etag");
    assert!(etag.is_some(), "Expected ETag header on 200 response");
    // ETag must be a quoted string per RFC 7232
    let etag_val = etag.unwrap().to_str().unwrap();
    assert!(
        etag_val.starts_with('"') && etag_val.ends_with('"'),
        "ETag should be a quoted string, got: {etag_val}"
    );

    let last_modified = headers.get("last-modified");
    assert!(last_modified.is_some(), "Expected Last-Modified header on 200 response");
}

// 2. Sending If-None-Match with the current ETag value yields 304 Not Modified.
#[tokio::test]
#[ignore = "requires data/GeoLite2-City.mmdb to be present"]
async fn conditional_get_304_if_none_match() {
    // First request to obtain the ETag
    let (status, headers, _) = send_request(get("/json")).await;
    assert_eq!(status, StatusCode::OK);
    let etag = headers
        .get("etag")
        .expect("ETag must be present")
        .to_str()
        .unwrap()
        .to_owned();

    // Second request with If-None-Match
    let req = get_with_headers("/json", &[("if-none-match", &etag)]);
    let (status, _, body) = send_request(req).await;

    assert_eq!(status, StatusCode::NOT_MODIFIED, "Expected 304, got {status}");
    assert!(body.is_empty(), "304 body should be empty, got: {body:?}");
}

// 3. Sending If-Modified-Since with the Last-Modified date yields 304 Not Modified.
#[tokio::test]
#[ignore = "requires data/GeoLite2-City.mmdb to be present"]
async fn conditional_get_304_if_modified_since() {
    // First request to obtain Last-Modified
    let (status, headers, _) = send_request(get("/json")).await;
    assert_eq!(status, StatusCode::OK);
    let last_modified = headers
        .get("last-modified")
        .expect("Last-Modified must be present")
        .to_str()
        .unwrap()
        .to_owned();

    // Second request with If-Modified-Since set to the same date
    let req = get_with_headers("/json", &[("if-modified-since", &last_modified)]);
    let (status, _, body) = send_request(req).await;

    assert_eq!(status, StatusCode::NOT_MODIFIED, "Expected 304, got {status}");
    assert!(body.is_empty(), "304 body should be empty, got: {body:?}");
}

// 4. Sending If-None-Match with a stale/unknown ETag yields 200 (cache miss).
#[tokio::test]
#[ignore = "requires data/GeoLite2-City.mmdb to be present"]
async fn if_none_match_miss_returns_200() {
    let req = get_with_headers("/json", &[("if-none-match", "\"stale-etag-123\"")]);
    let (status, headers, _) = send_request(req).await;

    assert_eq!(status, StatusCode::OK, "Stale ETag should yield 200");
    // Response must include a fresh ETag for the client to cache
    assert!(
        headers.get("etag").is_some(),
        "200 response after ETag miss must include ETag"
    );
}

// 5. Sending If-Modified-Since with a very old date yields 200 (content is newer).
#[tokio::test]
#[ignore = "requires data/GeoLite2-City.mmdb to be present"]
async fn if_modified_since_old_date_returns_200() {
    let req = get_with_headers("/json", &[("if-modified-since", "Thu, 01 Jan 1970 00:00:00 GMT")]);
    let (status, headers, _) = send_request(req).await;

    assert_eq!(status, StatusCode::OK, "Old If-Modified-Since should yield 200");
    assert!(
        headers.get("last-modified").is_some(),
        "200 response must include Last-Modified"
    );
}
