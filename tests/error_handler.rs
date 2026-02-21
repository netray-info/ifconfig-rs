use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

fn test_config() -> Config {
    Config::load(Some("ifconfig.dev.toml")).expect("test config")
}

async fn send_request(req: Request<Body>) -> (StatusCode, String) {
    let config = test_config();
    let app = ifconfig_rs::build_app(&config).app;

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
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();

    let _ = tx.send(());

    (status, body_str)
}

#[tokio::test]
async fn handle_error_not_found() {
    // The SPA fallback serves index.html for unknown paths, so we get 200 + HTML
    // This is intentional: browsers should get the SPA for any route
    let req = Request::builder().uri("/does_not_exist").body(Body::empty()).unwrap();
    let (status, body) = send_request(req).await;
    // SPA fallback returns 200 with HTML
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("html"));
}
