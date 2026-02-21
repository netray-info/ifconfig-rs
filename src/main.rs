use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use ifconfig_rs::{build_app, Config};

#[tokio::main]
async fn main() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_format = std::env::var("IFCONFIG_LOG_FORMAT").unwrap_or_default();
    if log_format.eq_ignore_ascii_case("json") {
        tracing_subscriber::fmt().json().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let print_config = args.iter().any(|a| a == "--print-config");
    let config_path = args.iter().find(|a| !a.starts_with("--")).cloned();
    let config = Config::load(config_path.as_deref()).expect("Failed to load config");

    if print_config {
        println!(
            "{}",
            toml::to_string_pretty(&config).expect("Failed to serialize config")
        );
        return;
    }

    let bind_addr: SocketAddr = config.server.bind.parse().expect("Invalid bind address");
    info!("Starting server on {}", bind_addr);

    let bundle = build_app(&config).await;

    // Spawn admin server if configured
    if let Some(admin_app) = bundle.admin_app {
        let admin_bind: SocketAddr = config
            .server
            .admin_bind
            .as_ref()
            .expect("admin_bind must be set")
            .parse()
            .expect("Invalid admin bind address");
        let admin_listener = TcpListener::bind(admin_bind).await.expect("Failed to bind admin port");
        info!("Admin server listening on {}", admin_listener.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(admin_listener, admin_app)
                .with_graceful_shutdown(shutdown_signal())
                .await
                .expect("Admin server error");
        });
    }

    let app = bundle.app.into_make_service_with_connect_info::<SocketAddr>();
    let listener = TcpListener::bind(bind_addr).await.expect("Failed to bind");
    info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    info!("Shutting down gracefully...");
}
