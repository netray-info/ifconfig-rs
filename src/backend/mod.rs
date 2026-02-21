pub mod user_agent;
pub use user_agent::*;

use hickory_resolver::TokioResolver;
use maxminddb::{self, geoip2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Host {
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Ip {
    pub addr: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Location {
    pub city: Option<String>,
    pub country: Option<String>,
    pub country_iso: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
    pub continent: Option<String>,
    pub continent_code: Option<String>,
}

impl Location {
    pub fn unknown() -> Self {
        Location {
            city: Some("unknown".to_string()),
            country: Some("unknown".to_string()),
            country_iso: Some("unknown".to_string()),
            latitude: None,
            longitude: None,
            timezone: Some("unknown".to_string()),
            continent: Some("unknown".to_string()),
            continent_code: Some("unknown".to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Ifconfig {
    pub host: Option<Host>,
    pub ip: Ip,
    pub tcp: Tcp,
    pub location: Location,
    pub isp: Isp,
    pub is_tor: Option<bool>,
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
    pub dns_resolver: &'a TokioResolver,
}

pub async fn get_ifconfig(param: &IfconfigParam<'_>) -> Ifconfig {
    let host = param
        .dns_resolver
        .reverse_lookup(param.remote.ip())
        .await
        .ok()
        .and_then(|lookup| {
            lookup.into_iter().next().map(|name| {
                let s = name.to_string();
                Host {
                    name: s.strip_suffix('.').unwrap_or(&s).to_string(),
                }
            })
        });

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
        .map(|c| Location {
            city: c.city.names.english.map(|s| s.to_owned()),
            country: c.country.names.english.map(|s| s.to_owned()),
            country_iso: c.country.iso_code.map(|s| s.to_owned()),
            latitude: c.location.latitude,
            longitude: c.location.longitude,
            timezone: c.location.time_zone.map(|s| s.to_owned()),
            continent: c.continent.names.english.map(|s| s.to_owned()),
            continent_code: c.continent.code.map(|s| s.to_owned()),
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

    let is_tor = param.tor_exit_nodes.lookup(&param.remote.ip());

    let user_agent = param.user_agent_header.map(|s| param.user_agent_parser.parse(s));

    Ifconfig {
        host,
        ip,
        tcp,
        location,
        isp,
        is_tor,
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
