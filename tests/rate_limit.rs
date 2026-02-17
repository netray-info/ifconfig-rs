use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

/// Config with a very tight rate limit: burst of 1, 2 requests per minute.
fn rate_limit_config() -> Config {
    let mut config = Config::load(Some("ifconfig.dev.toml")).expect("test config");
    config.rate_limit.per_ip_per_minute = 2;
    config.rate_limit.per_ip_burst = 1;
    config
}

async fn spawn_server(config: &Config) -> (SocketAddr, tokio::sync::oneshot::Sender<()>) {
    let app = ifconfig_rs::build_app(config);
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

async fn do_get(addr: SocketAddr, path: &str) -> StatusCode {
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();
    let uri = format!("http://{}{}", addr, path);
    let req = Request::builder()
        .uri(&uri)
        .header("user-agent", "curl/8.0")
        .header("accept", "*/*")
        .body(Body::empty())
        .unwrap();
    let response = client.request(req).await.unwrap();
    // Drain body so the connection is released
    let status = response.status();
    let _ = response.into_body().collect().await;
    status
}

#[tokio::test]
async fn rate_limit_returns_429_after_burst() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // First request should succeed (burst = 1)
    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::OK, "first request should succeed");

    // Second request should be rate-limited
    let status = do_get(addr, "/ip").await;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "second request should be rate-limited"
    );
}

#[tokio::test]
async fn rate_limit_health_exempt() {
    let config = rate_limit_config();
    let (addr, _tx) = spawn_server(&config).await;

    // Exhaust the rate limit on a normal endpoint
    let _ = do_get(addr, "/ip").await;
    let status = do_get(addr, "/ip").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "rate limit should be hit");

    // /health should still work
    let status = do_get(addr, "/health").await;
    assert_eq!(status, StatusCode::OK, "/health should be exempt from rate limiting");
}
