pub mod asn_heuristic;
pub mod bot;
pub mod cloud_provider;
pub mod datacenter;
pub mod feodo;
pub mod spamhaus;
pub mod user_agent;
pub mod vpn;
pub use bot::{BotDb, BotInfo};
pub use cloud_provider::{CloudProvider, CloudProviderDb};
pub use datacenter::DatacenterRanges;
pub use feodo::FeodoBotnetIps;
pub use spamhaus::SpamhausDrop;
pub use user_agent::*;
pub use vpn::VpnRanges;

use maxminddb::{self, geoip2};
use mhost::resolver::{MultiQuery, ResolverGroup};
use mhost::RecordType;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Host {
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Ip {
    pub addr: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Tcp {
    pub port: u16,
}

pub struct GeoIpCityDb(maxminddb::Reader<Vec<u8>>);

impl GeoIpCityDb {
    pub fn new(db_path: &str) -> Option<Self> {
        maxminddb::Reader::open_readfile(db_path).ok().map(GeoIpCityDb)
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
    pub fn new(db_path: &str) -> Option<Self> {
        maxminddb::Reader::open_readfile(db_path).ok().map(GeoIpAsnDb)
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<geoip2::Isp<'_>> {
        self.0.lookup(ip).ok().and_then(|r| r.decode().ok().flatten())
    }
}

pub struct TorExitNodes(Option<HashSet<IpAddr>>);

impl TorExitNodes {
    pub fn from_file(path: &str) -> Self {
        let set = std::fs::read_to_string(path)
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

    pub fn lookup(&self, addr: &IpAddr) -> Option<bool> {
        self.0.as_ref().map(|set| set.contains(addr))
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Location {
    pub city: Option<String>,
    pub region: Option<String>,
    pub region_code: Option<String>,
    pub country: Option<String>,
    pub country_iso: Option<String>,
    pub postal_code: Option<String>,
    pub is_eu: Option<bool>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
    pub continent: Option<String>,
    pub continent_code: Option<String>,
    pub accuracy_radius_km: Option<u16>,
}

impl Location {
    pub fn unknown() -> Self {
        Location {
            city: Some("unknown".to_string()),
            region: None,
            region_code: None,
            country: Some("unknown".to_string()),
            country_iso: Some("unknown".to_string()),
            postal_code: None,
            is_eu: None,
            latitude: None,
            longitude: None,
            timezone: Some("unknown".to_string()),
            continent: Some("unknown".to_string()),
            continent_code: Some("unknown".to_string()),
            accuracy_radius_km: None,
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Isp {
    pub name: Option<String>,
    pub asn: Option<u32>,
}

impl Isp {
    pub fn unknown() -> Self {
        Isp {
            name: Some("unknown".to_string()),
            asn: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Network {
    /// Primary classification: "cloud", "bot", "vpn", "tor", "botnet_c2", "threat", "hosting", or "residential".
    #[serde(rename = "type")]
    pub network_type: String,
    /// Cloud / VPN / hosting / bot provider name, if identified.
    pub provider: Option<String>,
    /// Cloud service name (e.g. "EC2", "Cloud Functions").
    pub service: Option<String>,
    /// Cloud region (e.g. "us-east-1").
    pub region: Option<String>,
    pub is_datacenter: bool,
    pub is_vpn: bool,
    pub is_tor: bool,
    pub is_proxy: bool,
    pub is_bot: bool,
    pub is_threat: bool,
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Ifconfig {
    pub host: Option<Host>,
    pub ip: Ip,
    pub tcp: Tcp,
    pub location: Location,
    pub isp: Isp,
    pub network: Option<Network>,
    pub user_agent: Option<UserAgent>,
    pub user_agent_header: Option<String>,
}

pub struct IfconfigParam<'a> {
    pub remote: &'a SocketAddr,
    pub user_agent_header: &'a Option<&'a str>,
    pub user_agent_parser: &'a UserAgentParser,
    pub geoip_city_db: &'a GeoIpCityDb,
    pub geoip_asn_db: &'a GeoIpAsnDb,
    pub tor_exit_nodes: &'a TorExitNodes,
    pub feodo_botnet_ips: Option<&'a FeodoBotnetIps>,
    pub vpn_ranges: Option<&'a VpnRanges>,
    pub cloud_provider_db: Option<&'a CloudProviderDb>,
    pub datacenter_ranges: Option<&'a DatacenterRanges>,
    pub bot_db: Option<&'a BotDb>,
    pub spamhaus_drop: Option<&'a SpamhausDrop>,
    pub dns_resolver: &'a ResolverGroup,
    /// When true, skip the reverse DNS (PTR) lookup. Used for `?ip=` lookups
    /// where PTR is slow and usually unwanted.
    pub skip_dns: bool,
}

pub async fn get_ifconfig(param: &IfconfigParam<'_>) -> Ifconfig {
    let host = if param.skip_dns {
        None
    } else {
        async {
            let resolver = param.dns_resolver.resolvers().first()?;
            let query = MultiQuery::single(param.remote.ip(), RecordType::PTR).ok()?;
            let lookups = resolver.lookup(query).await.ok()?;
            lookups.ptr().into_iter().next().map(|name| {
                let s = name.to_string();
                Host {
                    name: s.strip_suffix('.').unwrap_or(&s).to_string(),
                }
            })
        }
        .await
    };

    let ip_addr = param.remote.ip().to_string();
    let ip_version = if param.remote.is_ipv4() { "4" } else { "6" };
    let ip = Ip {
        addr: ip_addr,
        version: ip_version.to_string(),
    };

    let tcp = Tcp {
        port: param.remote.port(),
    };

    let location = param
        .geoip_city_db
        .lookup(param.remote.ip())
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
            }
        })
        .unwrap_or(Location::unknown());

    let isp = param
        .geoip_asn_db
        .lookup(param.remote.ip())
        .map(|isp| Isp {
            name: isp.autonomous_system_organization.map(|s| s.to_owned()),
            asn: isp.autonomous_system_number,
        })
        .unwrap_or(Isp::unknown());

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

    let asn_class = isp
        .name
        .as_deref()
        .map(asn_heuristic::classify_asn)
        .unwrap_or(asn_heuristic::AsnClassification::None);

    let is_vpn = vpn_cidr
        || matches!(asn_class, asn_heuristic::AsnClassification::Vpn { .. });

    let is_bot = bot.is_some();

    let is_datacenter = cloud.is_some()
        || dc_range_match
        || matches!(asn_class, asn_heuristic::AsnClassification::Hosting { .. });

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

        Network {
            network_type,
            provider,
            service: cloud.as_ref().and_then(|c| c.service.clone()),
            region: cloud.as_ref().and_then(|c| c.region.clone()),
            is_datacenter,
            is_vpn,
            is_tor,
            is_proxy: false,
            is_bot,
            is_threat,
        }
    };

    let user_agent = param.user_agent_header.map(|s| param.user_agent_parser.parse(s));

    Ifconfig {
        host,
        ip,
        tcp,
        location,
        isp,
        network: Some(network),
        user_agent,
        user_agent_header: param.user_agent_header.map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tor_exit_nodes_empty_returns_none() {
        let nodes = TorExitNodes::empty();
        let addr: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(nodes.lookup(&addr), None);
    }

    #[test]
    fn tor_exit_nodes_from_file_missing_returns_none() {
        let nodes = TorExitNodes::from_file("/nonexistent/path/tor_exit_nodes.txt");
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
