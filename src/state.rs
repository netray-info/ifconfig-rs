use crate::backend::user_agent::UserAgentParser;
use crate::backend::{GeoIpAsnDb, GeoIpCityDb, TorExitNodes};
use crate::config::Config;
use serde::Serialize;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub project_info: Arc<ProjectInfo>,
    pub user_agent_parser: Option<Arc<UserAgentParser>>,
    pub geoip_city_db: Option<Arc<GeoIpCityDb>>,
    pub geoip_asn_db: Option<Arc<GeoIpAsnDb>>,
    pub tor_exit_nodes: Arc<TorExitNodes>,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub base_url: String,
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
        };

        AppState {
            config: Arc::new(Config {
                server: crate::config::ServerConfig {
                    bind: config.server.bind.clone(),
                    trusted_proxies: config.server.trusted_proxies.clone(),
                },
                base_url: config.base_url.clone(),
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
        }
    }
}
