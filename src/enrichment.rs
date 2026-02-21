use crate::backend::user_agent::UserAgentParser;
use crate::backend::{BotDb, CloudProviderDb, DatacenterRanges, FeodoBotnetIps, GeoIpAsnDb, GeoIpCityDb, SpamhausDrop, TorExitNodes, VpnRanges};
use crate::config::Config;
use mhost::resolver::{ResolverGroup, ResolverGroupBuilder};
use std::sync::Arc;
use tracing::{info, warn};

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
}

impl EnrichmentContext {
    /// Load all backends from the paths specified in `config`.
    /// DNS resolver is built from system config (async).
    pub async fn load(config: &Config) -> Self {
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

        let feodo_botnet_ips = config.feodo_botnet_ips.as_deref().map(|path| {
            let ips = FeodoBotnetIps::from_file(path);
            info!("Loaded Feodo botnet IPs from {}", path);
            Arc::new(ips)
        });

        let cloud_provider_db = config.cloud_provider_ranges.as_deref().and_then(|path| {
            match CloudProviderDb::from_file(path) {
                Some(db) => {
                    info!("Loaded cloud provider ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load cloud provider ranges from {}", path);
                    None
                }
            }
        });

        let vpn_ranges = config.vpn_ranges.as_deref().and_then(|path| {
            match VpnRanges::from_file(path) {
                Some(db) => {
                    info!("Loaded VPN ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load VPN ranges from {}", path);
                    None
                }
            }
        });

        let datacenter_ranges = config.datacenter_ranges.as_deref().and_then(|path| {
            match DatacenterRanges::from_file(path) {
                Some(db) => {
                    info!("Loaded datacenter ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load datacenter ranges from {}", path);
                    None
                }
            }
        });

        let bot_db = config.bot_ranges.as_deref().and_then(|path| {
            match BotDb::from_file(path) {
                Some(db) => {
                    info!("Loaded bot ranges from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load bot ranges from {}", path);
                    None
                }
            }
        });

        let spamhaus_drop = config.spamhaus_drop.as_deref().and_then(|path| {
            match SpamhausDrop::from_file(path) {
                Some(db) => {
                    info!("Loaded Spamhaus DROP from {}", path);
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load Spamhaus DROP from {}", path);
                    None
                }
            }
        });

        let dns_resolver = ResolverGroupBuilder::new()
            .system()
            .build()
            .await
            .expect("Failed to create DNS resolver from system config");
        info!("DNS resolver initialized from system config");

        let geoip_city_build_epoch = geoip_city_db.as_ref().map(|db| db.build_epoch());

        EnrichmentContext {
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
        }
    }
}
