use arc_swap::ArcSwap;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use ifconfig_rs::{build_app, Config};

#[tokio::main]
async fn main() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,mhost=warn"));
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

    let config = Arc::new(config);
    let bundle = build_app(&config).await;

    // Spawn SIGHUP handler for hot-reloading enrichment data
    #[cfg(unix)]
    {
        let enrichment_handle = Arc::clone(&bundle.enrichment_handle);
        let reload_config = Arc::clone(&config);
        tokio::spawn(async move {
            let mut sig =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).expect("Failed to register SIGHUP handler");
            loop {
                sig.recv().await;
                reload_enrichment(&enrichment_handle, &reload_config, "SIGHUP").await;
            }
        });
    }

    // Spawn filesystem watcher for auto-reload (opt-in)
    if config.watch_data_files {
        spawn_file_watcher(Arc::clone(&config), Arc::clone(&bundle.enrichment_handle));
    }

    // Spawn admin server if configured
    if let Some(admin_app) = bundle.admin_app {
        let admin_bind: SocketAddr = config
            .server
            .admin_bind
            .as_ref()
            .expect("admin_bind must be set")
            .parse()
            .expect("Invalid admin bind address");
        if !admin_bind.ip().is_loopback() && config.server.admin_token.is_none() {
            warn!(
                "Admin server binding to non-loopback address {}. \
                 The /metrics endpoint has no authentication — ensure network-level access control \
                 or set server.admin_token.",
                admin_bind
            );
        }
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

async fn reload_enrichment(
    handle: &Arc<ArcSwap<ifconfig_rs::enrichment::EnrichmentContext>>,
    config: &Config,
    trigger: &str,
) {
    info!("{} triggered, reloading enrichment data...", trigger);
    match ifconfig_rs::enrichment::EnrichmentContext::load(config).await {
        Ok(new_ctx) => {
            handle.store(Arc::new(new_ctx));
            info!("Enrichment data reloaded successfully");
        }
        Err(e) => {
            warn!("Failed to reload enrichment data: {}; keeping previous context", e);
        }
    }
}

fn spawn_file_watcher(
    config: Arc<Config>,
    enrichment_handle: Arc<ArcSwap<ifconfig_rs::enrichment::EnrichmentContext>>,
) {
    use notify::{RecommendedWatcher, RecursiveMode, Watcher};

    // Collect unique parent directories of all configured data file paths
    let data_paths: Vec<&Option<String>> = vec![
        &config.geoip_city_db,
        &config.geoip_asn_db,
        &config.user_agent_regexes,
        &config.tor_exit_nodes,
        &config.cloud_provider_ranges,
        &config.feodo_botnet_ips,
        &config.vpn_ranges,
        &config.datacenter_ranges,
        &config.bot_ranges,
        &config.spamhaus_drop,
    ];
    let watch_dirs: HashSet<PathBuf> = data_paths
        .iter()
        .filter_map(|opt| opt.as_deref())
        .filter_map(|p| {
            let path = PathBuf::from(p);
            path.parent().map(|parent| parent.to_path_buf())
        })
        .filter(|dir| dir.exists())
        .collect();

    if watch_dirs.is_empty() {
        warn!("watch_data_files enabled but no data file directories found to watch");
        return;
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(16);

    // Create watcher in a dedicated thread (notify uses sync callbacks)
    std::thread::spawn(move || {
        let tx_clone = tx.clone();
        let mut watcher: RecommendedWatcher =
            match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    use notify::EventKind;
                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                            let _ = tx_clone.try_send(());
                        }
                        _ => {}
                    }
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("Failed to create filesystem watcher: {}", e);
                    return;
                }
            };

        for dir in &watch_dirs {
            match watcher.watch(dir, RecursiveMode::NonRecursive) {
                Ok(()) => info!("Watching directory for changes: {}", dir.display()),
                Err(e) => warn!("Failed to watch {}: {}", dir.display(), e),
            }
        }

        // Keep the watcher alive
        std::thread::park();
    });

    // Debounce + reload loop
    tokio::spawn(async move {
        loop {
            // Wait for first event
            if rx.recv().await.is_none() {
                break;
            }
            // Debounce: drain events for 500ms
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            while rx.try_recv().is_ok() {}

            reload_enrichment(&enrichment_handle, &config, "File change").await;
        }
    });
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    info!("Shutting down gracefully...");
}
