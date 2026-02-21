use crate::backend::user_agent::UserAgentParser;
use crate::backend::{GeoIpAsnDb, GeoIpCityDb, TorExitNodes};
use crate::config::Config;
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use hickory_resolver::TokioResolver;
use serde::Serialize;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::{info, warn};

pub type KeyedRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub project_info: Arc<ProjectInfo>,
    pub user_agent_parser: Option<Arc<UserAgentParser>>,
    pub geoip_city_db: Option<Arc<GeoIpCityDb>>,
    pub geoip_asn_db: Option<Arc<GeoIpAsnDb>>,
    pub tor_exit_nodes: Arc<TorExitNodes>,
    pub dns_resolver: Arc<TokioResolver>,
    pub rate_limiter: Arc<KeyedRateLimiter>,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub site_name: String,
}

impl AppState {
    pub fn new(config: &Config) -> Self {
        let user_agent_parser = config
            .user_agent_regexes
            .as_deref()
            .and_then(|path| match UserAgentParser::from_yaml(path) {
                Ok(parser) => {
                    info!("Loaded User-Agent regexes from {}", path);
                    Some(Arc::new(parser))
                }
                Err(e) => {
                    warn!("Failed to load User-Agent regexes from {}: {}", path, e);
                    None
                }
            });

        let geoip_city_db = config
            .geoip_city_db
            .as_deref()
            .and_then(|path| match GeoIpCityDb::new(path) {
                Some(db) => {
                    info!("Loaded GeoIP City database from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load GeoIP City database from {}", path);
                    None
                }
            });

        let geoip_asn_db = config
            .geoip_asn_db
            .as_deref()
            .and_then(|path| match GeoIpAsnDb::new(path) {
                Some(db) => {
                    info!("Loaded GeoIP ASN database from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load GeoIP ASN database from {}", path);
                    None
                }
            });

        let tor_exit_nodes = config
            .tor_exit_nodes
            .as_deref()
            .map(|path| {
                let nodes = TorExitNodes::from_file(path);
                info!("Loaded Tor exit nodes from {}", path);
                nodes
            })
            .unwrap_or_else(TorExitNodes::empty);

        let project_info = ProjectInfo {
            name: config.project_name.clone(),
            version: config.project_version.clone(),
            base_url: config.base_url.clone(),
            site_name: config.site_name.clone().unwrap_or_else(|| config.base_url.clone()),
        };

        let dns_resolver = TokioResolver::builder_tokio()
            .expect("Failed to read system DNS config")
            .build();
        info!("DNS resolver initialized from system config");

        let per_minute =
            NonZeroU32::new(config.rate_limit.per_ip_per_minute as u32).expect("per_ip_per_minute must be > 0");
        let burst = NonZeroU32::new(config.rate_limit.per_ip_burst).expect("per_ip_burst must be > 0");
        let quota = Quota::per_minute(per_minute).allow_burst(burst);
        let rate_limiter = Arc::new(RateLimiter::keyed(quota));
        info!(
            "Rate limiter configured: {} req/min, burst {}",
            config.rate_limit.per_ip_per_minute, config.rate_limit.per_ip_burst
        );

        AppState {
            config: Arc::new(Config {
                server: crate::config::ServerConfig {
                    bind: config.server.bind.clone(),
                    trusted_proxies: config.server.trusted_proxies.clone(),
                },
                base_url: config.base_url.clone(),
                site_name: config.site_name.clone(),
                project_name: config.project_name.clone(),
                project_version: config.project_version.clone(),
                geoip_city_db: config.geoip_city_db.clone(),
                geoip_asn_db: config.geoip_asn_db.clone(),
                user_agent_regexes: config.user_agent_regexes.clone(),
                tor_exit_nodes: config.tor_exit_nodes.clone(),
                rate_limit: crate::config::RateLimitConfig {
                    per_ip_per_minute: config.rate_limit.per_ip_per_minute,
                    per_ip_burst: config.rate_limit.per_ip_burst,
                },
            }),
            project_info: Arc::new(project_info),
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes: Arc::new(tor_exit_nodes),
            dns_resolver: Arc::new(dns_resolver),
            rate_limiter,
        }
    }
}
