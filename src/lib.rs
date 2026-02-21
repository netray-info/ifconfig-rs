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
use axum::middleware as axum_mw;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use enrichment::EnrichmentContext;
use state::AppState;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use state::ProjectInfo;

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

    let api_routes = routes::router(state.clone());

    let app = Router::new()
        .merge(api_routes)
        .fallback(routes::static_handler)
        .layer(axum_mw::from_fn(middleware::security_headers))
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::geoip_date_headers))
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            extractors::requester_info_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let admin_app = config.server.admin_bind.as_ref().and_then(|_| {
        let handle = metrics_handle?;
        Some(
            Router::new()
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
                .route("/health", get(|| async { axum::http::StatusCode::OK.into_response() })),
        )
    });

    AppBundle {
        app,
        admin_app,
        enrichment_handle,
    }
}
