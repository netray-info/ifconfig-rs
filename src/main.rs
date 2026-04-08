use arc_swap::ArcSwap;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

use ifconfig_rs::{Config, build_app, enrichment::EnrichmentContext};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let print_config = args.iter().any(|a| a == "--print-config");
    let check_mode = args.iter().any(|a| a == "--check");
    let config_path = args.iter().find(|a| !a.starts_with("--")).cloned();
    let config = Config::load(config_path.as_deref()).expect("Failed to load config");

    netray_common::telemetry::init_subscriber(
        &config.telemetry,
        "info,ifconfig_rs=debug,hyper=warn,h2=warn,mhost=warn",
    );

    if print_config {
        println!(
            "{}",
            toml::to_string_pretty(&config).expect("Failed to serialize config")
        );
        netray_common::telemetry::shutdown();
        return;
    }

    if check_mode {
        let exit_code = run_check(&config).await;
        netray_common::telemetry::shutdown();
        std::process::exit(exit_code);
    }

    let bind_addr: SocketAddr = config.server.bind.parse().expect("Invalid bind address");
    info!("Starting server on {}", bind_addr);
    info!(
        enabled = config.batch.enabled,
        max_size = config.batch.max_size,
        "Batch endpoint"
    );

    let config = Arc::new(config);
    let bundle = build_app(&config).await;

    // Spawn SIGHUP handler for hot-reloading enrichment data
    #[cfg(unix)]
    {
        let enrichment_handle = Arc::clone(&bundle.enrichment_handle);
        let reload_config = Arc::clone(&config);
        tokio::spawn(async move {
            let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                .expect("Failed to register SIGHUP handler");
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
                .with_graceful_shutdown(netray_common::server::shutdown_signal())
                .await
                .expect("Admin server error");
        });
    }

    let app = bundle.app.into_make_service_with_connect_info::<SocketAddr>();
    let listener = TcpListener::bind(bind_addr).await.expect("Failed to bind");
    info!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(netray_common::server::shutdown_signal())
        .await
        .expect("Server error");

    netray_common::telemetry::shutdown();
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

/// Validate all configured data files and print a summary.
/// Returns 0 if all mandatory sources loaded successfully, 1 otherwise.
async fn run_check(config: &Config) -> i32 {
    println!("Checking configuration and data files...\n");

    let mut ok = true;

    // Check mandatory file fields
    let mandatory = [
        ("geoip_city_db", config.geoip_city_db.as_deref()),
        ("geoip_asn_db", config.geoip_asn_db.as_deref()),
        ("user_agent_regexes", config.user_agent_regexes.as_deref()),
    ];
    for (name, path) in &mandatory {
        match path {
            None => {
                println!("[MISSING] {name}: not configured (required)");
                ok = false;
            }
            Some(p) => {
                if tokio::fs::metadata(p).await.is_ok() {
                    println!("[OK]      {name}: {p}");
                } else {
                    println!("[ERROR]   {name}: {p} — file not found");
                    ok = false;
                }
            }
        }
    }

    // Check optional file fields
    let optional = [
        ("tor_exit_nodes", config.tor_exit_nodes.as_deref()),
        ("cloud_provider_ranges", config.cloud_provider_ranges.as_deref()),
        ("feodo_botnet_ips", config.feodo_botnet_ips.as_deref()),
        ("cins_army_ips", config.cins_army_ips.as_deref()),
        ("vpn_ranges", config.vpn_ranges.as_deref()),
        ("datacenter_ranges", config.datacenter_ranges.as_deref()),
        ("bot_ranges", config.bot_ranges.as_deref()),
        ("spamhaus_drop", config.spamhaus_drop.as_deref()),
        ("asn_patterns", config.asn_patterns.as_deref()),
        ("asn_info", config.asn_info.as_deref()),
    ];
    for (name, path) in &optional {
        match path {
            None => println!("[SKIP]    {name}: not configured"),
            Some(p) => {
                if tokio::fs::metadata(p).await.is_ok() {
                    println!("[OK]      {name}: {p}");
                } else {
                    println!("[WARN]    {name}: {p} — file not found");
                }
            }
        }
    }

    // Attempt a full enrichment context load to catch parse errors
    println!("\nAttempting enrichment context load...");
    match EnrichmentContext::load(config).await {
        Ok(ctx) => {
            println!("[OK]      Enrichment context loaded successfully");
            if !ctx.missing_optional.is_empty() {
                println!("          Warnings: {}", ctx.missing_optional.join(", "));
            }
        }
        Err(e) => {
            println!("[ERROR]   Enrichment context failed to load: {e}");
            ok = false;
        }
    }

    println!();
    if ok {
        println!("Check passed.");
        0
    } else {
        println!("Check FAILED — see errors above.");
        1
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
        &config.cins_army_ips,
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
