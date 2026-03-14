use crate::backend::{new_dns_cache, DnsCache, Ifconfig};
use crate::config::Config;
use crate::enrichment::EnrichmentContext;
use arc_swap::ArcSwap;
use governor::clock::DefaultClock;
use governor::middleware::StateInformationMiddleware;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use ip_network::IpNetwork;
use moka::future::Cache;
use regex::RegexSet;
use serde::Serialize;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

pub type IpCache = Cache<IpAddr, Arc<Ifconfig>>;

pub type KeyedRateLimiter =
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock, StateInformationMiddleware>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub project_info: Arc<ProjectInfo>,
    pub enrichment: Arc<ArcSwap<EnrichmentContext>>,
    pub rate_limiter: Arc<KeyedRateLimiter>,
    pub target_rate_limiter: Arc<KeyedRateLimiter>,
    pub header_filters: Arc<RegexSet>,
    pub trusted_proxies: Arc<Vec<IpNetwork>>,
    pub dns_cache: Arc<DnsCache>,
    pub ip_cache: IpCache,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RateLimitInfo {
    pub per_ip_per_minute: u32,
    pub per_ip_burst: u32,
    pub per_target_per_minute: u32,
    pub per_target_burst: u32,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BatchInfo {
    pub enabled: bool,
    pub max_size: usize,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct BuildInfo {
    pub date: String,
    pub revision: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
    pub base_url: String,
    pub site_name: String,
    pub batch: BatchInfo,
    pub rate_limit: RateLimitInfo,
    pub build: BuildInfo,
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
            batch: BatchInfo {
                enabled: config.batch.enabled,
                max_size: config.batch.max_size,
            },
            rate_limit: RateLimitInfo {
                per_ip_per_minute: config.rate_limit.per_ip_per_minute,
                per_ip_burst: config.rate_limit.per_ip_burst,
                per_target_per_minute: config.rate_limit.per_target_per_minute,
                per_target_burst: config.rate_limit.per_target_burst,
            },
            build: BuildInfo {
                date: env!("BUILD_DATE").to_string(),
                revision: env!("GIT_SHORT_SHA").to_string(),
            },
        };

        if config.batch.enabled {
            assert!(
                config.batch.max_size > 0,
                "batch.max_size must be > 0 when batch is enabled"
            );
            if config.batch.max_size > 10_000 {
                warn!(
                    "batch.max_size={} is very large; values above 10000 risk resource exhaustion under load",
                    config.batch.max_size
                );
            }
        }

        let per_minute = NonZeroU32::new(config.rate_limit.per_ip_per_minute).expect("per_ip_per_minute must be > 0");
        let burst = NonZeroU32::new(config.rate_limit.per_ip_burst).expect("per_ip_burst must be > 0");
        let quota = Quota::per_minute(per_minute).allow_burst(burst);
        let rate_limiter = Arc::new(RateLimiter::keyed(quota).with_middleware::<StateInformationMiddleware>());
        info!(
            "Rate limiter configured: {} req/min, burst {}",
            config.rate_limit.per_ip_per_minute, config.rate_limit.per_ip_burst
        );

        let target_per_minute =
            NonZeroU32::new(config.rate_limit.per_target_per_minute).expect("per_target_per_minute must be > 0");
        let target_burst = NonZeroU32::new(config.rate_limit.per_target_burst).expect("per_target_burst must be > 0");
        let target_quota = Quota::per_minute(target_per_minute).allow_burst(target_burst);
        let target_rate_limiter =
            Arc::new(RateLimiter::keyed(target_quota).with_middleware::<StateInformationMiddleware>());
        info!(
            "Target rate limiter configured: {} req/min, burst {}",
            config.rate_limit.per_target_per_minute, config.rate_limit.per_target_burst
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

        let ip_cache = Cache::builder()
            .max_capacity(config.cache.max_entries)
            .time_to_live(Duration::from_secs(config.cache.ttl_secs))
            .build();

        AppState {
            config: Arc::new(config.clone()),
            project_info: Arc::new(project_info),
            enrichment: Arc::new(ArcSwap::from_pointee(enrichment)),
            rate_limiter,
            target_rate_limiter,
            header_filters: Arc::new(header_filters),
            trusted_proxies: Arc::new(trusted_proxies),
            dns_cache: Arc::new(new_dns_cache()),
            ip_cache,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BatchConfig, Config};

    async fn build_state(f: impl FnOnce(&mut Config)) -> AppState {
        let mut config = Config::load(None).unwrap();
        f(&mut config);
        AppState::new(&config).await
    }

    #[tokio::test]
    async fn invalid_trusted_proxy_cidr_is_skipped() {
        let state = build_state(|c| {
            c.server.trusted_proxies = vec!["10.0.0.0/8".to_string(), "not-a-cidr".to_string()];
        })
        .await;
        assert_eq!(state.trusted_proxies.len(), 1);
    }

    #[tokio::test]
    async fn valid_trusted_proxies_are_parsed() {
        let state = build_state(|c| {
            c.server.trusted_proxies = vec!["192.168.0.0/24".to_string(), "10.0.0.1".to_string()];
        })
        .await;
        assert_eq!(state.trusted_proxies.len(), 2);
    }

    #[tokio::test]
    async fn invalid_header_filter_regex_is_skipped() {
        let state = build_state(|c| {
            c.filtered_headers = vec!["^X-Valid-.*".to_string(), "[invalid regex".to_string()];
        })
        .await;
        assert_eq!(state.header_filters.len(), 1);
    }

    #[tokio::test]
    async fn all_header_filter_regexes_valid() {
        let state = build_state(|c| {
            c.filtered_headers = vec!["^Authorization$".to_string(), "^X-Api-Key$".to_string()];
        })
        .await;
        assert_eq!(state.header_filters.len(), 2);
    }

    #[tokio::test]
    #[should_panic(expected = "batch.max_size must be > 0")]
    async fn batch_max_size_zero_panics() {
        build_state(|c| {
            c.batch = BatchConfig {
                enabled: true,
                max_size: 0,
            };
        })
        .await;
    }

    #[tokio::test]
    async fn batch_max_size_zero_ok_when_disabled() {
        // max_size=0 is not validated when batch is disabled
        let state = build_state(|c| {
            c.batch = BatchConfig {
                enabled: false,
                max_size: 0,
            };
        })
        .await;
        assert_eq!(state.config.batch.max_size, 0);
    }
}
