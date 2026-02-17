use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde_json::json;

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

// ---- Handler macro to reduce boilerplate ----

macro_rules! endpoint_handler {
    ($handler_fn:ident, $mod_name:ident) => {
        async fn $handler_fn(
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
                |fmt, remote, ua, uap, city, asn, tor| {
                    handlers::$mod_name::formatted(fmt, remote, ua, uap, city, asn, tor)
                },
                |remote, ua, uap, city, asn, tor| handlers::$mod_name::plain(remote, ua, uap, city, asn, tor),
                |remote, ua, uap, city, asn, tor| handlers::$mod_name::json(remote, ua, uap, city, asn, tor),
            )
        }
    };
}

macro_rules! endpoint_format_handler {
    ($handler_fn:ident, $mod_name:ident) => {
        async fn $handler_fn(
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
                |fmt, remote, ua, uap, city, asn, tor| {
                    handlers::$mod_name::formatted(fmt, remote, ua, uap, city, asn, tor)
                },
                |remote, ua, uap, city, asn, tor| handlers::$mod_name::plain(remote, ua, uap, city, asn, tor),
                |remote, ua, uap, city, asn, tor| handlers::$mod_name::json(remote, ua, uap, city, asn, tor),
            )
        }
    };
}

use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use std::net::SocketAddr;

fn dispatch_standard(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    formatted_fn: impl Fn(
        &OutputFormat,
        &SocketAddr,
        &Option<&str>,
        &UserAgentParser,
        &GeoIpCityDb,
        &GeoIpAsnDb,
        &TorExitNodes,
    ) -> Option<String>,
    plain_fn: impl Fn(
        &SocketAddr,
        &Option<&str>,
        &UserAgentParser,
        &GeoIpCityDb,
        &GeoIpAsnDb,
        &TorExitNodes,
    ) -> Option<String>,
    json_fn: impl Fn(
        &SocketAddr,
        &Option<&str>,
        &UserAgentParser,
        &GeoIpCityDb,
        &GeoIpAsnDb,
        &TorExitNodes,
    ) -> Option<serde_json::Value>,
) -> Response {
    let (uap, city, asn, tor) = match resolve_backends(state) {
        Some(backends) => backends,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "backends not available").into_response(),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;

    match format {
        NegotiatedFormat::Html => serve_spa(),
        NegotiatedFormat::Plain => match plain_fn(&req_info.remote, &ua_opt, uap, city, asn, tor) {
            Some(body) => respond_plain(body),
            None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
        },
        NegotiatedFormat::Json => match json_fn(&req_info.remote, &ua_opt, uap, city, asn, tor) {
            Some(val) => respond_json_value(val),
            None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
        },
        NegotiatedFormat::Yaml => {
            let fmt = OutputFormat::Yaml;
            match formatted_fn(&fmt, &req_info.remote, &ua_opt, uap, city, asn, tor) {
                Some(body) => respond_formatted(fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
        NegotiatedFormat::Toml => {
            let fmt = OutputFormat::Toml;
            match formatted_fn(&fmt, &req_info.remote, &ua_opt, uap, city, asn, tor) {
                Some(body) => respond_formatted(fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
        NegotiatedFormat::Csv => {
            let fmt = OutputFormat::Csv;
            match formatted_fn(&fmt, &req_info.remote, &ua_opt, uap, city, asn, tor) {
                Some(body) => respond_formatted(fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
    }
}

fn resolve_backends(state: &AppState) -> Option<(&UserAgentParser, &GeoIpCityDb, &GeoIpAsnDb, &TorExitNodes)> {
    let uap = state.user_agent_parser.as_deref()?;
    let city = state.geoip_city_db.as_deref()?;
    let asn = state.geoip_asn_db.as_deref()?;
    let tor = &*state.tor_exit_nodes;
    Some((uap, city, asn, tor))
}

// ---- Root handler (special: also serves SPA) ----

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
        handlers::root::formatted,
        handlers::root::plain,
        handlers::root::json,
    )
}

async fn root_format_handler(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    // Extract format from URI path: /json, /yaml, /toml, /csv
    let suffix = uri.path().trim_start_matches('/');
    let format = negotiate(Some(suffix), &headers);
    dispatch_standard(
        format,
        &req_info,
        &state,
        handlers::root::formatted,
        handlers::root::plain,
        handlers::root::json,
    )
}

// ---- Standard endpoint handlers ----

endpoint_handler!(ip_handler, ip);
endpoint_format_handler!(ip_format_handler, ip);

endpoint_handler!(tcp_handler, tcp);
endpoint_format_handler!(tcp_format_handler, tcp);

endpoint_handler!(host_handler, host);
endpoint_format_handler!(host_format_handler, host);

endpoint_handler!(location_handler, location);
endpoint_format_handler!(location_format_handler, location);

endpoint_handler!(isp_handler, isp);
endpoint_format_handler!(isp_format_handler, isp);

endpoint_handler!(user_agent_handler, user_agent);
endpoint_format_handler!(user_agent_format_handler, user_agent);

endpoint_handler!(all_handler, all);
endpoint_format_handler!(all_format_handler, all);

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
    ip_version_dispatch("4", None, &state, &headers, &extensions)
}

async fn ipv4_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("4", Some(&fmt), &state, &headers, &extensions)
}

async fn ipv6_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("6", None, &state, &headers, &extensions)
}

async fn ipv6_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    ip_version_dispatch("6", Some(&fmt), &state, &headers, &extensions)
}

fn ip_version_dispatch(
    version: &str,
    suffix: Option<&str>,
    state: &AppState,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(headers, extensions);
    let format = negotiate(suffix, headers);

    let (uap, city, asn, tor) = match resolve_backends(state) {
        Some(backends) => backends,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, "backends not available").into_response(),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;

    match format {
        NegotiatedFormat::Html => serve_spa(),
        NegotiatedFormat::Plain => {
            match handlers::ip_version::plain(version, &req_info.remote, &ua_opt, uap, city, asn, tor) {
                Some(body) => respond_plain(body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
        NegotiatedFormat::Json => {
            match handlers::ip_version::json(version, &req_info.remote, &ua_opt, uap, city, asn, tor) {
                Some(val) => respond_json_value(val),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match handlers::ip_version::formatted(version, &output_fmt, &req_info.remote, &ua_opt, uap, city, asn, tor)
            {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => (StatusCode::NOT_FOUND, "not implemented").into_response(),
            }
        }
    }
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
