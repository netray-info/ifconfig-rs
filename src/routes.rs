use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use governor::clock::{Clock, DefaultClock};
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;

use utoipa::OpenApi;

use crate::backend::*;
use crate::enrichment::EnrichmentContext;
use crate::error::{error_response, ErrorResponse};
use crate::extractors::{extract_headers, filter_headers, RequesterInfo};
use crate::format::{self, OutputFormat};
use crate::handlers;
use crate::negotiate::{negotiate, NegotiatedFormat};
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ifconfig-rs",
        description = "IP address lookup and enrichment API",
        version = "0.8.0",
        license(name = "MIT"),
    ),
    paths(
        root_handler,
        ip_handler,
        ip_cidr_handler,
        tcp_handler,
        host_handler,
        location_handler,
        isp_handler,
        user_agent_handler,
        network_handler,
        all_handler,
        headers_handler,
        ipv4_handler,
        ipv6_handler,
        batch_handler,
        health_handler,
        ready_handler,
    ),
    components(schemas(
        Ifconfig, Ip, Tcp, Host, Location, Isp, Network,
        UserAgent, Browser, OS, Device, ErrorResponse,
    ))
)]
struct ApiDoc;

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
        .route("/ip/cidr", get(ip_cidr_handler))
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
        .route("/network", get(network_handler))
        .route("/network/{fmt}", get(network_format_handler))
        .route("/all", get(all_handler))
        .route("/all/{fmt}", get(all_format_handler))
        .route("/headers", get(headers_handler))
        .route("/headers/{fmt}", get(headers_format_handler))
        .route("/ipv4", get(ipv4_handler))
        .route("/ipv4/{fmt}", get(ipv4_format_handler))
        .route("/ipv6", get(ipv6_handler))
        .route("/ipv6/{fmt}", get(ipv6_format_handler))
        // Batch endpoint
        .route("/batch", post(batch_handler))
        .route("/batch/{fmt}", post(batch_format_handler))
        // Meta endpoint (site info for SPA)
        .route("/meta", get(meta_handler))
        // Probe endpoints (no content negotiation)
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        // OpenAPI spec + docs UI
        .route("/api-docs/openapi.json", get(openapi_handler))
        .route("/docs", get(docs_handler))
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

// ---- Query parameter helpers ----

fn parse_query_param<'a>(uri: &'a str, key: &str) -> Option<&'a str> {
    let query = uri.split('?').nth(1)?;
    query
        .split('&')
        .find_map(|p| p.strip_prefix(key).and_then(|rest| rest.strip_prefix('=')))
}

fn parse_ip_param(uri: &str) -> Option<IpAddr> {
    parse_query_param(uri, "ip").and_then(|s| s.parse().ok())
}

fn parse_dns_param(uri: &str) -> bool {
    parse_query_param(uri, "dns")
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

fn is_global_ip(ip: IpAddr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return false;
    }
    match ip {
        IpAddr::V4(v4) => !v4.is_private() && !v4.is_link_local(),
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            // ULA fc00::/7
            if segs[0] & 0xfe00 == 0xfc00 {
                return false;
            }
            // Link-local fe80::/10
            if segs[0] & 0xffc0 == 0xfe80 {
                return false;
            }
            // Multicast ff00::/8
            if segs[0] & 0xff00 == 0xff00 {
                return false;
            }
            // IPv4-mapped ::ffff:x.x.x.x — check the embedded v4 address
            if let Some(v4) = v6.to_ipv4_mapped() {
                return !v4.is_private() && !v4.is_link_local() && !v4.is_loopback();
            }
            true
        }
    }
}

// ---- Compute-once dispatch ----

fn resolve_backends(ctx: &EnrichmentContext) -> Option<(&UserAgentParser, &GeoIpCityDb, &GeoIpAsnDb, &TorExitNodes)> {
    let uap = ctx.user_agent_parser.as_deref()?;
    let city = ctx.geoip_city_db.as_deref()?;
    let asn = ctx.geoip_asn_db.as_deref()?;
    let tor = &*ctx.tor_exit_nodes;
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

    // Parse ?ip= to override target IP
    let (target_addr, skip_dns) = match parse_ip_param(&req_info.uri) {
        Some(ip) => {
            if !is_global_ip(ip) {
                return error_response(StatusCode::BAD_REQUEST, "private/loopback IP not allowed");
            }
            let dns_opt_in = parse_dns_param(&req_info.uri);
            (SocketAddr::new(ip, 0), !dns_opt_in)
        }
        None => (req_info.remote, false),
    };

    let ctx = state.enrichment.load();

    let (uap, city, asn, tor) = match resolve_backends(&ctx) {
        Some(backends) => backends,
        None => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "backends not available"),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;
    let ifconfig = handlers::make_ifconfig(&target_addr, &ua_opt, uap, city, asn, tor, ctx.feodo_botnet_ips.as_deref(), ctx.vpn_ranges.as_deref(), ctx.cloud_provider_db.as_deref(), ctx.datacenter_ranges.as_deref(), ctx.bot_db.as_deref(), ctx.spamhaus_drop.as_deref(), &ctx.dns_resolver, skip_dns).await;

    let fields = format::parse_fields_param(&req_info.uri);

    match format {
        NegotiatedFormat::Html => unreachable!(),
        NegotiatedFormat::Plain => respond_plain(to_plain_fn(&ifconfig)),
        NegotiatedFormat::Json => match to_json_fn(&ifconfig) {
            Some(val) => {
                let val = match &fields {
                    Some(f) => format::filter_fields(val, f),
                    None => val,
                };
                respond_json_value(val)
            }
            None => error_response(StatusCode::NOT_FOUND, "not implemented"),
        },
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match to_json_fn(&ifconfig).map(|v| match &fields {
                Some(f) => format::filter_fields(v, f),
                None => v,
            }).and_then(|v| output_fmt.serialize_body(&v)) {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => error_response(StatusCode::NOT_FOUND, "not implemented"),
            }
        }
    }
}

// ---- Root handler ----

#[utoipa::path(
    get, path = "/",
    description = "Returns the caller's full enrichment data (IP, location, ISP, network classification, user agent). Content-negotiated: returns HTML (SPA) for browsers, plain text for CLI clients, or structured data when an Accept header or format suffix is used.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
    ),
    responses(
        (status = 200, description = "Full ifconfig data", body = Ifconfig),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/ip",
    description = "Returns the caller's IP address and version (4 or 6).",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "IP address info", body = Ip),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/ip/cidr",
    description = "Returns the caller's IP in CIDR notation (e.g. 203.0.113.42/32 or 2001:db8::1/128). Plain text only.",
    responses(
        (status = 200, description = "IP in CIDR notation", content_type = "text/plain"),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn ip_cidr_handler(headers: HeaderMap, extensions: axum::http::Extensions) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let ip = req_info.remote.ip();
    let prefix_len = if ip.is_ipv4() { 32 } else { 128 };
    respond_plain(format!("{}/{}\n", ip, prefix_len))
}

#[utoipa::path(
    get, path = "/tcp",
    description = "Returns the caller's source TCP port. Omitted for ?ip= queries.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "TCP port info", body = Tcp),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/host",
    description = "Returns the reverse DNS (PTR) hostname for the caller's IP. Skipped by default for ?ip= queries unless ?dns=true.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
    ),
    responses(
        (status = 200, description = "Reverse DNS hostname", body = Host),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/location",
    description = "Returns geolocation data (city, country, coordinates, timezone) from the GeoIP database.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "Geolocation data", body = Location),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/isp",
    description = "Returns ISP name and ASN number from the GeoIP ASN database.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "ISP / ASN info", body = Isp),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/user_agent",
    description = "Returns the parsed User-Agent header (browser, OS, device families and versions).",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "Parsed User-Agent", body = UserAgent),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/network",
    description = "Returns network classification (cloud, VPN, Tor, bot, hosting, residential) with provider details and boolean flags.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "Network classification", body = Network),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn network_handler(
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
        handlers::network::to_json,
        handlers::network::to_plain,
    )
    .await
}

async fn network_format_handler(
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
        handlers::network::to_json,
        handlers::network::to_plain,
    )
    .await
}

#[utoipa::path(
    get, path = "/all",
    description = "Returns all enrichment data. Equivalent to / but always returns structured data (never HTML).",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
    ),
    responses(
        (status = 200, description = "All enrichment data", body = Ifconfig),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/headers",
    description = "Returns the request headers as received by the server (after proxy processing).",
    responses(
        (status = 200, description = "Request headers as key-value pairs"),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn headers_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let format = negotiate(None, &headers);
    let req_headers = filter_headers(extract_headers(&headers), &state.header_filters);
    dispatch_headers(format, &req_headers)
}

async fn headers_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
) -> Response {
    let format = negotiate(Some(&fmt), &headers);
    let req_headers = filter_headers(extract_headers(&headers), &state.header_filters);
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
                None => error_response(StatusCode::INTERNAL_SERVER_ERROR, "serialization failed"),
            }
        }
    }
}

// ---- Batch handler ----

#[utoipa::path(
    post, path = "/batch",
    description = "Batch-enrich multiple IPs in a single request. Each IP consumes one rate-limit token. Disabled by default (requires batch.enabled = true in config).",
    request_body(content = Vec<String>, description = "JSON array of IP address strings"),
    params(
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookups"),
    ),
    responses(
        (status = 200, description = "Array of enrichment results", body = Vec<Ifconfig>),
        (status = 400, description = "Invalid request body or empty array", body = ErrorResponse),
        (status = 404, description = "Batch endpoint is disabled", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn batch_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
    body: axum::body::Bytes,
) -> Response {
    batch_dispatch(None, &state, &headers, &extensions, &body).await
}

async fn batch_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
    body: axum::body::Bytes,
) -> Response {
    batch_dispatch(Some(&fmt), &state, &headers, &extensions, &body).await
}

async fn batch_dispatch(
    suffix: Option<&str>,
    state: &AppState,
    headers: &HeaderMap,
    extensions: &axum::http::Extensions,
    body: &[u8],
) -> Response {
    if !state.config.batch.enabled {
        return error_response(StatusCode::NOT_FOUND, "batch endpoint is disabled");
    }

    let ips: Vec<String> = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "request body must be a JSON array of IP strings");
        }
    };

    if ips.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty IP array");
    }

    if ips.len() > state.config.batch.max_size {
        return error_response(
            StatusCode::BAD_REQUEST,
            &format!("batch size {} exceeds maximum {}", ips.len(), state.config.batch.max_size),
        );
    }

    // Rate-limit: consume N tokens for N IPs
    let req_info = get_requester_info(headers, extensions);
    let caller_ip = req_info.remote.ip();
    let n = NonZeroU32::new(ips.len() as u32).unwrap_or(NonZeroU32::MIN);
    match state.rate_limiter.check_key_n(&caller_ip, n) {
        Ok(Ok(_snapshot)) => {}
        Ok(Err(not_until)) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            let retry_after = wait.as_secs().saturating_add(1);
            let limit = state.config.rate_limit.per_ip_burst;
            let mut resp = error_response(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded");
            let h = resp.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(retry_after));
            return resp;
        }
        Err(_insufficient) => {
            let limit = state.config.rate_limit.per_ip_burst;
            let mut resp = error_response(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded");
            let h = resp.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(1u64));
            return resp;
        }
    };

    let ctx = state.enrichment.load();
    let (uap, city, asn, tor) = match resolve_backends(&ctx) {
        Some(backends) => backends,
        None => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "backends not available"),
    };

    let dns_opt_in = parse_dns_param(&req_info.uri);
    let fields = format::parse_fields_param(&req_info.uri);
    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;

    let mut results: Vec<serde_json::Value> = Vec::with_capacity(ips.len());
    for ip_str in &ips {
        let safe_input: &str = if ip_str.len() > 45 { &ip_str[..45] } else { ip_str };
        let ip: IpAddr = match ip_str.parse() {
            Ok(ip) => ip,
            Err(_) => {
                results.push(json!({"error": "invalid IP address", "input": safe_input}));
                continue;
            }
        };

        if !is_global_ip(ip) {
            results.push(json!({"error": "private/loopback IP not allowed", "input": safe_input}));
            continue;
        }

        let target_addr = SocketAddr::new(ip, 0);
        let skip_dns = !dns_opt_in;
        let ifconfig = handlers::make_ifconfig(
            &target_addr, &ua_opt, uap, city, asn, tor,
            ctx.feodo_botnet_ips.as_deref(), ctx.vpn_ranges.as_deref(),
            ctx.cloud_provider_db.as_deref(), ctx.datacenter_ranges.as_deref(),
            ctx.bot_db.as_deref(), ctx.spamhaus_drop.as_deref(),
            &ctx.dns_resolver, skip_dns,
        ).await;

        let mut val = serde_json::to_value(&ifconfig).unwrap_or(json!(null));
        if let Some(ref f) = fields {
            val = format::filter_fields(val, f);
        }
        results.push(val);
    }

    let format = match suffix {
        Some(fmt) => negotiate(Some(fmt), headers),
        None => NegotiatedFormat::Json,
    };

    match format {
        NegotiatedFormat::Json | NegotiatedFormat::Html | NegotiatedFormat::Plain => {
            let arr = serde_json::Value::Array(results);
            respond_json_value(arr)
        }
        NegotiatedFormat::Yaml => {
            let arr = serde_json::Value::Array(results);
            match OutputFormat::Yaml.serialize_body(&arr) {
                Some(body) => respond_formatted(OutputFormat::Yaml.content_type(), body),
                None => error_response(StatusCode::INTERNAL_SERVER_ERROR, "serialization failed"),
            }
        }
        NegotiatedFormat::Toml => {
            // TOML doesn't support top-level arrays; wrap in a table
            let wrapped = json!({"results": results});
            match OutputFormat::Toml.serialize_body(&wrapped) {
                Some(body) => respond_formatted(OutputFormat::Toml.content_type(), body),
                None => error_response(StatusCode::INTERNAL_SERVER_ERROR, "serialization failed"),
            }
        }
        NegotiatedFormat::Csv => {
            let body = format::json_array_to_csv(&results);
            respond_formatted(OutputFormat::Csv.content_type(), body)
        }
    }
}

// ---- IP version handlers ----

#[utoipa::path(
    get, path = "/ipv4",
    description = "Returns IP info only if the caller connected via IPv4. Returns 404 for IPv6 clients.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "IPv4 address info", body = Ip),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 404, description = "Client is not using IPv4", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get, path = "/ipv6",
    description = "Returns IP info only if the caller connected via IPv6. Returns 404 for IPv4 clients.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
    ),
    responses(
        (status = 200, description = "IPv6 address info", body = Ip),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 404, description = "Client is not using IPv6", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
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

    // Parse ?ip= to override target IP
    let (target_addr, skip_dns) = match parse_ip_param(&req_info.uri) {
        Some(ip) => {
            if !is_global_ip(ip) {
                return error_response(StatusCode::BAD_REQUEST, "private/loopback IP not allowed");
            }
            let dns_opt_in = parse_dns_param(&req_info.uri);
            (SocketAddr::new(ip, 0), !dns_opt_in)
        }
        None => (req_info.remote, false),
    };

    let ctx = state.enrichment.load();

    let (uap, city, asn, tor) = match resolve_backends(&ctx) {
        Some(backends) => backends,
        None => return error_response(StatusCode::INTERNAL_SERVER_ERROR, "backends not available"),
    };

    let ua_ref = req_info.user_agent.as_deref();
    let ua_opt: Option<&str> = ua_ref;
    let ifconfig = handlers::make_ifconfig(&target_addr, &ua_opt, uap, city, asn, tor, ctx.feodo_botnet_ips.as_deref(), ctx.vpn_ranges.as_deref(), ctx.cloud_provider_db.as_deref(), ctx.datacenter_ranges.as_deref(), ctx.bot_db.as_deref(), ctx.spamhaus_drop.as_deref(), &ctx.dns_resolver, skip_dns).await;

    if ifconfig.ip.version != version {
        return error_response(StatusCode::NOT_FOUND, "not implemented");
    }

    match format {
        NegotiatedFormat::Html => unreachable!(),
        NegotiatedFormat::Plain => respond_plain(handlers::ip_version::to_plain(&ifconfig)),
        NegotiatedFormat::Json => match handlers::ip_version::to_json(&ifconfig) {
            Some(val) => respond_json_value(val),
            None => error_response(StatusCode::NOT_FOUND, "not implemented"),
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
                None => error_response(StatusCode::NOT_FOUND, "not implemented"),
            }
        }
    }
}

// ---- Meta handler (site info for SPA) ----

async fn meta_handler(State(state): State<AppState>) -> Response {
    (StatusCode::OK, axum::Json(&*state.project_info)).into_response()
}

// ---- Health handler ----

#[utoipa::path(
    get, path = "/health",
    description = "Liveness probe. Always returns 200 with {\"status\": \"ok\"}. Exempt from rate limiting.",
    responses((status = 200, description = "Liveness probe"))
)]
async fn health_handler() -> Response {
    (StatusCode::OK, axum::Json(json!({ "status": "ok" }))).into_response()
}

// ---- Readiness handler ----

#[utoipa::path(
    get, path = "/ready",
    description = "Readiness probe. Returns 200 when GeoIP databases and UA parser are loaded, 503 otherwise. Exempt from rate limiting.",
    responses(
        (status = 200, description = "All backends loaded"),
        (status = 503, description = "One or more backends not ready"),
    )
)]
async fn ready_handler(State(state): State<AppState>) -> Response {
    let ctx = state.enrichment.load();
    let has_city_db = ctx.geoip_city_db.is_some();
    let has_asn_db = ctx.geoip_asn_db.is_some();
    let has_ua_parser = ctx.user_agent_parser.is_some();

    if has_city_db && has_asn_db && has_ua_parser {
        (StatusCode::OK, axum::Json(json!({ "status": "ready" }))).into_response()
    } else {
        let mut missing = Vec::new();
        if !has_city_db {
            missing.push("GeoIP City database not loaded");
        }
        if !has_asn_db {
            missing.push("GeoIP ASN database not loaded");
        }
        if !has_ua_parser {
            missing.push("User-Agent parser not loaded");
        }
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({
                "status": "not_ready",
                "reason": missing.join("; ")
            })),
        )
            .into_response()
    }
}

// ---- OpenAPI spec handler ----

async fn openapi_handler() -> Response {
    let mut spec = ApiDoc::openapi();
    spec.info.version = env!("CARGO_PKG_VERSION").to_string();
    let json = spec.to_pretty_json().unwrap_or_default();
    respond_formatted("application/json", json)
}

// ---- API docs UI handler ----

async fn docs_handler() -> Response {
    Html(include_str!("scalar_docs.html")).into_response()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ip_param_v4() {
        assert_eq!(
            parse_ip_param("/all/json?ip=8.8.8.8"),
            Some("8.8.8.8".parse().unwrap())
        );
    }

    #[test]
    fn parse_ip_param_v6() {
        assert_eq!(
            parse_ip_param("/all/json?ip=2001:db8::1"),
            Some("2001:db8::1".parse().unwrap())
        );
    }

    #[test]
    fn parse_ip_param_missing() {
        assert_eq!(parse_ip_param("/all/json"), None);
        assert_eq!(parse_ip_param("/all/json?fields=ip"), None);
    }

    #[test]
    fn parse_ip_param_invalid() {
        assert_eq!(parse_ip_param("/all/json?ip=notanip"), None);
    }

    #[test]
    fn parse_dns_param_values() {
        assert!(parse_dns_param("/all/json?ip=8.8.8.8&dns=true"));
        assert!(parse_dns_param("/all/json?dns=1&ip=8.8.8.8"));
        assert!(!parse_dns_param("/all/json?ip=8.8.8.8"));
        assert!(!parse_dns_param("/all/json?ip=8.8.8.8&dns=false"));
    }

    #[test]
    fn is_global_ip_rejects_private() {
        assert!(!is_global_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_global_ip("10.0.0.1".parse().unwrap()));
        assert!(!is_global_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_global_ip("172.16.0.1".parse().unwrap()));
        assert!(!is_global_ip("169.254.1.1".parse().unwrap()));
        assert!(!is_global_ip("0.0.0.0".parse().unwrap()));
        assert!(!is_global_ip("::1".parse().unwrap()));
        assert!(!is_global_ip("::".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_accepts_public() {
        assert!(is_global_ip("8.8.8.8".parse().unwrap()));
        assert!(is_global_ip("1.1.1.1".parse().unwrap()));
        assert!(is_global_ip("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_rejects_ipv6_private() {
        assert!(!is_global_ip("fc00::1".parse().unwrap()));
        assert!(!is_global_ip("fd12::1".parse().unwrap()));
        assert!(!is_global_ip("fe80::1".parse().unwrap()));
        assert!(!is_global_ip("ff02::1".parse().unwrap()));
        assert!(!is_global_ip("::ffff:10.0.0.1".parse().unwrap()));
        assert!(!is_global_ip("::ffff:192.168.1.1".parse().unwrap()));
        assert!(!is_global_ip("::ffff:172.16.0.1".parse().unwrap()));
        assert!(!is_global_ip("::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_accepts_public_ipv6() {
        assert!(is_global_ip("2606:4700::1111".parse().unwrap()));
        assert!(is_global_ip("::ffff:8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn openapi_version_matches_cargo_pkg() {
        let mut spec = ApiDoc::openapi();
        spec.info.version = env!("CARGO_PKG_VERSION").to_string();
        assert_eq!(spec.info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn openapi_spec_contains_error_schema_and_examples() {
        let spec = ApiDoc::openapi();
        let json = spec.to_pretty_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let schemas = &parsed["components"]["schemas"];
        // ErrorResponse schema exists
        assert!(schemas.get("ErrorResponse").is_some(), "ErrorResponse schema missing");
        // Ip schema has example on addr field
        let ip_addr = &schemas["Ip"]["properties"]["addr"];
        assert!(ip_addr.get("example").is_some(), "Ip.addr example missing");
        // Location schema has example on city field
        let loc_city = &schemas["Location"]["properties"]["city"];
        assert!(loc_city.get("example").is_some(), "Location.city example missing");
    }
}
