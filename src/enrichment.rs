use crate::backend::user_agent::UserAgentParser;
use crate::backend::{BotDb, CloudProviderDb, DatacenterRanges, FeodoBotnetIps, GeoIpAsnDb, GeoIpCityDb, SpamhausDrop, TorExitNodes, VpnRanges};
use crate::config::Config;
use mhost::resolver::{ResolverGroup, ResolverGroupBuilder};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, thiserror::Error)]
#[error("Failed to create DNS resolver: {0}")]
pub struct LoadError(String);

/// Groups all reloadable backend resources. Stored behind `ArcSwap` in `AppState`
/// so SIGHUP can atomically swap in a freshly loaded context without dropping
/// in-flight requests.
pub struct EnrichmentContext {
    pub user_agent_parser: Option<Arc<UserAgentParser>>,
    pub geoip_city_db: Option<Arc<GeoIpCityDb>>,
    pub geoip_asn_db: Option<Arc<GeoIpAsnDb>>,
    pub tor_exit_nodes: Arc<TorExitNodes>,
    pub feodo_botnet_ips: Option<Arc<FeodoBotnetIps>>,
    pub cloud_provider_db: Option<Arc<CloudProviderDb>>,
    pub vpn_ranges: Option<Arc<VpnRanges>>,
    pub datacenter_ranges: Option<Arc<DatacenterRanges>>,
    pub bot_db: Option<Arc<BotDb>>,
    pub spamhaus_drop: Option<Arc<SpamhausDrop>>,
    pub dns_resolver: Arc<ResolverGroup>,
    pub geoip_city_build_epoch: Option<u64>,
    /// Optional sources that were configured (path provided) but failed to load.
    /// Surfaced in `/ready` warnings and emitted as a single startup log line.
    pub missing_optional: Vec<&'static str>,
}

impl EnrichmentContext {
    /// Load all backends from the paths specified in `config`.
    /// DNS resolver is built from system config (async).
    pub async fn load(config: &Config) -> Result<Self, LoadError> {
        let geoip_city_db = if let Some(path) = config.geoip_city_db.as_deref() {
            match GeoIpCityDb::new(path).await {
                Some(db) => {
                    info!("Loaded GeoIP City database from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    error!("Failed to load GeoIP City database from {}", path);
                    return Err(LoadError(format!("Failed to load GeoIP City database from {path}")));
                }
            }
        } else {
            warn!("geoip_city_db not configured; geolocation lookups disabled");
            None
        };

        let geoip_asn_db = if let Some(path) = config.geoip_asn_db.as_deref() {
            match GeoIpAsnDb::new(path).await {
                Some(db) => {
                    info!("Loaded GeoIP ASN database from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    error!("Failed to load GeoIP ASN database from {}", path);
                    return Err(LoadError(format!("Failed to load GeoIP ASN database from {path}")));
                }
            }
        } else {
            warn!("geoip_asn_db not configured; ISP/ASN lookups disabled");
            None
        };

        let user_agent_parser = if let Some(path) = config.user_agent_regexes.as_deref() {
            match UserAgentParser::from_yaml(path).await {
                Ok(parser) => {
                    info!("Loaded User-Agent regexes from {}", path);
                    Some(Arc::new(parser))
                }
                Err(e) => {
                    warn!("Failed to load User-Agent regexes from {}: {}", path, e);
                    None
                }
            }
        } else {
            warn!("user_agent_regexes not configured; User-Agent parsing disabled");
            None
        };

        let tor_exit_nodes = if let Some(path) = config.tor_exit_nodes.as_deref() {
            let nodes = TorExitNodes::from_file(path).await;
            if nodes.is_loaded() {
                info!("Loaded Tor exit nodes from {}", path);
            } else {
                warn!("Failed to load Tor exit nodes from {}", path);
            }
            nodes
        } else {
            warn!("tor_exit_nodes not configured; Tor exit node detection disabled");
            TorExitNodes::empty()
        };

        let feodo_botnet_ips = if let Some(path) = config.feodo_botnet_ips.as_deref() {
            let ips = FeodoBotnetIps::from_file(path).await;
            if ips.is_loaded() {
                info!("Loaded Feodo botnet IPs from {}", path);
                Some(Arc::new(ips))
            } else {
                warn!("Failed to load Feodo botnet IPs from {}", path);
                None
            }
        } else {
            warn!("feodo_botnet_ips not configured; botnet C2 detection disabled");
            None
        };

        let cloud_provider_db = if let Some(path) = config.cloud_provider_ranges.as_deref() {
            match CloudProviderDb::from_file(path).await {
                Some(db) => {
                    info!("Loaded cloud provider ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load cloud provider ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("cloud_provider_ranges not configured; cloud provider detection disabled");
            None
        };

        let vpn_ranges = if let Some(path) = config.vpn_ranges.as_deref() {
            match VpnRanges::from_file(path).await {
                Some(db) => {
                    info!("Loaded VPN ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load VPN ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("vpn_ranges not configured; CIDR-based VPN detection disabled");
            None
        };

        let datacenter_ranges = if let Some(path) = config.datacenter_ranges.as_deref() {
            match DatacenterRanges::from_file(path).await {
                Some(db) => {
                    info!("Loaded datacenter ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load datacenter ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("datacenter_ranges not configured; CIDR-based datacenter detection disabled");
            None
        };

        let bot_db = if let Some(path) = config.bot_ranges.as_deref() {
            match BotDb::from_file(path).await {
                Some(db) => {
                    info!("Loaded bot ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load bot ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("bot_ranges not configured; bot IP detection disabled");
            None
        };

        let spamhaus_drop = if let Some(path) = config.spamhaus_drop.as_deref() {
            match SpamhausDrop::from_file(path).await {
                Some(db) => {
                    info!("Loaded Spamhaus DROP from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load Spamhaus DROP from {}", path);
                    None
                }
            }
        } else {
            warn!("spamhaus_drop not configured; threat/hijacked-netblock detection disabled");
            None
        };

        let dns_resolver = ResolverGroupBuilder::new()
            .system()
            .build()
            .await
            .map_err(|e| LoadError(e.to_string()))?;
        info!("DNS resolver initialized from system config");

        let geoip_city_build_epoch = geoip_city_db.as_ref().map(|db| db.build_epoch());

        // Collect optional sources that were configured but failed to load.
        let missing_optional: Vec<&'static str> = [
            (config.user_agent_regexes.is_some()    && user_agent_parser.is_none(),      "user_agent_regexes"),
            (config.tor_exit_nodes.is_some()        && !tor_exit_nodes.is_loaded(),      "tor_exit_nodes"),
            (config.feodo_botnet_ips.is_some()      && feodo_botnet_ips.is_none(),       "feodo_botnet_ips"),
            (config.cloud_provider_ranges.is_some() && cloud_provider_db.is_none(),      "cloud_provider_ranges"),
            (config.vpn_ranges.is_some()            && vpn_ranges.is_none(),             "vpn_ranges"),
            (config.datacenter_ranges.is_some()     && datacenter_ranges.is_none(),      "datacenter_ranges"),
            (config.bot_ranges.is_some()            && bot_db.is_none(),                 "bot_ranges"),
            (config.spamhaus_drop.is_some()         && spamhaus_drop.is_none(),          "spamhaus_drop"),
        ]
        .into_iter()
        .filter_map(|(failed, name)| if failed { Some(name) } else { None })
        .collect();

        if !missing_optional.is_empty() {
            warn!("Optional data sources not loaded: {}", missing_optional.join(", "));
        }

        let ctx = EnrichmentContext {
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes: Arc::new(tor_exit_nodes),
            feodo_botnet_ips,
            cloud_provider_db,
            vpn_ranges,
            datacenter_ranges,
            bot_db,
            spamhaus_drop,
            dns_resolver: Arc::new(dns_resolver),
            geoip_city_build_epoch,
            missing_optional,
        };

        // Report which enrichment sources are loaded (updates on reload too).
        metrics::gauge!("enrichment_sources_loaded", "source" => "geoip_city")
            .set(f64::from(ctx.geoip_city_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "geoip_asn")
            .set(f64::from(ctx.geoip_asn_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "user_agent")
            .set(f64::from(ctx.user_agent_parser.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "tor_exit_nodes")
            .set(f64::from(ctx.tor_exit_nodes.is_loaded() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "cloud_provider")
            .set(f64::from(ctx.cloud_provider_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "vpn_ranges")
            .set(f64::from(ctx.vpn_ranges.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "datacenter_ranges")
            .set(f64::from(ctx.datacenter_ranges.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "bot_ranges")
            .set(f64::from(ctx.bot_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "feodo_botnet")
            .set(f64::from(ctx.feodo_botnet_ips.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "spamhaus_drop")
            .set(f64::from(ctx.spamhaus_drop.is_some() as u8));

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn load_with_no_paths_succeeds() {
        let config = Config::load(None).unwrap();
        let ctx = EnrichmentContext::load(&config).await;
        assert!(ctx.is_ok());
        let ctx = ctx.unwrap();
        assert!(ctx.geoip_city_db.is_none());
        assert!(ctx.geoip_asn_db.is_none());
        assert!(ctx.user_agent_parser.is_none());
        assert!(ctx.cloud_provider_db.is_none());
        assert!(ctx.vpn_ranges.is_none());
        assert!(ctx.geoip_city_build_epoch.is_none());
    }

    #[tokio::test]
    async fn nonexistent_geoip_city_path_returns_error() {
        let mut config = Config::load(None).unwrap();
        config.geoip_city_db = Some("/nonexistent/GeoLite2-City.mmdb".to_string());
        let result = EnrichmentContext::load(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nonexistent_geoip_asn_path_returns_error() {
        let mut config = Config::load(None).unwrap();
        config.geoip_asn_db = Some("/nonexistent/GeoLite2-ASN.mmdb".to_string());
        let result = EnrichmentContext::load(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nonexistent_ua_regex_path_yields_none() {
        let mut config = Config::load(None).unwrap();
        config.user_agent_regexes = Some("/nonexistent/regexes.yaml".to_string());
        let ctx = EnrichmentContext::load(&config).await.unwrap();
        assert!(ctx.user_agent_parser.is_none());
    }

    #[tokio::test]
    async fn no_geoip_means_build_epoch_is_none() {
        let config = Config::load(None).unwrap();
        let ctx = EnrichmentContext::load(&config).await.unwrap();
        assert!(ctx.geoip_city_build_epoch.is_none());
    }
}
