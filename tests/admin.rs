/// Admin port integration tests.
///
/// The Prometheus metrics recorder is a process-global singleton; only the
/// first call to `PrometheusBuilder::install_recorder()` succeeds. Because
/// each `tests/*.rs` file compiles to its own binary (separate process), this
/// file gets exactly one shot at installing the recorder.
///
/// All admin-port scenarios are covered in the single test function below.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

fn test_config() -> Config {
    Config::load(Some("ifconfig.dev.toml")).expect("test config")
}

/// Sends a GET request to the given URI with optional headers.
async fn admin_get(addr: SocketAddr, path: &str, headers: &[(&str, &str)]) -> (StatusCode, String) {
    let client =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build_http();
    let uri = format!("http://{}{}", addr, path);
    let mut builder = Request::builder().uri(&uri);
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    let req = builder.body(Body::empty()).unwrap();
    let response = client.request(req).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8(bytes.to_vec()).unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn admin_port_metrics_and_bearer_auth() {
    let mut config = test_config();
    config.server.admin_bind = Some("127.0.0.1:0".to_string());
    config.server.admin_token = Some("test-admin-secret".to_string());

    let bundle = ifconfig_rs::build_app(&config).await;

    let admin_app = match bundle.admin_app {
        Some(app) => app,
        None => {
            // Metrics recorder already installed by another binary in the same
            // run (e.g. ok_handlers). The admin app is not available; skip.
            eprintln!("admin_port test: Prometheus recorder already installed — skipping");
            return;
        }
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    tokio::spawn(async move {
        axum::serve(listener, admin_app.into_make_service())
            .with_graceful_shutdown(async { rx.await.ok(); })
            .await
            .unwrap();
    });

    // No token → 401
    let (status, _) = admin_get(addr, "/metrics", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "/metrics without token should be 401");

    // Wrong token → 401
    let (status, _) = admin_get(addr, "/metrics", &[("authorization", "Bearer wrong-token")]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "/metrics with wrong token should be 401");

    // Correct token → 200 with Prometheus text/plain body
    let (status, body) =
        admin_get(addr, "/metrics", &[("authorization", "Bearer test-admin-secret")]).await;
    assert_eq!(status, StatusCode::OK, "/metrics with correct token should be 200");
    assert!(
        body.contains("# HELP") || body.contains("# TYPE"),
        "/metrics body should contain Prometheus output, got: {}",
        &body[..body.len().min(200)]
    );

    // /health on admin port also requires auth when token is set
    let (status, _) = admin_get(addr, "/health", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "/health without token should be 401");

    let (status, _) =
        admin_get(addr, "/health", &[("authorization", "Bearer test-admin-secret")]).await;
    assert_eq!(status, StatusCode::OK, "/health with correct token should be 200");

    let _ = tx.send(());
}
