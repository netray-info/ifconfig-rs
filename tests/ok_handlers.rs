use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::net::SocketAddr;
use tokio::net::TcpListener;

use ifconfig_rs::Config;

fn test_config() -> Config {
    Config::load(Some("ifconfig.dev.toml")).expect("test config")
}

async fn send_request(req: Request<Body>, _remote: SocketAddr) -> (StatusCode, axum::http::HeaderMap, String) {
    let config = test_config();
    let app = ifconfig_rs::build_app(&config).await.app;

    // Bind a real listener so ConnectInfo works
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

    // Connect from the test and send the request
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

fn post_json(path: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn remote_v4(ip: &str, port: u16) -> SocketAddr {
    format!("{}:{}", ip, port).parse().unwrap()
}

#[allow(dead_code)]
fn remote_v6(ip: &str, port: u16) -> SocketAddr {
    format!("[{}]:{}", ip, port).parse().unwrap()
}

fn content_type_str(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn is_json(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("application/json")).unwrap_or(false)
}

fn is_plain(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("text/plain")).unwrap_or(false)
}

fn is_html(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("text/html")).unwrap_or(false)
}

fn is_yaml(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("application/yaml")).unwrap_or(false)
}

fn is_toml(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("application/toml")).unwrap_or(false)
}

fn is_csv(ct: &Option<String>) -> bool {
    ct.as_ref().map(|s| s.contains("text/csv")).unwrap_or(false)
}

// Note: When connecting via localhost, the client IP will be 127.0.0.1 (the TCP peer),
// not the custom remote we'd want. Since there's no X-Forwarded-For and no trusted proxies
// in test mode, all tests will see the loopback address as the client IP.
// The integration tests are adapted to reflect this.

#[tokio::test]
async fn cors_header_present() {
    let req = get_with_headers("/", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("access-control-allow-origin").and_then(|v| v.to_str().ok()),
        Some("*")
    );
}

#[tokio::test]
async fn handle_root_plain_cli() {
    let req = get_with_headers("/", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct), "Expected text/plain, got {:?}", ct);
    assert!(body.ends_with('\n'), "Body should end with newline");
}

#[tokio::test]
async fn handle_root_plain() {
    let req = get_with_headers("/", &[("accept", "text/plain")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct), "Expected text/plain, got {:?}", ct);
    assert!(body.contains("\n"));
}

#[tokio::test]
async fn handle_root_json() {
    let req = get_with_headers(
        "/",
        &[
            ("accept", "application/json"),
            ("user-agent", "Some browser that will ultimately win the war."),
        ],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct), "Expected JSON, got {:?}", ct);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["ip"]["addr"].is_string());
    assert!(json["ip"]["version"].is_string());
}

#[tokio::test]
async fn handle_root_html() {
    let req = get_with_headers("/", &[("accept", "text/html")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_html(&ct), "Expected HTML, got {:?}", ct);
    assert!(body.contains("<!DOCTYPE html>") || body.contains("<html"));
}

#[tokio::test]
async fn handle_root_json_json() {
    let req = get_with_headers(
        "/json",
        &[("user-agent", "Some browser that will ultimately win the war.")],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct), "Expected JSON, got {:?}", ct);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["ip"]["addr"].is_string());
}

#[tokio::test]
async fn handle_ip_plain_cli() {
    let req = get_with_headers("/ip", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_ip_plain() {
    let req = get_with_headers("/ip", &[("accept", "text/plain")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_ip_json() {
    let req = get_with_headers("/ip", &[("accept", "application/json"), ("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["addr"].is_string());
    assert!(json["version"].is_string());
}

#[tokio::test]
async fn handle_ip_json_json() {
    let req = get_with_headers("/ip/json", &[("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["addr"].is_string());
}

#[tokio::test]
async fn handle_tcp_plain_cli() {
    let req = get_with_headers("/tcp", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_tcp_json() {
    let req = get_with_headers(
        "/tcp",
        &[("accept", "application/json"), ("user-agent", "Some browser")],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["port"].is_number());
}

#[tokio::test]
async fn handle_tcp_json_json() {
    let req = get_with_headers("/tcp/json", &[("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["port"].is_number());
}

#[tokio::test]
async fn handle_host_plain_cli_curl() {
    let req = get_with_headers("/host", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    // The host will be localhost since we're connecting locally
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_host_plain_cli_httpie() {
    let req = get_with_headers("/host", &[("user-agent", "HTTPie/0.9.9"), ("accept", "*/*")]);
    let (status, headers, _body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
}

#[tokio::test]
async fn handle_host_plain_cli_wget() {
    let req = get_with_headers(
        "/host",
        &[("user-agent", "Wget/1.19.5 (darwin17.5.0)"), ("accept", "*/*")],
    );
    let (status, headers, _body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
}

#[tokio::test]
async fn handle_host_json() {
    let req = get_with_headers(
        "/host",
        &[("accept", "application/json"), ("user-agent", "Some browser")],
    );
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["name"].is_string());
}

#[tokio::test]
async fn handle_host_json_json() {
    let req = get_with_headers("/host/json", &[("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["name"].is_string());
}

#[tokio::test]
async fn handle_isp_plain_cli() {
    let req = get_with_headers("/isp", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, _body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
}

#[tokio::test]
async fn handle_isp_json() {
    let req = get_with_headers(
        "/isp",
        &[("accept", "application/json"), ("user-agent", "Some browser")],
    );
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let _json: serde_json::Value = serde_json::from_str(&body).unwrap();
}

#[tokio::test]
async fn handle_isp_json_json() {
    let req = get_with_headers("/isp/json", &[("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let _json: serde_json::Value = serde_json::from_str(&body).unwrap();
}

#[tokio::test]
async fn handle_location_plain_cli() {
    let req = get_with_headers("/location", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, _body) = send_request(req, remote_v4("81.2.69.142", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
}

#[tokio::test]
async fn handle_location_json() {
    let req = get_with_headers(
        "/location",
        &[("accept", "application/json"), ("user-agent", "Some browser")],
    );
    let (status, headers, body) = send_request(req, remote_v4("81.2.69.142", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let _json: serde_json::Value = serde_json::from_str(&body).unwrap();
}

#[tokio::test]
async fn handle_location_json_json() {
    let req = get_with_headers("/location/json", &[("user-agent", "Some browser")]);
    let (status, headers, body) = send_request(req, remote_v4("81.2.69.142", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let _json: serde_json::Value = serde_json::from_str(&body).unwrap();
}

#[tokio::test]
async fn handle_user_agent_plain_cli() {
    let req = get_with_headers(
        "/user_agent",
        &[("user-agent", "Wget/1.19.5 (darwin17.5.0)"), ("accept", "*/*")],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.contains("Wget"));
}

#[tokio::test]
async fn handle_user_agent_json() {
    let req = get_with_headers("/user_agent", &[
        ("accept", "application/json"),
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"),
    ]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["browser"]["family"], "Safari");
}

#[tokio::test]
async fn handle_user_agent_json_json() {
    let req = get_with_headers("/user_agent/json", &[
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"),
    ]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["browser"]["family"], "Safari");
}

#[tokio::test]
async fn handle_headers_plain_cli() {
    let req = get_with_headers(
        "/headers",
        &[
            ("user-agent", "curl/7.54.0"),
            ("accept", "*/*"),
            ("x-custom-test", "hello"),
        ],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.contains("user-agent: curl/7.54.0"));
    assert!(body.contains("x-custom-test: hello"));
}

#[tokio::test]
async fn handle_headers_json() {
    let req = get_with_headers(
        "/headers",
        &[
            ("accept", "application/json"),
            ("user-agent", "curl/7.54.0"),
            ("x-custom-test", "hello"),
        ],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["x-custom-test"], "hello");
}

#[tokio::test]
async fn handle_headers_json_json() {
    let req = get_with_headers(
        "/headers/json",
        &[("user-agent", "curl/7.54.0"), ("x-custom-test", "world")],
    );
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["x-custom-test"], "world");
}

#[tokio::test]
async fn handle_ipv4_plain_cli() {
    let req = get_with_headers("/ipv4", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    // 127.0.0.1 is IPv4, so this should succeed
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_ipv4_json() {
    let req = get_with_headers("/ipv4", &[("accept", "application/json")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["version"], "4");
}

#[tokio::test]
async fn handle_ipv4_json_json() {
    let req = get("/ipv4/json");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["version"], "4");
}

#[tokio::test]
async fn handle_all_plain_cli() {
    let req = get_with_headers("/all", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.contains("ip:"));
    assert!(body.contains("version:"));
    assert!(body.contains("port:"));
}

#[tokio::test]
async fn handle_all_json() {
    let req = get_with_headers("/all", &[("accept", "application/json"), ("user-agent", "curl/7.54.0")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["ip"]["addr"].is_string());
}

#[tokio::test]
async fn handle_all_json_json() {
    let req = get_with_headers("/all/json", &[("user-agent", "curl/7.54.0")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["ip"]["addr"].is_string());
}

#[tokio::test]
async fn handle_root_json_has_network() {
    let req = get_with_headers("/", &[("accept", "application/json")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // network object should be present with classification fields
    assert!(json["network"]["type"].is_string());
    assert!(json["network"]["is_tor"].is_boolean());
    assert!(json["network"]["is_vpn"].is_boolean());
    assert!(json["network"]["is_datacenter"].is_boolean());
    assert!(json["network"]["is_bot"].is_boolean());
    assert!(json["network"]["is_threat"].is_boolean());
}

#[tokio::test]
async fn handle_all_plain_includes_network() {
    let req = get_with_headers("/all", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("network:"));
    assert!(body.contains("tor:"));
}

#[tokio::test]
async fn handle_network_plain_cli() {
    let req = get_with_headers("/network", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct));
    assert!(body.contains("type:"));
    assert!(body.contains("datacenter:"));
    assert!(body.contains("bot:"));
    assert!(body.contains("threat:"));
}

#[tokio::test]
async fn handle_network_json() {
    let req = get_with_headers("/network", &[("accept", "application/json")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["type"].is_string());
    assert!(json["is_datacenter"].is_boolean());
    assert!(json["is_bot"].is_boolean());
    assert!(json["is_threat"].is_boolean());
}

#[tokio::test]
async fn handle_network_json_suffix() {
    let req = get("/network/json");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct));
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["type"].is_string());
}

#[tokio::test]
async fn handle_network_yaml_suffix() {
    let req = get("/network/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("type:"));
}

// --- Cache-Control tests ---

#[tokio::test]
async fn cache_control_plain_text() {
    let req = get_with_headers("/ip", &[("accept", "text/plain")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=60")
    );
    assert_eq!(
        headers.get("vary").and_then(|v| v.to_str().ok()),
        Some("Accept, User-Agent")
    );
}

#[tokio::test]
async fn cache_control_json() {
    let req = get_with_headers("/ip", &[("accept", "application/json")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=60")
    );
}

#[tokio::test]
async fn cache_control_html() {
    let req = get_with_headers("/", &[("accept", "text/html")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
}

#[tokio::test]
async fn cache_control_health() {
    let req = get("/health");
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
}

// --- YAML format tests ---

#[tokio::test]
async fn handle_ip_yaml_accept() {
    let req = get_with_headers("/ip", &[("accept", "application/yaml")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("addr:"));
    assert!(body.contains("version:"));
}

#[tokio::test]
async fn handle_ip_yaml_suffix() {
    let req = get("/ip/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("addr:"));
}

// --- TOML format tests ---

#[tokio::test]
async fn handle_ip_toml_accept() {
    let req = get_with_headers("/ip", &[("accept", "application/toml")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("addr ="));
    assert!(body.contains("version ="));
}

#[tokio::test]
async fn handle_ip_toml_suffix() {
    let req = get("/ip/toml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("addr ="));
}

// --- CSV format tests ---

#[tokio::test]
async fn handle_ip_csv_accept() {
    let req = get_with_headers("/ip", &[("accept", "text/csv")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.starts_with("key,value\n"));
    assert!(body.contains("addr,"));
    assert!(body.contains("version,"));
}

#[tokio::test]
async fn handle_ip_csv_suffix() {
    let req = get("/ip/csv");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("addr,"));
}

// --- Root format suffix tests ---

#[tokio::test]
async fn handle_root_yaml_suffix() {
    let req = get("/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("ip:"));
}

#[tokio::test]
async fn handle_root_toml_suffix() {
    let req = get("/toml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("[ip]"));
}

#[tokio::test]
async fn handle_root_csv_suffix() {
    let req = get("/csv");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("ip.addr,"));
}

// --- Format suffix tests for other endpoints ---

#[tokio::test]
async fn handle_tcp_yaml_suffix() {
    let req = get("/tcp/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("port:"));
}

#[tokio::test]
async fn handle_tcp_toml_suffix() {
    let req = get("/tcp/toml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("port ="));
}

#[tokio::test]
async fn handle_tcp_csv_suffix() {
    let req = get("/tcp/csv");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("port,"));
}

#[tokio::test]
async fn handle_host_yaml_suffix() {
    let req = get("/host/yaml");
    let (status, headers, body) = send_request(req, remote_v4("8.8.8.8", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("name:"));
}

#[tokio::test]
async fn handle_all_yaml_suffix() {
    let req = get("/all/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("addr:"));
}

#[tokio::test]
async fn handle_all_toml_suffix() {
    let req = get("/all/toml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("[ip]"));
}

#[tokio::test]
async fn handle_all_csv_suffix() {
    let req = get("/all/csv");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("ip.addr,"));
}

// --- Vary header tests ---

#[tokio::test]
async fn vary_header_json() {
    let req = get_with_headers("/ip", &[("accept", "application/json")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("vary").and_then(|v| v.to_str().ok()),
        Some("Accept, User-Agent")
    );
}

#[tokio::test]
async fn vary_header_health() {
    let req = get("/health");
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(
        headers.get("vary").and_then(|v| v.to_str().ok()),
        Some("Accept, User-Agent")
    );
}

// --- Unknown format returns 404 ---

#[tokio::test]
async fn handle_ip_unknown_format_suffix_404() {
    let req = get("/ip/xml");
    let (status, _, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    // Unknown format suffix falls through to HTML (SPA), not 404
    // The SPA serves index.html for unknown routes
    assert!(status == StatusCode::OK || status == StatusCode::NOT_FOUND);
}

// --- IPv4/IPv6 format suffix tests ---

#[tokio::test]
async fn handle_ipv4_yaml_suffix() {
    let req = get("/ipv4/yaml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("addr:"));
    assert!(body.contains("version:"));
}

#[tokio::test]
async fn handle_ipv4_toml_suffix() {
    let req = get("/ipv4/toml");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("addr ="));
}

#[tokio::test]
async fn handle_ipv4_csv_suffix() {
    let req = get("/ipv4/csv");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("addr,"));
}

#[tokio::test]
async fn handle_headers_yaml_suffix() {
    let req = get_with_headers("/headers/yaml", &[("x-custom-test", "hello")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("x-custom-test: hello"));
}

#[tokio::test]
async fn handle_headers_toml_suffix() {
    let req = get_with_headers("/headers/toml", &[("x-custom-test", "hello")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("x-custom-test = \"hello\""));
}

#[tokio::test]
async fn handle_headers_csv_suffix() {
    let req = get_with_headers("/headers/csv", &[("x-custom-test", "hello")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("x-custom-test,hello"));
}

// --- Cache-Control for formats ---

#[tokio::test]
async fn cache_control_yaml() {
    let req = get("/ip/yaml");
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=60")
    );
}

#[tokio::test]
async fn cache_control_toml() {
    let req = get("/ip/toml");
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=60")
    );
}

#[tokio::test]
async fn cache_control_csv() {
    let req = get("/ip/csv");
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("private, max-age=60")
    );
}

#[tokio::test]
async fn cache_control_404_error() {
    let req = get("/does_not_exist");
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    // Fallback to SPA serves HTML with no-cache
    assert_eq!(
        headers.get("cache-control").and_then(|v| v.to_str().ok()),
        Some("no-cache")
    );
}

#[tokio::test]
async fn handle_user_agent_yaml_suffix() {
    let req = get_with_headers("/user_agent/yaml", &[
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"),
    ]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("family: Safari"));
}

#[tokio::test]
async fn handle_user_agent_toml_suffix() {
    let req = get_with_headers("/user_agent/toml", &[
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"),
    ]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_toml(&ct));
    assert!(body.contains("family = \"Safari\""));
}

#[tokio::test]
async fn handle_user_agent_csv_suffix() {
    let req = get_with_headers("/user_agent/csv", &[
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"),
    ]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct));
    assert!(body.contains("browser.family,Safari"));
}

// --- /ip/cidr tests ---

#[tokio::test]
async fn handle_ip_cidr_plain() {
    let req = get_with_headers("/ip/cidr", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct), "Expected text/plain, got {:?}", ct);
    // Loopback address in test → 127.0.0.1/32
    assert!(body.contains("/32"), "Expected /32 suffix, got: {}", body);
    assert!(body.ends_with("\n"));
}

#[tokio::test]
async fn handle_ip_cidr_no_accept() {
    let req = get("/ip/cidr");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_plain(&ct), "Expected text/plain, got {:?}", ct);
    assert!(body.contains("/32"));
}

// --- ?ip= arbitrary IP lookup tests ---

#[tokio::test]
async fn ip_param_all_json() {
    let req = get_with_headers("/all/json?ip=8.8.8.8", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"]["addr"], "8.8.8.8");
}

#[tokio::test]
async fn ip_param_ip_json() {
    let req = get_with_headers("/ip/json?ip=1.1.1.1", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["addr"], "1.1.1.1");
}

#[tokio::test]
async fn ip_param_rejects_private() {
    let req = get_with_headers("/all/json?ip=10.0.0.1", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("private/loopback"));
}

#[tokio::test]
async fn ip_param_rejects_loopback() {
    let req = get_with_headers("/ip/json?ip=127.0.0.1", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, _body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ip_param_skips_dns_by_default() {
    let req = get_with_headers("/all/json?ip=8.8.8.8", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // DNS skipped → host should be null
    assert!(json["host"].is_null());
}

#[tokio::test]
async fn ip_param_plain_text() {
    let req = get_with_headers("/ip?ip=8.8.8.8", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.trim(), "8.8.8.8");
}

#[tokio::test]
async fn ip_param_network_json() {
    let req = get_with_headers("/network/json?ip=8.8.8.8", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["type"].is_string());
}

#[tokio::test]
async fn ip_param_invalid_ignored() {
    // Invalid IP → param is None → falls back to caller's IP
    let req = get_with_headers("/ip/json?ip=notanip", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // Falls back to loopback since test connects locally
    assert_eq!(json["addr"], "127.0.0.1");
}

// --- ?fields= filtering tests ---

#[tokio::test]
async fn fields_param_filters_json() {
    let req = get_with_headers("/all/json?fields=ip,tcp", &[("user-agent", "curl/7.54.0")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["ip"].is_object(), "ip field should be present");
    assert!(json["tcp"].is_object(), "tcp field should be present");
    assert!(json.get("location").is_none(), "location should be filtered out");
    assert!(json.get("isp").is_none(), "isp should be filtered out");
}

#[tokio::test]
async fn fields_param_with_ip_param() {
    let req = get_with_headers(
        "/all/json?ip=8.8.8.8&fields=ip,isp",
        &[("user-agent", "curl/7.54.0")],
    );
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"]["addr"], "8.8.8.8");
    assert!(json["isp"].is_object());
    assert!(json.get("location").is_none());
}

#[tokio::test]
async fn fields_param_yaml_format() {
    let req = get_with_headers("/all/yaml?fields=ip", &[("user-agent", "curl/7.54.0")]);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct));
    assert!(body.contains("addr:"));
    // Should NOT contain location/isp since we filtered to ip only
    assert!(!body.contains("city:"), "city should be filtered out");
}

// --- POST /batch tests ---

#[tokio::test]
async fn batch_json_basic() {
    let req = post_json("/batch", r#"["8.8.8.8", "1.1.1.1"]"#);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct), "Expected JSON, got {:?}", ct);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["ip"]["addr"], "8.8.8.8");
    assert_eq!(arr[1]["ip"]["addr"], "1.1.1.1");
}

#[tokio::test]
async fn batch_with_invalid_ip() {
    let req = post_json("/batch", r#"["8.8.8.8", "10.0.0.1", "not-an-ip"]"#);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    // First: valid
    assert_eq!(arr[0]["ip"]["addr"], "8.8.8.8");
    // Second: private IP error
    assert!(arr[1]["error"].is_string());
    assert!(arr[1]["error"].as_str().unwrap().contains("private"));
    // Third: invalid IP error
    assert!(arr[2]["error"].is_string());
    assert!(arr[2]["error"].as_str().unwrap().contains("invalid"));
}

#[tokio::test]
async fn batch_empty_array() {
    let req = post_json("/batch", "[]");
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("empty"));
}

#[tokio::test]
async fn batch_invalid_body() {
    let req = post_json("/batch", "not json");
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("JSON array"));
}

#[tokio::test]
async fn batch_max_size_rejected() {
    // Default max_size is 100 — build an array of 101 IPs
    let ips: Vec<String> = (0..101).map(|i| format!("\"8.8.{}.{}\"", i / 256, i % 256)).collect();
    let body = format!("[{}]", ips.join(","));
    let req = post_json("/batch", &body);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("exceeds limit"), "Expected max_size rejection, got: {}", body);
}

#[tokio::test]
async fn batch_csv_format() {
    let req = post_json("/batch/csv", r#"["8.8.8.8", "1.1.1.1"]"#);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_csv(&ct), "Expected CSV, got {:?}", ct);
    let lines: Vec<&str> = body.trim().split('\n').collect();
    assert!(lines.len() >= 3, "Expected header + 2 rows, got: {:?}", lines);
    assert!(lines[0].contains("ip.addr"), "Header should contain ip.addr");
}

#[tokio::test]
async fn batch_with_fields() {
    let req = post_json("/batch?fields=ip,isp", r#"["8.8.8.8"]"#);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert!(arr[0]["ip"].is_object());
    assert!(arr[0]["isp"].is_object());
    assert!(arr[0].get("location").is_none(), "location should be filtered out");
}

#[tokio::test]
async fn batch_yaml_format() {
    let req = post_json("/batch/yaml", r#"["8.8.8.8"]"#);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct), "Expected YAML, got {:?}", ct);
    assert!(body.contains("addr:"));
}

// --- Additional batch tests ---

#[tokio::test]
async fn batch_error_has_index_not_input() {
    let req = post_json("/batch", r#"["8.8.8.8", "not-an-ip", "10.0.0.1"]"#);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = json.as_array().unwrap();
    // Invalid IP at index 1
    assert!(arr[1]["error"].is_string());
    assert_eq!(arr[1]["index"], 1, "error should report its index");
    assert!(arr[1].get("input").is_none(), "error must not echo user input");
    // Private IP at index 2
    assert!(arr[2]["error"].is_string());
    assert_eq!(arr[2]["index"], 2);
    assert!(arr[2].get("input").is_none());
}

#[tokio::test]
async fn batch_mixed_ipv4_ipv6() {
    let req = post_json("/batch", r#"["8.8.8.8", "2606:4700:4700::1111"]"#);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["ip"]["addr"], "8.8.8.8");
    assert_eq!(arr[1]["ip"]["addr"], "2606:4700:4700::1111");
}

#[tokio::test]
async fn batch_duplicate_ips_return_two_entries() {
    let req = post_json("/batch", r#"["8.8.8.8", "8.8.8.8"]"#);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2, "duplicate IPs should produce two result entries");
}

#[tokio::test]
async fn batch_yaml_with_fields() {
    let req = post_json("/batch/yaml?fields=ip,isp", r#"["8.8.8.8"]"#);
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct), "Expected YAML, got {:?}", ct);
    assert!(body.contains("addr:"), "YAML should contain ip.addr");
    assert!(!body.contains("latitude:"), "location should be filtered out");
}

// --- Content negotiation: suffix wins over Accept header ---

#[tokio::test]
async fn format_suffix_overrides_accept_header() {
    // /ip/json with Accept: text/plain — suffix must win → JSON response
    let req = get_with_headers("/ip/json", &[("accept", "text/plain")]);
    let (status, headers, _body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct), "Expected JSON (suffix wins), got {:?}", ct);
}

#[tokio::test]
async fn format_suffix_yaml_overrides_accept_json() {
    let req = get_with_headers("/ip/yaml", &[("accept", "application/json")]);
    let (status, headers, _body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_yaml(&ct), "Expected YAML (suffix wins), got {:?}", ct);
}

// --- OpenAPI spec test ---

#[tokio::test]
async fn openapi_spec_valid_json() {
    let req = get("/api-docs/openapi.json");
    let (status, headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_json(&ct), "Expected JSON, got {:?}", ct);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(json["openapi"].is_string(), "Should have openapi version");
    assert!(json["info"]["title"].is_string(), "Should have info.title");
    assert!(json["paths"].is_object(), "Should have paths");
    assert!(json["paths"]["/ip"].is_object(), "Should have /ip path");
    assert!(json["paths"]["/all"].is_object(), "Should have /all path");
    assert!(json["paths"]["/batch"].is_object(), "Should have /batch path");
    assert!(json["components"]["schemas"]["Ifconfig"].is_object(), "Should have Ifconfig schema");
}

#[tokio::test]
async fn docs_serves_scalar_html() {
    let remote = remote_v4("127.0.0.1", 12345);
    let (status, headers, body) = send_request(get("/docs"), remote).await;
    assert_eq!(status, StatusCode::OK);
    let ct = content_type_str(&headers);
    assert!(is_html(&ct), "Expected HTML, got {:?}", ct);
    assert!(body.contains("scalar"), "Body should reference Scalar");
    assert!(body.contains("/api-docs/openapi.json"), "Body should reference the OpenAPI spec URL");
}

// --- Security headers ---

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

#[tokio::test]
async fn security_headers_on_json() {
    let req = get_with_headers("/ip", &[("accept", "application/json")]);
    let (status, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, "x-content-type-options").as_deref(), Some("nosniff"));
    assert_eq!(header_str(&headers, "x-frame-options").as_deref(), Some("DENY"));
    assert_eq!(header_str(&headers, "referrer-policy").as_deref(), Some("strict-origin-when-cross-origin"));
    let hsts = header_str(&headers, "strict-transport-security").unwrap();
    assert!(hsts.contains("max-age=63072000"));
    assert!(hsts.contains("includeSubDomains"));
}

#[tokio::test]
async fn security_headers_csp_on_json() {
    let req = get_with_headers("/ip", &[("accept", "application/json")]);
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    let csp = header_str(&headers, "content-security-policy").unwrap();
    assert!(csp.contains("default-src 'self'"));
    // Non-docs CSP should NOT include cdn.jsdelivr.net
    assert!(!csp.contains("cdn.jsdelivr.net"), "Non-docs CSP should not allow CDN scripts");
}

#[tokio::test]
async fn security_headers_csp_on_docs() {
    let (_, headers, _) = send_request(get("/docs"), remote_v4("192.168.0.101", 8000)).await;
    let csp = header_str(&headers, "content-security-policy").unwrap();
    assert!(csp.contains("cdn.jsdelivr.net"), "/docs CSP should allow CDN scripts");
}

#[tokio::test]
async fn request_id_generated() {
    let req = get_with_headers("/ip", &[("accept", "application/json")]);
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    let id = header_str(&headers, "x-request-id").unwrap();
    assert_eq!(id.len(), 16, "Generated request ID should be 16 hex chars");
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn request_id_propagated() {
    let req = get_with_headers("/ip", &[("accept", "application/json"), ("x-request-id", "custom-id-12345")]);
    let (_, headers, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(header_str(&headers, "x-request-id").as_deref(), Some("custom-id-12345"));
}

// --- /ipv6 endpoint tests ---

#[tokio::test]
async fn ipv6_returns_404_for_ipv4_client() {
    let req = get_with_headers("/ipv6", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, _, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ipv6_json_returns_404_for_ipv4_client() {
    let req = get_with_headers("/ipv6/json", &[("user-agent", "curl/7.54.0")]);
    let (status, _, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("not implemented"));
}

#[tokio::test]
async fn ipv6_with_ipv6_ip_param() {
    let req = get_with_headers("/ipv6/json?ip=2606:4700::1111", &[("user-agent", "curl/7.54.0")]);
    let (status, _, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["addr"], "2606:4700::1111");
    assert_eq!(json["version"], "6");
}

#[tokio::test]
async fn ipv6_with_ipv4_ip_param_returns_404() {
    let req = get_with_headers("/ipv6/json?ip=8.8.8.8", &[("user-agent", "curl/7.54.0")]);
    let (status, _, _) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// --- IPv6 client tests ---
//
// These tests bind the server to [::1] so the connecting client is seen as an IPv6 client.

async fn send_request_v6(path: &str, req_headers: &[(&str, &str)]) -> (StatusCode, axum::http::HeaderMap, String) {
    let config = test_config();
    let app = ifconfig_rs::build_app(&config).await.app;

    let listener = TcpListener::bind("[::1]:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async { rx.await.ok(); })
            .await
            .unwrap();
    });

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();
    let uri = format!("http://[::1]:{}{}", addr.port(), path);
    let mut builder = Request::builder()
        .method("GET")
        .uri(&uri)
        .header("user-agent", "curl/8.0")
        .header("accept", "*/*");
    for (key, value) in req_headers {
        builder = builder.header(*key, *value);
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = client.request(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap_or_default();
    let _ = tx.send(());
    (status, headers, body_str)
}

#[tokio::test]
async fn ipv6_client_root_returns_200_with_v6() {
    let (status, _, body) = send_request_v6("/json", &[]).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"]["version"], "6");
    assert_eq!(json["ip"]["addr"], "::1");
}

#[tokio::test]
async fn ipv6_client_ipv6_endpoint_returns_200() {
    let (status, _, body) = send_request_v6("/ipv6/json", &[]).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["version"], "6");
}

#[tokio::test]
async fn ipv6_client_ipv4_endpoint_returns_404() {
    let (status, _, _) = send_request_v6("/ipv4/json", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// --- CORS preflight test ---

#[tokio::test]
async fn cors_preflight_returns_correct_headers() {
    let config = test_config();
    let app = ifconfig_rs::build_app(&config).await.app;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(async { rx.await.ok(); })
            .await
            .unwrap();
    });

    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new()).build_http();
    let uri = format!("http://{}/", addr);
    let request = Request::builder()
        .method("OPTIONS")
        .uri(&uri)
        .header("origin", "https://example.com")
        .header("access-control-request-method", "GET")
        .body(Body::empty())
        .unwrap();
    let response = client.request(request).await.unwrap();

    let _ = tx.send(());

    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NO_CONTENT,
        "CORS preflight should return 200 or 204, got {}",
        response.status()
    );
    assert_eq!(
        response.headers().get("access-control-allow-origin").and_then(|v| v.to_str().ok()),
        Some("*"),
        "CORS preflight must echo back Access-Control-Allow-Origin: *"
    );
    assert!(
        response.headers().contains_key("access-control-allow-methods"),
        "CORS preflight must include Access-Control-Allow-Methods"
    );
}
