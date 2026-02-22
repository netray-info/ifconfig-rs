pub mod asn_heuristic;
pub mod asn_info;
pub mod bot;
pub mod cloud_provider;
pub mod datacenter;
pub mod feodo;
pub mod spamhaus;
pub mod user_agent;
pub mod vpn;
pub use asn_info::AsnInfo;
pub use bot::{BotDb, BotInfo};
pub use cloud_provider::{CloudProvider, CloudProviderDb};
pub use datacenter::DatacenterRanges;
pub use feodo::FeodoBotnetIps;
pub use spamhaus::SpamhausDrop;
pub use user_agent::*;
pub use vpn::VpnRanges;

use lru::LruCache;
use maxminddb::{self, geoip2};
use mhost::resolver::{MultiQuery, ResolverGroup};
use mhost::RecordType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

/// In-memory LRU cache for reverse DNS (PTR) lookups.
/// Stores `Option<String>` so a failed lookup is also cached (avoiding repeated timeouts).
pub type DnsCache = std::sync::Mutex<LruCache<IpAddr, (Option<String>, std::time::Instant)>>;
const DNS_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(60);
const DNS_CACHE_CAPACITY: usize = 1024;

pub fn new_dns_cache() -> DnsCache {
    let capacity = std::num::NonZeroUsize::new(DNS_CACHE_CAPACITY).expect("DNS_CACHE_CAPACITY > 0");
    std::sync::Mutex::new(LruCache::new(capacity))
}

/// Returns `true` if `ip` is a publicly routable address.
/// Returns `false` for loopback, unspecified, RFC 1918 private, link-local,
/// IPv6 ULA (fc00::/7), multicast, and IPv4-mapped private addresses.
pub fn is_global_ip(ip: IpAddr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return false;
    }
    match ip {
        IpAddr::V4(v4) => !v4.is_private() && !v4.is_link_local(),
        IpAddr::V6(v6) => {
            let segs = v6.segments();
            // ULA fc00::/7
            if segs[0] & 0xfe00 == 0xfc00 {
                return false;
            }
            // Link-local fe80::/10
            if segs[0] & 0xffc0 == 0xfe80 {
                return false;
            }
            // Multicast ff00::/8
            if segs[0] & 0xff00 == 0xff00 {
                return false;
            }
            // IPv4-mapped ::ffff:x.x.x.x — check the embedded v4 address
            if let Some(v4) = v6.to_ipv4_mapped() {
                return !v4.is_private() && !v4.is_link_local() && !v4.is_loopback();
            }
            true
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Ip {
    #[schema(example = "203.0.113.42")]
    pub addr: String,
    #[schema(example = "4")]
    pub version: String,
    #[schema(example = "dns.example.com")]
    pub hostname: Option<String>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Tcp {
    #[schema(example = 54321)]
    pub port: u16,
}

pub struct GeoIpCityDb(maxminddb::Reader<Vec<u8>>);

impl GeoIpCityDb {
    pub async fn new(db_path: &str) -> Option<Self> {
        let bytes = tokio::fs::read(db_path).await.ok()?;
        maxminddb::Reader::from_source(bytes).ok().map(GeoIpCityDb)
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<geoip2::City<'_>> {
        self.0.lookup(ip).ok().and_then(|r| r.decode().ok().flatten())
    }

    pub fn build_epoch(&self) -> u64 {
        self.0.metadata.build_epoch
    }
}

pub struct GeoIpAsnDb(maxminddb::Reader<Vec<u8>>);

impl GeoIpAsnDb {
    pub async fn new(db_path: &str) -> Option<Self> {
        let bytes = tokio::fs::read(db_path).await.ok()?;
        maxminddb::Reader::from_source(bytes).ok().map(GeoIpAsnDb)
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<(geoip2::Isp<'_>, Option<String>)> {
        let result = self.0.lookup(ip).ok()?;
        let prefix = result.network().ok().map(|n| n.to_string());
        let isp = result.decode().ok().flatten()?;
        Some((isp, prefix))
    }
}

pub struct TorExitNodes(Option<HashSet<IpAddr>>);

impl TorExitNodes {
    pub async fn from_file(path: &str) -> Self {
        let set = tokio::fs::read_to_string(path)
            .await
            .ok()
            .map(|contents| {
                contents
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .filter_map(|line| line.trim().parse().ok())
                    .collect::<HashSet<IpAddr>>()
            })
            .filter(|set| !set.is_empty());
        TorExitNodes(set)
    }

    pub fn empty() -> Self {
        TorExitNodes(None)
    }

    pub fn is_loaded(&self) -> bool {
        self.0.is_some()
    }

    pub fn lookup(&self, addr: &IpAddr) -> Option<bool> {
        self.0.as_ref().map(|set| set.contains(addr))
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Location {
    #[schema(example = "Berlin")]
    pub city: Option<String>,
    #[schema(example = "Berlin")]
    pub region: Option<String>,
    #[schema(example = "BE")]
    pub region_code: Option<String>,
    #[schema(example = "Germany")]
    pub country: Option<String>,
    #[schema(example = "DE")]
    pub country_iso: Option<String>,
    #[schema(example = "10115")]
    pub postal_code: Option<String>,
    #[schema(example = true)]
    pub is_eu: Option<bool>,
    #[schema(example = 52.5200)]
    pub latitude: Option<f64>,
    #[schema(example = 13.4050)]
    pub longitude: Option<f64>,
    #[schema(example = "Europe/Berlin")]
    pub timezone: Option<String>,
    #[schema(example = "Europe")]
    pub continent: Option<String>,
    #[schema(example = "EU")]
    pub continent_code: Option<String>,
    #[schema(example = 100)]
    pub accuracy_radius_km: Option<u16>,
    /// Country where the IP block is registered (differs from `country` for VPN exit nodes).
    #[schema(example = "United States")]
    pub registered_country: Option<String>,
    #[schema(example = "US")]
    pub registered_country_iso: Option<String>,
}

impl Location {
    pub fn unknown() -> Self {
        Location {
            city: None,
            region: None,
            region_code: None,
            country: None,
            country_iso: None,
            postal_code: None,
            is_eu: None,
            latitude: None,
            longitude: None,
            timezone: None,
            continent: None,
            continent_code: None,
            accuracy_radius_km: None,
            registered_country: None,
            registered_country_iso: None,
        }
    }
}


#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Classification {
    /// Primary classification: "cloud", "bot", "vpn", "tor", "botnet_c2", "threat", "hosting", "residential", or "internal".
    #[serde(rename = "type")]
    #[schema(example = "residential")]
    pub network_type: String,
    /// True when the IP belongs to a private or reserved range (RFC 1918, loopback, link-local, IPv6 ULA).
    #[schema(example = false)]
    pub is_internal: bool,
    #[schema(example = false)]
    pub is_datacenter: bool,
    #[schema(example = false)]
    pub is_vpn: bool,
    #[schema(example = false)]
    pub is_tor: bool,
    #[schema(example = false)]
    pub is_bot: bool,
    #[schema(example = false)]
    pub is_threat: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Network {
    #[schema(example = 64496)]
    pub asn: Option<u32>,
    #[schema(example = "Example Telecom AG")]
    pub org: Option<String>,
    #[schema(example = json!(null))]
    pub prefix: Option<String>,
    /// Cloud / VPN / hosting / bot provider name, if identified.
    #[schema(example = json!(null))]
    pub provider: Option<String>,
    /// Cloud service name (e.g. "EC2", "Cloud Functions").
    #[schema(example = json!(null))]
    pub service: Option<String>,
    /// Cloud region (e.g. "us-east-1").
    #[schema(example = json!(null))]
    pub region: Option<String>,
    /// Network role from ipverse/as-metadata (e.g. "stub", "tier1_transit").
    #[schema(example = json!(null))]
    pub network_role: Option<String>,
    pub classification: Classification,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Ifconfig {
    pub ip: Ip,
    pub tcp: Option<Tcp>,
    pub location: Location,
    pub network: Network,
    pub user_agent: Option<UserAgent>,
}

pub struct IfconfigParam<'a> {
    pub remote: &'a SocketAddr,
    pub user_agent_header: &'a Option<&'a str>,
    /// `None` when the UA-parser regexes file is not configured/loaded.
    pub user_agent_parser: Option<&'a UserAgentParser>,
    /// `None` when the GeoLite2-City database is not configured/loaded.
    pub geoip_city_db: Option<&'a GeoIpCityDb>,
    /// `None` when the GeoLite2-ASN database is not configured/loaded.
    pub geoip_asn_db: Option<&'a GeoIpAsnDb>,
    pub tor_exit_nodes: &'a TorExitNodes,
    pub feodo_botnet_ips: Option<&'a FeodoBotnetIps>,
    pub vpn_ranges: Option<&'a VpnRanges>,
    pub cloud_provider_db: Option<&'a CloudProviderDb>,
    pub datacenter_ranges: Option<&'a DatacenterRanges>,
    pub bot_db: Option<&'a BotDb>,
    pub spamhaus_drop: Option<&'a SpamhausDrop>,
    pub asn_patterns: &'a asn_heuristic::AsnPatterns,
    pub asn_info: Option<&'a asn_info::AsnInfo>,
    pub dns_resolver: &'a ResolverGroup,
    pub dns_cache: &'a DnsCache,
    /// When true, skip the reverse DNS (PTR) lookup. Used for `?ip=` lookups
    /// where PTR is slow and usually unwanted.
    pub skip_dns: bool,
}

pub async fn get_ifconfig(param: &IfconfigParam<'_>) -> Ifconfig {
    let hostname = if param.skip_dns {
        None
    } else {
        let ip = param.remote.ip();
        // Check cache — lock is released before any await point.
        let cached: Option<Option<String>> = {
            let mut cache = param.dns_cache.lock().unwrap();
            cache.get(&ip).and_then(|entry| {
                if entry.1.elapsed() < DNS_CACHE_TTL {
                    Some(entry.0.clone())
                } else {
                    None // expired — treat as miss
                }
            })
        };
        if let Some(hostname) = cached {
            hostname
        } else {
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                let resolver = param.dns_resolver.resolvers().first()?;
                let query = MultiQuery::single(ip, RecordType::PTR).ok()?;
                let lookups = match resolver.lookup(query).await {
                    Ok(r) => r,
                    Err(e) => { tracing::debug!("PTR lookup error for {ip}: {e}"); return None; }
                };
                lookups.ptr().into_iter().next().map(|name| {
                    let s = name.to_string();
                    s.strip_suffix('.').unwrap_or(&s).to_string()
                })
            })
            .await;
            let result = result.ok().flatten();
            param.dns_cache.lock().unwrap().put(ip, (result.clone(), std::time::Instant::now()));
            result
        }
    };

    let ip_addr = param.remote.ip().to_string();
    let ip_version = if param.remote.is_ipv4() { "4" } else { "6" };
    let ip = Ip {
        addr: ip_addr,
        version: ip_version.to_string(),
        hostname,
    };

    let tcp = if param.remote.port() == 0 {
        None // ?ip= query — port is synthetic (0), omit from response
    } else {
        Some(Tcp { port: param.remote.port() })
    };

    let location = param
        .geoip_city_db
        .and_then(|db| db.lookup(param.remote.ip()))
        .map(|c| {
            let subdivision = c.subdivisions.first();
            Location {
                city: c.city.names.english.map(|s| s.to_owned()),
                region: subdivision.and_then(|s| s.names.english.map(|s| s.to_owned())),
                region_code: subdivision.and_then(|s| s.iso_code.map(|s| s.to_owned())),
                country: c.country.names.english.map(|s| s.to_owned()),
                country_iso: c.country.iso_code.map(|s| s.to_owned()),
                postal_code: c.postal.code.map(|s| s.to_owned()),
                is_eu: c.country.is_in_european_union,
                latitude: c.location.latitude,
                longitude: c.location.longitude,
                timezone: c.location.time_zone.map(|s| s.to_owned()),
                continent: c.continent.names.english.map(|s| s.to_owned()),
                continent_code: c.continent.code.map(|s| s.to_owned()),
                accuracy_radius_km: c.location.accuracy_radius,
                registered_country: c.registered_country.names.english.map(|s| s.to_owned()),
                registered_country_iso: c.registered_country.iso_code.map(|s| s.to_owned()),
            }
        })
        .unwrap_or(Location::unknown());

    let (asn_number, asn_org, asn_prefix) = param
        .geoip_asn_db
        .and_then(|db| db.lookup(param.remote.ip()))
        .map(|(isp, prefix)| (
            isp.autonomous_system_number,
            isp.autonomous_system_organization.map(|s| s.to_owned()),
            prefix,
        ))
        .unwrap_or((None, None, None));

    // --- Classification flags ---
    let is_tor = param
        .tor_exit_nodes
        .lookup(&param.remote.ip())
        .unwrap_or(false);

    let is_botnet_c2 = param
        .feodo_botnet_ips
        .and_then(|db| db.lookup(&param.remote.ip()))
        .unwrap_or(false);

    let cloud = param
        .cloud_provider_db
        .and_then(|db| db.lookup(param.remote.ip()).cloned());

    let vpn_cidr = param
        .vpn_ranges
        .map(|db| db.lookup(param.remote.ip()))
        .unwrap_or(false);

    let bot = param
        .bot_db
        .and_then(|db| db.lookup(param.remote.ip()).cloned());

    let is_threat = param
        .spamhaus_drop
        .map(|db| db.lookup(param.remote.ip()))
        .unwrap_or(false);

    let dc_range_match = param
        .datacenter_ranges
        .map(|db| db.lookup(param.remote.ip()))
        .unwrap_or(false);

    let asn_class = asn_heuristic::classify_asn(asn_number, asn_org.as_deref(), param.asn_patterns);

    let asn_meta = asn_number.and_then(|n| param.asn_info.and_then(|db| db.lookup(n)));

    let is_vpn = vpn_cidr
        || matches!(asn_class, asn_heuristic::AsnClassification::Vpn { .. });

    let is_bot = bot.is_some();

    let is_datacenter = cloud.is_some()
        || dc_range_match
        || matches!(asn_class, asn_heuristic::AsnClassification::Hosting { .. })
        || asn_meta.map(|m| m.category == asn_info::AsnCategory::Hosting).unwrap_or(false);

    // Build network object — primary type uses priority order:
    // cloud > bot > VPN > Tor > botnet_c2 > threat > hosting > residential
    let network = {
        let (network_type, provider) = if cloud.is_some() {
            ("cloud".to_string(), cloud.as_ref().map(|c| c.provider.clone()))
        } else if is_bot {
            ("bot".to_string(), bot.as_ref().map(|b| b.provider.clone()))
        } else if is_vpn {
            let vpn_provider = match &asn_class {
                asn_heuristic::AsnClassification::Vpn { provider } => {
                    Some(provider.to_string())
                }
                _ => None,
            };
            ("vpn".to_string(), vpn_provider)
        } else if is_tor {
            ("tor".to_string(), None)
        } else if is_botnet_c2 {
            ("botnet_c2".to_string(), None)
        } else if is_threat {
            ("threat".to_string(), None)
        } else if is_datacenter {
            let hosting_provider = match &asn_class {
                asn_heuristic::AsnClassification::Hosting { provider } => {
                    Some(provider.to_string())
                }
                _ => None,
            };
            ("hosting".to_string(), hosting_provider)
        } else {
            ("residential".to_string(), None)
        };

        let is_internal = !is_global_ip(param.remote.ip());
        // Internal IPs override the primary type; all other flags still reflect
        // what the enrichment data says (likely all false for private ranges).
        let network_type = if is_internal { "internal".to_string() } else { network_type };

        let network_role = asn_meta.and_then(|m| m.network_role.clone());

        Network {
            asn: asn_number,
            org: asn_org,
            prefix: asn_prefix,
            provider: if is_internal { None } else { provider },
            service: if is_internal { None } else { cloud.as_ref().and_then(|c| c.service.clone()) },
            region: if is_internal { None } else { cloud.as_ref().and_then(|c| c.region.clone()) },
            network_role: if is_internal { None } else { network_role },
            classification: Classification {
                network_type,
                is_internal,
                is_datacenter,
                is_vpn,
                is_tor,
                is_bot,
                is_threat,
            },
        }
    };

    let user_agent = param.user_agent_parser.and_then(|uap| {
        param.user_agent_header.map(|s| {
            let mut ua = uap.parse(s);
            ua.raw = Some(s.to_string());
            ua
        })
    });

    // Emit null-field counters for enrichment quality tracking.
    // Null rate per field = rate(ifconfig_null_field_total{field="..."}[5m])
    //                     / rate(http_requests_total[5m])
    if ip.hostname.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "hostname").increment(1);
    }
    if location.city.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "city").increment(1);
    }
    if location.country.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "country").increment(1);
    }
    if network.asn.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "asn").increment(1);
    }
    if network.org.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "org").increment(1);
    }
    if user_agent.is_none() {
        metrics::counter!("ifconfig_null_field_total", "field" => "user_agent").increment(1);
    }

    Ifconfig {
        ip,
        tcp,
        location,
        network,
        user_agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_global_ip_public_v4() {
        assert!(is_global_ip("203.0.113.1".parse().unwrap()));
        assert!(is_global_ip("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_rejects_private_v4() {
        assert!(!is_global_ip("10.0.0.1".parse().unwrap()));
        assert!(!is_global_ip("172.16.0.1".parse().unwrap()));
        assert!(!is_global_ip("192.168.1.1".parse().unwrap()));
        assert!(!is_global_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_global_ip("169.254.0.1".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_public_v6() {
        assert!(is_global_ip("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn is_global_ip_rejects_private_v6() {
        assert!(!is_global_ip("::1".parse::<IpAddr>().unwrap())); // loopback
        assert!(!is_global_ip("fc00::1".parse::<IpAddr>().unwrap())); // ULA
        assert!(!is_global_ip("fd00::1".parse::<IpAddr>().unwrap())); // ULA
        assert!(!is_global_ip("fe80::1".parse::<IpAddr>().unwrap())); // link-local
        assert!(!is_global_ip("ff02::1".parse::<IpAddr>().unwrap())); // multicast
    }

    #[test]
    fn tor_exit_nodes_empty_returns_none() {
        let nodes = TorExitNodes::empty();
        let addr: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(nodes.lookup(&addr), None);
    }

    #[tokio::test]
    async fn tor_exit_nodes_from_file_missing_returns_none() {
        let nodes = TorExitNodes::from_file("/nonexistent/path/tor_exit_nodes.txt").await;
        let addr: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(nodes.lookup(&addr), None);
    }

    #[test]
    fn tor_exit_nodes_lookup_found() {
        let mut set = HashSet::new();
        set.insert("198.51.100.1".parse::<IpAddr>().unwrap());
        set.insert("203.0.113.5".parse::<IpAddr>().unwrap());
        let nodes = TorExitNodes(Some(set));

        assert_eq!(nodes.lookup(&"198.51.100.1".parse().unwrap()), Some(true));
        assert_eq!(nodes.lookup(&"203.0.113.5".parse().unwrap()), Some(true));
    }

    #[test]
    fn tor_exit_nodes_lookup_not_found() {
        let mut set = HashSet::new();
        set.insert("198.51.100.1".parse::<IpAddr>().unwrap());
        let nodes = TorExitNodes(Some(set));

        assert_eq!(nodes.lookup(&"10.0.0.1".parse().unwrap()), Some(false));
    }

    #[test]
    fn tor_exit_nodes_lookup_ipv6() {
        let mut set = HashSet::new();
        set.insert("2001:db8::1".parse::<IpAddr>().unwrap());
        let nodes = TorExitNodes(Some(set));

        assert_eq!(nodes.lookup(&"2001:db8::1".parse().unwrap()), Some(true));
        assert_eq!(nodes.lookup(&"2001:db8::2".parse().unwrap()), Some(false));
    }
}
