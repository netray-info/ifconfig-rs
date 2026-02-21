use crate::config::Config;
use crate::enrichment::EnrichmentContext;
use arc_swap::ArcSwap;
use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use ip_network::IpNetwork;
use regex::RegexSet;
use serde::Serialize;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::{info, warn};

pub type KeyedRateLimiter =
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, StateInformationMiddleware>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub project_info: Arc<ProjectInfo>,
    pub enrichment: Arc<ArcSwap<EnrichmentContext>>,
    pub rate_limiter: Arc<KeyedRateLimiter>,
    pub header_filters: Arc<RegexSet>,
    pub trusted_proxies: Arc<Vec<IpNetwork>>,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub site_name: String,
}

impl AppState {
    pub async fn new(config: &Config) -> Self {
        let enrichment = EnrichmentContext::load(config)
            .await
            .expect("Failed to load enrichment context at startup");

        let project_info = ProjectInfo {
            name: config.project_name.clone(),
            version: config.project_version.clone(),
            base_url: config.base_url.clone(),
            site_name: config.site_name.clone().unwrap_or_else(|| config.base_url.clone()),
        };

        let per_minute =
            NonZeroU32::new(config.rate_limit.per_ip_per_minute).expect("per_ip_per_minute must be > 0");
        let burst = NonZeroU32::new(config.rate_limit.per_ip_burst).expect("per_ip_burst must be > 0");
        let quota = Quota::per_minute(per_minute).allow_burst(burst);
        let rate_limiter = Arc::new(RateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>());
        info!(
            "Rate limiter configured: {} req/min, burst {}",
            config.rate_limit.per_ip_per_minute, config.rate_limit.per_ip_burst
        );

        let valid_patterns: Vec<&str> = config
            .filtered_headers
            .iter()
            .filter(|pattern| match regex::Regex::new(pattern) {
                Ok(_) => true,
                Err(e) => {
                    warn!("Invalid header filter regex '{}': {}", pattern, e);
                    false
                }
            })
            .map(|s| s.as_str())
            .collect();
        let header_filters = RegexSet::new(&valid_patterns).expect("pre-validated regex patterns");
        if !header_filters.is_empty() {
            info!("Header filters loaded: {} patterns", header_filters.len());
        }

        let trusted_proxies: Vec<IpNetwork> = config
            .server
            .trusted_proxies
            .iter()
            .filter_map(|s| match s.parse::<IpNetwork>() {
                Ok(net) => Some(net),
                Err(_) => {
                    // Fall back to parsing as bare IP (host-only CIDR)
                    match s.parse::<IpAddr>() {
                        Ok(ip) => Some(IpNetwork::from(ip)),
                        Err(e) => {
                            warn!("Invalid trusted proxy '{}': {}", s, e);
                            None
                        }
                    }
                }
            })
            .collect();
        if !trusted_proxies.is_empty() {
            info!("Trusted proxies loaded: {} entries", trusted_proxies.len());
        }

        AppState {
            config: Arc::new(config.clone()),
            project_info: Arc::new(project_info),
            enrichment: Arc::new(ArcSwap::from_pointee(enrichment)),
            rate_limiter,
            header_filters: Arc::new(header_filters),
            trusted_proxies: Arc::new(trusted_proxies),
        }
    }
}
