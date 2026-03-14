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
use crate::error::{error_response, AppError, ErrorResponse};
use crate::extractors::{extract_headers, filter_headers, RequesterInfo};
use crate::format::{self, OutputFormat};
use crate::handlers;
use crate::negotiate::{negotiate, NegotiatedFormat};
use crate::state::AppState;

fn check_target_rate_limit(state: &AppState, target_ip: IpAddr) -> Option<Response> {
    if state.target_rate_limiter.check_key(&target_ip).is_err() {
        Some(AppError::RateLimited { retry_after_secs: 1 }.into_response())
    } else {
        None
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ifconfig-rs",
        description = "IP address lookup and enrichment API.\n\n\
            ## Rate Limiting\n\
            All endpoints (except `/health`, `/ready`) are rate-limited per source IP.\n\
            Every response includes:\n\
            - `X-RateLimit-Limit` — requests allowed per minute\n\
            - `X-RateLimit-Remaining` — tokens left in the current window\n\n\
            When the limit is exceeded (HTTP 429):\n\
            - `Retry-After` — seconds until a new token is available\n\
            - `X-RateLimit-Reset` — Unix timestamp when the limit resets",
        version = "0.8.0",
        license(name = "MIT"),
    ),
    paths(
        root_handler,
        ip_handler,
        ip_cidr_handler,
        tcp_handler,
        location_handler,
        user_agent_handler,
        network_handler,
        all_handler,
        headers_handler,
        ipv4_handler,
        ipv6_handler,
        country_handler,
        city_handler,
        asn_handler,
        timezone_handler,
        latitude_handler,
        longitude_handler,
        region_handler,
        batch_handler,
        meta_handler,
        health_handler,
        ready_handler,
    ),
    components(schemas(
        Ifconfig, Ip, Tcp, Location, Network, CloudInfo, VpnInfo, NetworkBot,
        UserAgent, Browser, OS, Device, crate::error::ErrorInfo, ErrorResponse,
        MetaResponse, DataSources,
        crate::state::RateLimitInfo, crate::state::BatchInfo, crate::state::BuildInfo,
    )),
    tags(
        (name = "IP", description = "IP address lookup and version endpoints"),
        (name = "Location", description = "Geolocation data from GeoIP databases"),
        (name = "Network", description = "Network classification (cloud, VPN, Tor, bot, hosting) with ASN and org"),
        (name = "TCP", description = "Source TCP port of the connection"),
        (name = "User Agent", description = "Parsed User-Agent header"),
        (name = "Headers", description = "Raw request headers as received by the server"),
        (name = "Batch", description = "Batch enrichment of multiple IPs"),
        (name = "Probes", description = "Liveness, readiness, and site metadata endpoints"),
    )
)]
struct ApiDoc;

/// Build the main router.
pub fn router() -> Router<AppState> {
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
        .route("/location", get(location_handler))
        .route("/location/{fmt}", get(location_format_handler))
        .route("/user_agent", get(user_agent_handler))
        .route("/user_agent/{fmt}", get(user_agent_format_handler))
        .route("/network", get(network_handler))
        .route("/network/{fmt}", get(network_format_handler))
        .route("/all", get(all_handler))
        .route("/all/{fmt}", get(all_format_handler))
        // Sub-field endpoints
        .route("/country", get(country_handler))
        .route("/country/{fmt}", get(country_format_handler))
        .route("/city", get(city_handler))
        .route("/city/{fmt}", get(city_format_handler))
        .route("/asn", get(asn_handler))
        .route("/asn/{fmt}", get(asn_format_handler))
        .route("/timezone", get(timezone_handler))
        .route("/timezone/{fmt}", get(timezone_format_handler))
        .route("/latitude", get(latitude_handler))
        .route("/latitude/{fmt}", get(latitude_format_handler))
        .route("/longitude", get(longitude_handler))
        .route("/longitude/{fmt}", get(longitude_format_handler))
        .route("/region", get(region_handler))
        .route("/region/{fmt}", get(region_format_handler))
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
    match Assets::get("index.html") {
        Some(content) => {
            let body = std::str::from_utf8(content.data.as_ref()).unwrap_or("");
            Html(body.to_string()).into_response()
        }
        None => (StatusCode::INTERNAL_SERVER_ERROR, "SPA not found").into_response(),
    }
}

// ---- Query parameter helpers ----

fn format_from_query(uri: &axum::http::Uri) -> Option<String> {
    let query = uri.query()?;
    query
        .split('&')
        .find_map(|p| p.strip_prefix("format=").map(|v| v.to_string()))
}

fn parse_query_param<'a>(uri: &'a str, key: &str) -> Option<&'a str> {
    let query = uri.split('?').nth(1)?;
    query
        .split('&')
        .find_map(|p| p.strip_prefix(key).and_then(|rest| rest.strip_prefix('=')))
}

fn parse_ip_param(uri: &str) -> Option<IpAddr> {
    parse_query_param(uri, "ip").and_then(|s| {
        if s.contains('%') {
            s.replace("%3A", ":").replace("%3a", ":").parse().ok()
        } else {
            s.parse().ok()
        }
    })
}

fn parse_dns_param(uri: &str) -> Option<bool> {
    parse_query_param(uri, "dns").map(|v| v.eq_ignore_ascii_case("true") || v == "1")
}

// ---- Compute-once dispatch ----

fn resolve_core_backends(
    ctx: &EnrichmentContext,
) -> (
    Option<&UserAgentParser>,
    Option<&GeoIpCityDb>,
    Option<&GeoIpAsnDb>,
    &TorExitNodes,
) {
    (
        ctx.user_agent_parser.as_deref(),
        ctx.geoip_city_db.as_deref(),
        ctx.geoip_asn_db.as_deref(),
        &*ctx.tor_exit_nodes,
    )
}

async fn dispatch_standard(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    to_json_fn: fn(&Ifconfig) -> Option<serde_json::Value>,
    to_plain_fn: fn(&Ifconfig) -> String,
) -> Response {
    dispatch(format, req_info, state, to_json_fn, to_plain_fn).await
}

/// Core dispatch: build an `Ifconfig` and render it in the requested format.
///
/// Accepts closures for JSON and plain-text building so callers can inject extra
/// content (e.g. `/all` appends request headers into the JSON and plain output).
/// `dispatch_standard` is the thin wrapper for the common fn-pointer callers.
///
/// Note: `ip_version_dispatch` is intentionally kept separate — it skips
/// `?fields=` filtering and adds a post-build IP-version check, which would
/// make a unified implementation more complex than the duplication.
async fn dispatch<F, G>(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    to_json_fn: F,
    to_plain_fn: G,
) -> Response
where
    F: Fn(&Ifconfig) -> Option<serde_json::Value>,
    G: Fn(&Ifconfig) -> String,
{
    if format == NegotiatedFormat::Html {
        return serve_spa();
    }

    if format == NegotiatedFormat::Unknown {
        return error_response(StatusCode::NOT_FOUND, "INVALID_FORMAT", "unknown format suffix");
    }

    // Parse ?ip= to override target IP
    let (target_addr, skip_dns) = match parse_ip_param(&req_info.uri) {
        Some(ip) => {
            if !state.config.internal_mode && !is_global_ip(ip) {
                return error_response(StatusCode::BAD_REQUEST, "INVALID_IP", "private/loopback IP not allowed");
            }
            let skip_dns = parse_dns_param(&req_info.uri).map(|v| !v).unwrap_or(false);
            (SocketAddr::new(ip, 0), skip_dns)
        }
        None => (req_info.remote, false),
    };

    if let Some(resp) = check_target_rate_limit(state, target_addr.ip()) {
        return resp;
    }

    let ctx = state.enrichment.load();

    let (uap, city, asn, tor) = resolve_core_backends(&ctx);

    let ua_opt = req_info.user_agent.as_deref();
    let ifconfig = handlers::make_ifconfig(
        &target_addr,
        &ua_opt,
        uap,
        city,
        asn,
        tor,
        ctx.feodo_botnet_ips.as_deref(),
        ctx.vpn_ranges.as_deref(),
        ctx.cloud_provider_db.as_deref(),
        ctx.datacenter_ranges.as_deref(),
        ctx.bot_db.as_deref(),
        ctx.spamhaus_drop.as_deref(),
        &ctx.dns_resolver,
        &state.dns_cache,
        skip_dns,
        ctx.asn_patterns.as_ref(),
        ctx.asn_info.as_deref(),
    )
    .await;

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
            None => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "not implemented"),
        },
        fmt => {
            let output_fmt = match fmt {
                NegotiatedFormat::Yaml => OutputFormat::Yaml,
                NegotiatedFormat::Toml => OutputFormat::Toml,
                NegotiatedFormat::Csv => OutputFormat::Csv,
                _ => unreachable!(),
            };
            match to_json_fn(&ifconfig)
                .map(|v| match &fields {
                    Some(f) => format::filter_fields(v, f),
                    None => v,
                })
                .and_then(|v| output_fmt.serialize_body(&v))
            {
                Some(body) => respond_formatted(output_fmt.content_type(), body),
                None => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "not implemented"),
            }
        }
    }
}

// ---- Root handler ----

#[utoipa::path(
    get, path = "/",
    tag = "IP",
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
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let fmt_query = format_from_query(&uri);
    let format = negotiate(fmt_query.as_deref(), &headers);
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
//
// Each standard endpoint follows the same pattern: negotiate format, build
// requester info, delegate to dispatch_standard with module-specific
// to_json/to_plain functions. The macro below eliminates the per-endpoint
// boilerplate while preserving the utoipa annotations for OpenAPI generation.

macro_rules! standard_endpoint {
    (
        $(#[$meta:meta])*
        handler = $handler:ident,
        format_handler = $format_handler:ident,
        module = $($module:ident)::+ $(,)?
    ) => {
        $(#[$meta])*
        async fn $handler(
            State(state): State<AppState>,
            uri: axum::http::Uri,
            headers: HeaderMap,
            extensions: axum::http::Extensions,
        ) -> Response {
            let req_info = get_requester_info(&headers, &extensions);
            let fmt_query = format_from_query(&uri);
            let format = negotiate(fmt_query.as_deref(), &headers);
            dispatch_standard(format, &req_info, &state, $($module)::+::to_json, $($module)::+::to_plain).await
        }

        async fn $format_handler(
            State(state): State<AppState>,
            Path(fmt): Path<String>,
            headers: HeaderMap,
            extensions: axum::http::Extensions,
        ) -> Response {
            let req_info = get_requester_info(&headers, &extensions);
            let format = negotiate(Some(&fmt), &headers);
            dispatch_standard(format, &req_info, &state, $($module)::+::to_json, $($module)::+::to_plain).await
        }
    };
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/ip",
        tag = "IP",
        description = "Returns the caller's IP address and version (4 or 6).",
        params(
            ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
            ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
            ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
        ),
        responses(
            (status = 200, description = "IP address info", body = Ip),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = ip_handler,
    format_handler = ip_format_handler,
    module = handlers::ip,
}

#[utoipa::path(
    get, path = "/ip/cidr",
    tag = "IP",
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

standard_endpoint! {
    #[utoipa::path(
        get, path = "/tcp",
        tag = "TCP",
        description = "Returns the caller's source TCP port. Omitted for ?ip= queries.",
        params(
            ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
            ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
            ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
        ),
        responses(
            (status = 200, description = "TCP port info", body = Tcp),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = tcp_handler,
    format_handler = tcp_format_handler,
    module = handlers::tcp,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/location",
        tag = "Location",
        description = "Returns geolocation data (city, country, coordinates, timezone) from the GeoIP database.",
        params(
            ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
            ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
            ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
        ),
        responses(
            (status = 200, description = "Geolocation data", body = Location),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = location_handler,
    format_handler = location_format_handler,
    module = handlers::location,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/user_agent",
        tag = "User Agent",
        description = "Returns the parsed User-Agent header (browser, OS, device families and versions).",
        params(
            ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
            ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
            ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
        ),
        responses(
            (status = 200, description = "Parsed User-Agent", body = UserAgent),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = user_agent_handler,
    format_handler = user_agent_format_handler,
    module = handlers::user_agent,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/network",
        tag = "Network",
        description = "Returns network classification (cloud, VPN, Tor, bot, hosting, residential) with provider details and boolean flags.",
        params(
            ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
            ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
            ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
        ),
        responses(
            (status = 200, description = "Network classification", body = Network),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = network_handler,
    format_handler = network_format_handler,
    module = handlers::network,
}

// ---- /all handler (custom: includes request headers in response) ----

#[utoipa::path(
    get, path = "/all",
    tag = "IP",
    description = "Returns all enrichment data including request headers. Equivalent to / but always returns structured data (never HTML). The JSON response includes a top-level `headers` object with the filtered request headers.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
    ),
    responses(
        (status = 200, description = "All enrichment data plus request headers", body = Ifconfig),
        (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn all_handler(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let fmt_query = format_from_query(&uri);
    let format = negotiate(fmt_query.as_deref(), &headers);
    let req_headers = filter_headers(extract_headers(&headers), &state.header_filters);
    dispatch_all(format, &req_info, &state, &req_headers).await
}

async fn all_format_handler(
    State(state): State<AppState>,
    Path(fmt): Path<String>,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let req_info = get_requester_info(&headers, &extensions);
    let format = negotiate(Some(&fmt), &headers);
    let req_headers = filter_headers(extract_headers(&headers), &state.header_filters);
    dispatch_all(format, &req_info, &state, &req_headers).await
}

async fn dispatch_all(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    req_headers: &[(String, String)],
) -> Response {
    // /all differs from standard endpoints only in how JSON and plain-text are built:
    // the request headers are merged into the JSON object and appended to plain output.
    dispatch(
        format,
        req_info,
        state,
        |ifconfig| {
            let mut val = handlers::all::to_json(ifconfig)?;
            if let serde_json::Value::Object(ref mut map) = val {
                map.insert("headers".to_string(), handlers::headers::to_json_value(req_headers));
            }
            Some(val)
        },
        |ifconfig| {
            let mut text = handlers::all::to_plain(ifconfig);
            text.push_str(&handlers::headers::to_plain(req_headers));
            text
        },
    )
    .await
}

// ---- Sub-field endpoints ----

standard_endpoint! {
    #[utoipa::path(
        get, path = "/country",
        tag = "Location",
        description = "Returns the caller's country name as plain text.",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "Country name", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = country_handler,
    format_handler = country_format_handler,
    module = handlers::country,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/city",
        tag = "Location",
        description = "Returns the caller's city name as plain text.",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "City name", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = city_handler,
    format_handler = city_format_handler,
    module = handlers::city,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/asn",
        tag = "Network",
        description = "Returns the ASN of the caller's IP as plain text (e.g. AS64496).",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "ASN", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = asn_handler,
    format_handler = asn_format_handler,
    module = handlers::asn,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/timezone",
        tag = "Location",
        description = "Returns the caller's timezone as plain text (e.g. Europe/Berlin).",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "Timezone", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = timezone_handler,
    format_handler = timezone_format_handler,
    module = handlers::timezone,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/latitude",
        tag = "Location",
        description = "Returns the caller's latitude as plain text.",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "Latitude", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = latitude_handler,
    format_handler = latitude_format_handler,
    module = handlers::latitude,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/longitude",
        tag = "Location",
        description = "Returns the caller's longitude as plain text.",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "Longitude", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = longitude_handler,
    format_handler = longitude_format_handler,
    module = handlers::longitude,
}

standard_endpoint! {
    #[utoipa::path(
        get, path = "/region",
        tag = "Location",
        description = "Returns the caller's region/state name as plain text.",
        params(("ip" = Option<String>, Query, description = "Look up this IP instead of caller's")),
        responses(
            (status = 200, description = "Region name", content_type = "text/plain"),
            (status = 400, description = "Invalid IP parameter", body = ErrorResponse),
            (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        )
    )]
    handler = region_handler,
    format_handler = region_format_handler,
    module = handlers::region,
}

// ---- Headers handler ----

#[utoipa::path(
    get, path = "/headers",
    tag = "Headers",
    description = "Returns the request headers as received by the server (after proxy processing).",
    responses(
        (status = 200, description = "Request headers as key-value pairs"),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
    )
)]
async fn headers_handler(State(state): State<AppState>, uri: axum::http::Uri, headers: HeaderMap) -> Response {
    let fmt_query = format_from_query(&uri);
    let format = negotiate(fmt_query.as_deref(), &headers);
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
        NegotiatedFormat::Unknown => error_response(StatusCode::NOT_FOUND, "INVALID_FORMAT", "unknown format suffix"),
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
                None => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "serialization failed",
                ),
            }
        }
    }
}

// ---- Batch handler ----

#[utoipa::path(
    post, path = "/batch",
    tag = "Batch",
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
        return error_response(StatusCode::NOT_FOUND, "BATCH_DISABLED", "batch endpoint is disabled");
    }

    let ips: Vec<String> = match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_FORMAT",
                "request body must be a JSON array of IP strings",
            );
        }
    };

    if ips.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "INVALID_FORMAT", "empty IP array");
    }

    if ips.len() > state.config.batch.max_size {
        return error_response(StatusCode::BAD_REQUEST, "BATCH_TOO_MANY", "batch size exceeds limit");
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
            let limit = state.config.rate_limit.per_ip_per_minute;
            let reset_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_add(retry_after);
            let mut resp = error_response(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "rate limit exceeded");
            let h = resp.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(retry_after));
            h.insert("x-ratelimit-reset", HeaderValue::from(reset_unix));
            return resp;
        }
        Err(_insufficient) => {
            let limit = state.config.rate_limit.per_ip_per_minute;
            let reset_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_add(1);
            let mut resp = error_response(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", "rate limit exceeded");
            let h = resp.headers_mut();
            h.insert("x-ratelimit-limit", HeaderValue::from(limit));
            h.insert("x-ratelimit-remaining", HeaderValue::from(0u32));
            h.insert("retry-after", HeaderValue::from(1u64));
            h.insert("x-ratelimit-reset", HeaderValue::from(reset_unix));
            return resp;
        }
    };

    let ctx: std::sync::Arc<EnrichmentContext> = std::sync::Arc::clone(&*state.enrichment.load());

    let fields = format::parse_fields_param(&req_info.uri);
    let skip_dns = !parse_dns_param(&req_info.uri).unwrap_or(false);
    let ua_owned: Option<String> = req_info.user_agent.clone();
    let dns_cache = state.dns_cache.clone();

    // Pre-validate IPs and spawn concurrent lookups
    let mut results: Vec<serde_json::Value> = vec![json!(null); ips.len()];
    let mut set = tokio::task::JoinSet::new();
    // Bound concurrency to avoid overwhelming the DNS resolver and GeoIP databases.
    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(10));

    for (i, ip_str) in ips.iter().enumerate() {
        let ip: IpAddr = match ip_str.parse() {
            Ok(ip) => ip,
            Err(_) => {
                results[i] = json!({"error": {"code": "INVALID_IP", "message": "invalid IP address"}, "index": i});
                continue;
            }
        };

        if !state.config.internal_mode && !is_global_ip(ip) {
            results[i] =
                json!({"error": {"code": "INVALID_IP", "message": "private/loopback IP not allowed"}, "index": i});
            continue;
        }

        if state.target_rate_limiter.check_key(&ip).is_err() {
            results[i] =
                json!({"error": {"code": "RATE_LIMITED", "message": "target rate limit exceeded"}, "index": i});
            continue;
        }

        let ctx = std::sync::Arc::clone(&ctx);
        let ua = ua_owned.clone();
        let fields = fields.clone();
        let dns_cache = dns_cache.clone();
        let permit = std::sync::Arc::clone(&sem).acquire_owned().await.unwrap();
        set.spawn(async move {
            let _permit = permit;
            let uap = ctx.user_agent_parser.as_deref();
            let city = ctx.geoip_city_db.as_deref();
            let asn = ctx.geoip_asn_db.as_deref();
            let tor = &*ctx.tor_exit_nodes;
            let ua_ref = ua.as_deref();
            let target_addr = SocketAddr::new(ip, 0);

            let lookup = handlers::make_ifconfig(
                &target_addr,
                &ua_ref,
                uap,
                city,
                asn,
                tor,
                ctx.feodo_botnet_ips.as_deref(),
                ctx.vpn_ranges.as_deref(),
                ctx.cloud_provider_db.as_deref(),
                ctx.datacenter_ranges.as_deref(),
                ctx.bot_db.as_deref(),
                ctx.spamhaus_drop.as_deref(),
                &ctx.dns_resolver,
                &dns_cache,
                skip_dns,
                ctx.asn_patterns.as_ref(),
                ctx.asn_info.as_deref(),
            );
            let ifconfig = match tokio::time::timeout(std::time::Duration::from_secs(5), lookup).await {
                Ok(result) => result,
                Err(_) => {
                    return (
                        i,
                        json!({"error": {"code": "TIMEOUT", "message": "lookup timed out"}, "index": i}),
                    )
                }
            };

            let mut val = serde_json::to_value(&ifconfig).unwrap_or(json!(null));
            if let Some(ref f) = fields {
                val = format::filter_fields(val, f);
            }
            (i, val)
        });
    }

    while let Some(res) = set.join_next().await {
        if let Ok((i, val)) = res {
            results[i] = val;
        }
    }

    let format = match suffix {
        Some(fmt) => negotiate(Some(fmt), headers),
        None => NegotiatedFormat::Json,
    };

    match format {
        NegotiatedFormat::Json | NegotiatedFormat::Html | NegotiatedFormat::Plain | NegotiatedFormat::Unknown => {
            let arr = serde_json::Value::Array(results);
            respond_json_value(arr)
        }
        NegotiatedFormat::Yaml => {
            let arr = serde_json::Value::Array(results);
            match OutputFormat::Yaml.serialize_body(&arr) {
                Some(body) => respond_formatted(OutputFormat::Yaml.content_type(), body),
                None => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "serialization failed",
                ),
            }
        }
        NegotiatedFormat::Toml => {
            // TOML doesn't support top-level arrays; wrap in a table
            let wrapped = json!({"results": results});
            match OutputFormat::Toml.serialize_body(&wrapped) {
                Some(body) => respond_formatted(OutputFormat::Toml.content_type(), body),
                None => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "serialization failed",
                ),
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
    tag = "IP",
    description = "Returns IP info only if the caller connected via IPv4. Returns 404 for IPv6 clients.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
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
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let fmt_query = format_from_query(&uri);
    ip_version_dispatch("4", fmt_query.as_deref(), &state, &headers, &extensions).await
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
    tag = "IP",
    description = "Returns IP info only if the caller connected via IPv6. Returns 404 for IPv4 clients.",
    params(
        ("ip" = Option<String>, Query, description = "Look up this IP instead of caller's"),
        ("fields" = Option<String>, Query, description = "Comma-separated field names to include"),
        ("dns" = Option<String>, Query, description = "Set to 'true' to enable PTR lookup for ?ip= queries"),
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
    uri: axum::http::Uri,
    headers: HeaderMap,
    extensions: axum::http::Extensions,
) -> Response {
    let fmt_query = format_from_query(&uri);
    ip_version_dispatch("6", fmt_query.as_deref(), &state, &headers, &extensions).await
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

    if format == NegotiatedFormat::Unknown {
        return error_response(StatusCode::NOT_FOUND, "INVALID_FORMAT", "unknown format suffix");
    }

    // Parse ?ip= to override target IP
    let (target_addr, skip_dns) = match parse_ip_param(&req_info.uri) {
        Some(ip) => {
            if !state.config.internal_mode && !is_global_ip(ip) {
                return error_response(StatusCode::BAD_REQUEST, "INVALID_IP", "private/loopback IP not allowed");
            }
            let skip_dns = parse_dns_param(&req_info.uri).map(|v| !v).unwrap_or(false);
            (SocketAddr::new(ip, 0), skip_dns)
        }
        None => (req_info.remote, false),
    };

    if let Some(resp) = check_target_rate_limit(state, target_addr.ip()) {
        return resp;
    }

    let ctx = state.enrichment.load();

    let (uap, city, asn, tor) = resolve_core_backends(&ctx);

    let ua_opt = req_info.user_agent.as_deref();
    let ifconfig = handlers::make_ifconfig(
        &target_addr,
        &ua_opt,
        uap,
        city,
        asn,
        tor,
        ctx.feodo_botnet_ips.as_deref(),
        ctx.vpn_ranges.as_deref(),
        ctx.cloud_provider_db.as_deref(),
        ctx.datacenter_ranges.as_deref(),
        ctx.bot_db.as_deref(),
        ctx.spamhaus_drop.as_deref(),
        &ctx.dns_resolver,
        &state.dns_cache,
        skip_dns,
        ctx.asn_patterns.as_ref(),
        ctx.asn_info.as_deref(),
    )
    .await;

    if ifconfig.ip.version != version {
        return error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "not implemented");
    }

    match format {
        NegotiatedFormat::Html => unreachable!(),
        NegotiatedFormat::Plain => respond_plain(handlers::ip_version::to_plain(&ifconfig)),
        NegotiatedFormat::Json => match handlers::ip_version::to_json(&ifconfig) {
            Some(val) => respond_json_value(val),
            None => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "not implemented"),
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
                None => error_response(StatusCode::NOT_FOUND, "NOT_FOUND", "not implemented"),
            }
        }
    }
}

// ---- Meta handler (site info for SPA) ----

#[derive(serde::Serialize, utoipa::ToSchema)]
struct DataSources {
    geoip_city: bool,
    geoip_asn: bool,
    user_agent: bool,
    tor: bool,
    vpn: bool,
    cloud: bool,
    datacenter: bool,
    bot: bool,
    feodo: bool,
    spamhaus: bool,
    asn_info: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    geoip_city_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    geoip_asn_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_agent_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tor_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vpn_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cloud_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    datacenter_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bot_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    feodo_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spamhaus_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    asn_info_updated: Option<String>,
}

/// Converts a Unix epoch (seconds) to an ISO 8601 date string (`YYYY-MM-DD`).
/// Uses the civil_from_days algorithm (Howard Hinnant) — no external dependencies.
fn epoch_to_iso_date(epoch_secs: u64) -> String {
    let z = (epoch_secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct MetaResponse<'a> {
    name: &'a str,
    version: &'a str,
    base_url: &'a str,
    site_name: &'a str,
    batch: &'a crate::state::BatchInfo,
    rate_limit: &'a crate::state::RateLimitInfo,
    data_sources: DataSources,
    /// ISO 8601 date of the loaded GeoIP City database build, or `null` if not loaded.
    geoip_database_date: Option<String>,
    build: &'a crate::state::BuildInfo,
}

#[utoipa::path(
    get, path = "/meta",
    tag = "Probes",
    description = "Returns site metadata used by the SPA: project name, version, base URL, site name, \
        batch availability, rate limit settings, loaded data sources, and build info.",
    responses(
        (status = 200, description = "Site metadata", body = MetaResponse),
    )
)]
async fn meta_handler(State(state): State<AppState>) -> Response {
    let info = &*state.project_info;
    let ctx = state.enrichment.load();
    let response = MetaResponse {
        name: &info.name,
        version: &info.version,
        base_url: &info.base_url,
        site_name: &info.site_name,
        batch: &info.batch,
        rate_limit: &info.rate_limit,
        data_sources: DataSources {
            geoip_city: ctx.geoip_city_db.is_some(),
            geoip_asn: ctx.geoip_asn_db.is_some(),
            user_agent: ctx.user_agent_parser.is_some(),
            tor: ctx.tor_exit_nodes.is_loaded(),
            vpn: ctx.vpn_ranges.is_some(),
            cloud: ctx.cloud_provider_db.is_some(),
            datacenter: ctx.datacenter_ranges.is_some(),
            bot: ctx.bot_db.is_some(),
            feodo: ctx.feodo_botnet_ips.is_some(),
            spamhaus: ctx.spamhaus_drop.is_some(),
            asn_info: ctx.asn_info.is_some(),
            geoip_city_updated: ctx.data_file_dates.geoip_city.clone(),
            geoip_asn_updated: ctx.data_file_dates.geoip_asn.clone(),
            user_agent_updated: ctx.data_file_dates.user_agent.clone(),
            tor_updated: ctx.data_file_dates.tor.clone(),
            vpn_updated: ctx.data_file_dates.vpn.clone(),
            cloud_updated: ctx.data_file_dates.cloud.clone(),
            datacenter_updated: ctx.data_file_dates.datacenter.clone(),
            bot_updated: ctx.data_file_dates.bot.clone(),
            feodo_updated: ctx.data_file_dates.feodo.clone(),
            spamhaus_updated: ctx.data_file_dates.spamhaus.clone(),
            asn_info_updated: ctx.data_file_dates.asn_info.clone(),
        },
        geoip_database_date: ctx.geoip_city_build_epoch.map(epoch_to_iso_date),
        build: &info.build,
    };
    (StatusCode::OK, axum::Json(response)).into_response()
}

// ---- Health handler ----

#[utoipa::path(
    get, path = "/health",
    tag = "Probes",
    description = "Liveness probe. Always returns 200 with {\"status\": \"ok\"}. Exempt from rate limiting.",
    responses((status = 200, description = "Liveness probe"))
)]
async fn health_handler() -> Response {
    (StatusCode::OK, axum::Json(json!({ "status": "ok" }))).into_response()
}

// ---- Readiness handler ----

#[utoipa::path(
    get, path = "/ready",
    tag = "Probes",
    description = "Readiness probe. Returns 200 when GeoIP databases and UA parser are loaded, 503 otherwise. \
        A 200 response may include a `warnings` array listing optional data sources that were configured but \
        failed to load (e.g. datacenter ranges, bot ranges, Spamhaus DROP). Exempt from rate limiting.",
    responses(
        (status = 200, description = "Core backends loaded; optional `warnings` array lists any missing optional sources"),
        (status = 503, description = "One or more core backends not ready"),
    )
)]
pub async fn ready_handler(State(state): State<AppState>) -> Response {
    let ctx = state.enrichment.load();
    let has_city_db = ctx.geoip_city_db.is_some();
    let has_asn_db = ctx.geoip_asn_db.is_some();
    let has_ua_parser = ctx.user_agent_parser.is_some();

    if has_city_db && has_asn_db && has_ua_parser {
        let warnings: Vec<&str> = ctx.missing_optional.to_vec();
        if warnings.is_empty() {
            (StatusCode::OK, axum::Json(json!({ "status": "ready" }))).into_response()
        } else {
            (
                StatusCode::OK,
                axum::Json(json!({ "status": "ready", "warnings": warnings })),
            )
                .into_response()
        }
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
pub struct Assets;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_to_iso_date_known_values() {
        assert_eq!(epoch_to_iso_date(0), "1970-01-01");
        assert_eq!(epoch_to_iso_date(86400), "1970-01-02");
        assert_eq!(epoch_to_iso_date(1_000_000_000), "2001-09-09");
        assert_eq!(epoch_to_iso_date(1_700_000_000), "2023-11-14");
        assert_eq!(epoch_to_iso_date(1_740_268_800), "2025-02-23");
    }

    #[test]
    fn parse_ip_param_v4() {
        assert_eq!(parse_ip_param("/all/json?ip=8.8.8.8"), Some("8.8.8.8".parse().unwrap()));
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
        assert_eq!(parse_dns_param("/all/json?ip=8.8.8.8&dns=true"), Some(true));
        assert_eq!(parse_dns_param("/all/json?dns=1&ip=8.8.8.8"), Some(true));
        assert_eq!(parse_dns_param("/all/json?ip=8.8.8.8"), None);
        assert_eq!(parse_dns_param("/all/json?ip=8.8.8.8&dns=false"), Some(false));
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
