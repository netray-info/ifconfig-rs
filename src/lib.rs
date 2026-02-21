pub mod backend;
pub mod config;
pub mod enrichment;
pub mod error;
pub mod extractors;
pub mod format;
pub mod handlers;
pub mod middleware;
pub mod negotiate;
pub mod routes;
pub mod state;

use arc_swap::ArcSwap;
use axum::extract::DefaultBodyLimit;
use axum::middleware as axum_mw;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use enrichment::EnrichmentContext;
use state::AppState;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use state::ProjectInfo;

/// Middleware that requires a valid `Authorization: Bearer <token>` header.
/// Used to protect the admin port when `server.admin_token` is configured.
async fn admin_bearer_auth(
    axum::extract::State(expected): axum::extract::State<String>,
    req: axum::http::Request<axum::body::Body>,
    next: axum_mw::Next,
) -> axum::response::Response {
    let ok = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|t| t == expected)
        .unwrap_or(false);
    if ok {
        next.run(req).await
    } else {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            [(
                axum::http::header::WWW_AUTHENTICATE,
                axum::http::HeaderValue::from_static("Bearer realm=\"admin\""),
            )],
        )
            .into_response()
    }
}

pub struct AppBundle {
    pub app: Router,
    pub admin_app: Option<Router>,
    /// Handle to the shared `ArcSwap<EnrichmentContext>` for SIGHUP-based hot-reload.
    pub enrichment_handle: Arc<ArcSwap<EnrichmentContext>>,
}

pub async fn build_app(config: &Config) -> AppBundle {
    let state = AppState::new(config).await;
    let enrichment_handle = Arc::clone(&state.enrichment);

    // Try to install metrics recorder. May fail in tests where multiple
    // build_app calls run in the same process — that's fine, skip metrics.
    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .ok();

    if metrics_handle.is_some() {
        metrics_process::Collector::default().describe();
    }

    // Spawn periodic rate limiter cleanup to prevent unbounded DashMap growth
    {
        let limiter = Arc::clone(&state.rate_limiter);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            interval.tick().await; // skip the immediate first tick
            loop {
                interval.tick().await;
                let before = limiter.len();
                limiter.retain_recent();
                limiter.shrink_to_fit();
                let after = limiter.len();
                if before != after {
                    tracing::debug!(
                        "Rate limiter cleanup: {} -> {} entries",
                        before, after
                    );
                }
            }
        });
    }

    let api_routes = routes::router();

    let cors = if config.server.cors_allowed_origins.iter().any(|o| o == "*") {
        CorsLayer::new().allow_origin(Any)
    } else {
        let origins: Vec<axum::http::HeaderValue> = config
            .server
            .cors_allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new().allow_origin(AllowOrigin::list(origins))
    };

    let app = Router::new()
        .merge(api_routes)
        .fallback(routes::static_handler)
        .layer(DefaultBodyLimit::max(1_048_576))
        .layer(axum_mw::from_fn(middleware::security_headers))
        .layer(cors)
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::geoip_date_headers))
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            extractors::requester_info_middleware,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<axum::body::Body>| {
                let request_id = req
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-");
                tracing::info_span!(
                    "http_request",
                    method = %req.method(),
                    uri = %req.uri(),
                    request_id = %request_id,
                )
            }),
        )
        .layer(axum_mw::from_fn(middleware::record_metrics))
        .layer(axum_mw::from_fn(middleware::request_id))
        .layer(CompressionLayer::new())
        .with_state(state);

    let admin_app = config.server.admin_bind.as_ref().and_then(|_| {
        let handle = metrics_handle?;
        let mut router = Router::new()
            .route(
                "/metrics",
                get(move || {
                    let h = handle.clone();
                    async move {
                        metrics_process::Collector::default().collect();
                        h.render().into_response()
                    }
                }),
            )
            .route("/health", get(|| async { axum::http::StatusCode::OK.into_response() }));
        if let Some(token) = config.server.admin_token.clone() {
            router = router.layer(axum_mw::from_fn_with_state(token, admin_bearer_auth));
        }
        Some(router)
    });

    AppBundle {
        app,
        admin_app,
        enrichment_handle,
    }
}
