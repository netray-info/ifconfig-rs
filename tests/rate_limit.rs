use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

struct TestResponse {
    status: StatusCode,
    headers: HashMap<String, String>,
}

/// Config with a very tight rate limit: burst of 2, 2 requests per minute.
fn rate_limit_config() -> Config {
    let mut config = Config::load(Some("ifconfig.dev.toml")).expect("test config");
    config.rate_limit.per_ip_per_minute = 2;
    config.rate_limit.per_ip_burst = 2;
    config
}

async fn spawn_server(config: &Config) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let app = ifconfig_rs::build_app(config).app;
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

    (addr, tx)
}

async fn do_get_full(addr: SocketAddr, path: &str) -> TestResponse {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();
    let uri = format!("http://{}{}", addr, path);
    let req = Request::builder()
        .uri(&uri)
        .header("user-agent", "curl/8.0")
        .header("accept", "*/*")
        .body(Body::empty())
        .unwrap();
    let response = client.request(req).await.unwrap();
    let status = response.status();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let _ = response.into_body().collect().await;
    TestResponse { status, headers }
}

async fn do_get(addr: SocketAddr, path: &str) -> StatusCode {
    do_get_full(addr, path).await.status
}

#[tokio::test]
async fn rate_limit_returns_429_after_burst() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // First two requests should succeed (burst = 2)
    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::OK, "first request should succeed");

    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::OK, "second request should succeed");

    // Third request should be rate-limited
    let status = do_get(addr, "/ip").await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "third request should be rate-limited"
    );
}

#[tokio::test]
async fn rate_limit_health_exempt() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // Exhaust the rate limit
    let _ = do_get(addr, "/ip").await;
    let _ = do_get(addr, "/ip").await;
    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "rate limit should be hit");

    // /health should still work
    let status = do_get(addr, "/health").await;
    assert_eq!(status, StatusCode::OK, "/health should be exempt from rate limiting");
}

#[tokio::test]
async fn rate_limit_headers_on_success() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    let resp = do_get_full(addr, "/ip").await;
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(
        resp.headers.get("x-ratelimit-limit").map(|s| s.as_str()),
        Some("2"),
        "x-ratelimit-limit should equal burst size"
    );
    let remaining: u32 = resp
        .headers
        .get("x-ratelimit-remaining")
        .expect("x-ratelimit-remaining should be present")
        .parse()
        .expect("should be a number");
    assert!(remaining <= 2, "remaining should be <= burst");
}

#[tokio::test]
async fn rate_limit_headers_on_429() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // Exhaust burst
    let _ = do_get(addr, "/ip").await;
    let _ = do_get(addr, "/ip").await;

    let resp = do_get_full(addr, "/ip").await;
    assert_eq!(resp.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(resp.headers.get("x-ratelimit-limit").map(|s| s.as_str()), Some("2"),);
    assert_eq!(resp.headers.get("x-ratelimit-remaining").map(|s| s.as_str()), Some("0"),);
    let retry_after: u64 = resp
        .headers
        .get("retry-after")
        .expect("retry-after should be present on 429")
        .parse()
        .expect("should be a number");
    assert!(retry_after >= 1, "retry-after should be at least 1 second");
}

#[tokio::test]
async fn rate_limit_ready_exempt() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // Exhaust the rate limit
    let _ = do_get(addr, "/ip").await;
    let _ = do_get(addr, "/ip").await;
    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "rate limit should be hit");

    // /ready should still work
    let status = do_get(addr, "/ready").await;
    assert_eq!(status, StatusCode::OK, "/ready should be exempt from rate limiting");
}
