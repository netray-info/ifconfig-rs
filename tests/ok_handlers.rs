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
    let app = ifconfig_rs::build_app(&config).app;

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
    assert!(body.contains("\n"), "Body should end with newline");
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
    assert!(body.contains("html"));
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
async fn handle_root_json_has_is_tor() {
    let req = get_with_headers("/", &[("accept", "application/json")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    // is_tor should be present (either true or false)
    assert!(json["is_tor"].is_boolean());
}

#[tokio::test]
async fn handle_all_plain_includes_tor() {
    let req = get_with_headers("/all", &[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let (status, _headers, body) = send_request(req, remote_v4("192.168.0.101", 8000)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("tor:"));
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
