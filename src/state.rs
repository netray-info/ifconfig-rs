use crate::config::Config;
use crate::enrichment::EnrichmentContext;
use arc_swap::ArcSwap;
use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use regex::Regex;
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
    pub header_filters: Arc<Vec<Regex>>,
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
        let enrichment = EnrichmentContext::load(config).await;

        let project_info = ProjectInfo {
            name: config.project_name.clone(),
            version: config.project_version.clone(),
            base_url: config.base_url.clone(),
            site_name: config.site_name.clone().unwrap_or_else(|| config.base_url.clone()),
        };

        let per_minute =
            NonZeroU32::new(config.rate_limit.per_ip_per_minute as u32).expect("per_ip_per_minute must be > 0");
        let burst = NonZeroU32::new(config.rate_limit.per_ip_burst).expect("per_ip_burst must be > 0");
        let quota = Quota::per_minute(per_minute).allow_burst(burst);
        let rate_limiter = Arc::new(RateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>());
        info!(
            "Rate limiter configured: {} req/min, burst {}",
            config.rate_limit.per_ip_per_minute, config.rate_limit.per_ip_burst
        );

        let header_filters: Vec<Regex> = config
            .filtered_headers
            .iter()
            .filter_map(|pattern| match Regex::new(pattern) {
                Ok(re) => Some(re),
                Err(e) => {
                    warn!("Invalid header filter regex '{}': {}", pattern, e);
                    None
                }
            })
            .collect();
        if !header_filters.is_empty() {
            info!("Header filters loaded: {} patterns", header_filters.len());
        }

        AppState {
            config: Arc::new(Config {
                server: crate::config::ServerConfig {
                    bind: config.server.bind.clone(),
                    admin_bind: config.server.admin_bind.clone(),
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
                cloud_provider_ranges: config.cloud_provider_ranges.clone(),
                feodo_botnet_ips: config.feodo_botnet_ips.clone(),
                vpn_ranges: config.vpn_ranges.clone(),
                datacenter_ranges: config.datacenter_ranges.clone(),
                bot_ranges: config.bot_ranges.clone(),
                spamhaus_drop: config.spamhaus_drop.clone(),
                filtered_headers: config.filtered_headers.clone(),
                rate_limit: crate::config::RateLimitConfig {
                    per_ip_per_minute: config.rate_limit.per_ip_per_minute,
                    per_ip_burst: config.rate_limit.per_ip_burst,
                },
                batch: crate::config::BatchConfig {
                    enabled: config.batch.enabled,
                    max_size: config.batch.max_size,
                },
            }),
            project_info: Arc::new(project_info),
            enrichment: Arc::new(ArcSwap::from_pointee(enrichment)),
            rate_limiter,
            header_filters: Arc::new(header_filters),
        }
    }
}
