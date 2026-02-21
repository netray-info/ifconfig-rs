use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde_json::json;

use crate::backend::*;
use crate::extractors::{extract_headers, RequesterInfo};
use crate::format::OutputFormat;
use crate::handlers;
use crate::negotiate::{negotiate, NegotiatedFormat};
use crate::state::AppState;

/// Build the main router.
pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        // Root endpoint
        .route("/", get(root_handler))
        // Root format suffixes
        .route("/json", get(root_format_handler))
        .route("/yaml", get(root_format_handler))
        .route("/toml", get(root_format_handler))
        .route("/csv", get(root_format_handler))
        // Standard endpoints
        .route("/ip", get(ip_handler))
        .route("/ip/{fmt}", get(ip_format_handler))
        .route("/tcp", get(tcp_handler))
        .route("/tcp/{fmt}", get(tcp_format_handler))
        .route("/host", get(host_handler))
        .route("/host/{fmt}", get(host_format_handler))
        .route("/location", get(location_handler))
        .route("/location/{fmt}", get(location_format_handler))
        .route("/isp", get(isp_handler))
        .route("/isp/{fmt}", get(isp_format_handler))
        .route("/user_agent", get(user_agent_handler))
        .route("/user_agent/{fmt}", get(user_agent_format_handler))
        .route("/all", get(all_handler))
        .route("/all/{fmt}", get(all_format_handler))
        .route("/headers", get(headers_handler))
        .route("/headers/{fmt}", get(headers_format_handler))
        .route("/ipv4", get(ipv4_handler))
        .route("/ipv4/{fmt}", get(ipv4_format_handler))
        .route("/ipv6", get(ipv6_handler))
        .route("/ipv6/{fmt}", get(ipv6_format_handler))
        // Meta endpoint (site info for SPA)
        .route("/meta", get(meta_handler))
        // Health endpoint (no content negotiation)
        .route("/health", get(health_handler))
}

fn get_requester_info(headers: &HeaderMap, extensions: &axum::http::Extensions) -> RequesterInfo {
    extensions
        .get::<RequesterInfo>()
        .cloned()
        .unwrap_or_else(|| RequesterInfo {
            remote: "127.0.0.1:0".parse().unwrap(),
            user_agent: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            uri: "/".to_string(),
        })
}

/// Build a formatted response with proper content-type.
fn respond_formatted(content_type: &str, body: String) -> Response {
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, content_type)], body).into_response()
}

fn respond_plain(body: String) -> Response {
    respond_formatted("text/plain; charset=utf-8", body)
}

fn respond_json_value(value: serde_json::Value) -> Response {
    (StatusCode::OK, axum::Json(value)).into_response()
}

fn serve_spa() -> Response {
    #[derive(rust_embed::RustEmbed)]
    #[folder = "frontend/dist"]
    struct Assets;

    match Assets::get("index.html") {
        Some(content) => {
            let body = std::str::from_utf8(content.data.as_ref()).unwrap_or("");
            Html(body.to_string()).into_response()
        }
        None => (StatusCode::INTERNAL_SERVER_ERROR, "SPA not found").into_response(),
    }
}

// ---- Compute-once dispatch ----

fn resolve_backends(state: &AppState) -> Option<(&UserAgentParser, &GeoIpCityDb, &GeoIpAsnDb, &TorExitNodes)> {
    let uap = state.user_agent_parser.as_deref()?;
    let city = state.geoip_city_db.as_deref()?;
    let asn = state.geoip_asn_db.as_deref()?;
    let tor = &*state.tor_exit_nodes;
    Some((uap, city, asn, tor))
}

async fn dispatch_standard(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    to_json_fn: fn(&Ifconfig) -> Option<serde_json::Value>,
    to_plain_fn: fn(&Ifconfig) -> String,
) -> Response {
    if format == NegotiatedFormat::Html {
        return serve_spa();
    }

    let (uap, city, asn, tor) = match resolve_backends(state) {
        Some(backends) => backends,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "backends not available").into_response(),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;
    let ifconfig = handlers::make_ifconfig(&req_info.remote, &ua_opt, uap, city, asn, tor, &state.dns_resolver).await;

    match format {
        NegotiatedFormat::Html => unreachable!(),
        NegotiatedFormat::Plain => respond_plain(to_plain_fn(&ifconfig)),
        NegotiatedFormat::Json => match to_json_fn(&ifconfig) {
            Some(val) => respond_json_value(val),
            None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
        },
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match to_json_fn(&ifconfig).and_then(|v| output_fmt.serialize_body(&v)) {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
    }
}

// ---- Root handler ----

async fn root_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::root::to_json,
        handlers::root::to_plain,
    )
    .await
}

async fn root_format_handler(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let suffix = uri.path().trim_start_matches('/');
    let format = negotiate(Some(suffix), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::root::to_json,
        handlers::root::to_plain,
    )
    .await
}

// ---- Standard endpoint handlers ----

async fn ip_handler(State(state): State<AppState>, headers: HeaderMap, extensions: axum::http::Extensions) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(format, &req_info, &state, handlers::ip::to_json, handlers::ip::to_plain).await
}

async fn ip_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(format, &req_info, &state, handlers::ip::to_json, handlers::ip::to_plain).await
}

async fn tcp_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::tcp::to_json,
        handlers::tcp::to_plain,
    )
    .await
}

async fn tcp_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::tcp::to_json,
        handlers::tcp::to_plain,
    )
    .await
}

async fn host_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::host::to_json,
        handlers::host::to_plain,
    )
    .await
}

async fn host_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::host::to_json,
        handlers::host::to_plain,
    )
    .await
}

async fn location_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::location::to_json,
        handlers::location::to_plain,
    )
    .await
}

async fn location_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::location::to_json,
        handlers::location::to_plain,
    )
    .await
}

async fn isp_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::isp::to_json,
        handlers::isp::to_plain,
    )
    .await
}

async fn isp_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::isp::to_json,
        handlers::isp::to_plain,
    )
    .await
}

async fn user_agent_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::user_agent::to_json,
        handlers::user_agent::to_plain,
    )
    .await
}

async fn user_agent_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::user_agent::to_json,
        handlers::user_agent::to_plain,
    )
    .await
}

async fn all_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(None, &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::all::to_json,
        handlers::all::to_plain,
    )
    .await
}

async fn all_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::all::to_json,
        handlers::all::to_plain,
    )
    .await
}

// ---- Headers handler ----

async fn headers_handler(headers: HeaderMap) -> Response {
    let format = negotiate(None, &headers);
    let req_headers = extract_headers(&headers);
    dispatch_headers(format, &req_headers)
}

async fn headers_format_handler(Path(fmt): Path<String>, headers: HeaderMap) -> Response {
    let format = negotiate(Some(&fmt), &headers);
    let req_headers = extract_headers(&headers);
    dispatch_headers(format, &req_headers)
}

fn dispatch_headers(format: NegotiatedFormat, req_headers: &[(String, String)]) -> Response {
    match format {
        NegotiatedFormat::Html => serve_spa(),
        NegotiatedFormat::Plain => respond_plain(handlers::headers::to_plain(req_headers)),
        NegotiatedFormat::Json => respond_json_value(handlers::headers::to_json_value(req_headers)),
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match handlers::headers::formatted(&output_fmt, req_headers) {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => (StatusCode::INTERNAL_SERVER_ERROR, "serialization failed").into_response(),
            }
        }
    }
}

// ---- IP version handlers ----

async fn ipv4_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("4", None, &state, &headers, &extensions).await
}

async fn ipv4_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("4", Some(&fmt), &state, &headers, &extensions).await
}

async fn ipv6_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("6", None, &state, &headers, &extensions).await
}

async fn ipv6_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("6", Some(&fmt), &state, &headers, &extensions).await
}

async fn ip_version_dispatch(
    version: &str,
    suffix: Option<&str>,
    state: &AppState,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(headers, extensions);
    let format = negotiate(suffix, headers);

    if format == NegotiatedFormat::Html {
        return serve_spa();
    }

    let (uap, city, asn, tor) = match resolve_backends(state) {
        Some(backends) => backends,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "backends not available").into_response(),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;
    let ifconfig = handlers::make_ifconfig(&req_info.remote, &ua_opt, uap, city, asn, tor, &state.dns_resolver).await;

    if ifconfig.ip.version != version {
        return (StatusCode::NOT_FOUND, "not implemented").into_response();
    }

    match format {
        NegotiatedFormat::Html => unreachable!(),
        NegotiatedFormat::Plain => respond_plain(handlers::ip_version::to_plain(&ifconfig)),
        NegotiatedFormat::Json => match handlers::ip_version::to_json(&ifconfig) {
            Some(val) => respond_json_value(val),
            None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
        },
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match handlers::ip_version::to_json(&ifconfig).and_then(|v| output_fmt.serialize_body(&v)) {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
    }
}

// ---- Meta handler (site info for SPA) ----

async fn meta_handler(State(state): State<AppState>) -> Response {
    (StatusCode::OK, axum::Json(&*state.project_info)).into_response()
}

// ---- Health handler ----

async fn health_handler(State(state): State<AppState>) -> Response {
    let has_city_db = state.geoip_city_db.is_some();
    let has_asn_db = state.geoip_asn_db.is_some();

    if has_city_db && has_asn_db {
        (StatusCode::OK, axum::Json(json!({ "status": "ok" }))).into_response()
    } else {
        let mut missing = Vec::new();
        if !has_city_db {
            missing.push("GeoIP City database not loaded");
        }
        if !has_asn_db {
            missing.push("GeoIP ASN database not loaded");
        }
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({
                "status": "unhealthy",
                "reason": missing.join("; ")
            })),
        )
            .into_response()
    }
}

// ---- Static file serving ----

#[derive(rust_embed::RustEmbed)]
#[folder = "frontend/dist"]
struct Assets;

pub async fn static_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == "index.html" {
        return serve_spa_with_no_cache();
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let cache = if path.contains('.') && (path.contains("-") || path.contains("assets/")) {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, mime.as_ref().to_string()),
                    (axum::http::header::CACHE_CONTROL, cache.to_string()),
                ],
                content.data.to_vec(),
            )
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for unknown paths
            serve_spa_with_no_cache()
        }
    }
}

fn serve_spa_with_no_cache() -> Response {
    match Assets::get("index.html") {
        Some(content) => {
            let body = content.data.to_vec();
            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
                    (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
                ],
                body,
            )
                .into_response()
        }
        None => (StatusCode::INTERNAL_SERVER_ERROR, "SPA not found").into_response(),
    }
}
