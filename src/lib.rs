pub mod backend;
pub mod config;
pub mod error;
pub mod extractors;
pub mod format;
pub mod handlers;
pub mod middleware;
pub mod negotiate;
pub mod routes;
pub mod state;

use axum::middleware as axum_mw;
use axum::Router;
use state::AppState;

pub use config::Config;
pub use state::ProjectInfo;

pub fn build_app(config: &Config) -> Router {
    let state = AppState::new(config);

    let api_routes = routes::router(state.clone());

    Router::new()
        .merge(api_routes)
        .fallback(routes::static_handler)
        .layer(axum_mw::from_fn(middleware::security_headers))
        .layer(axum_mw::from_fn_with_state(state.clone(), middleware::rate_limit))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            extractors::requester_info_middleware,
        ))
        .with_state(state)
}

/// Build an app suitable for testing (without ConnectInfo requirement).
/// Tests should add their own ConnectInfo layer or use the real TCP listener.
pub fn build_app_for_test(config: &Config) -> Router {
    build_app(config)
}
