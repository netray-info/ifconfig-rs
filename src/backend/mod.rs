pub mod user_agent;
pub use user_agent::*;

use maxminddb::{self, geoip2};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Host {
    pub name: String,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Ip<'a> {
    pub addr: String,
    pub version: &'a str,
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
pub struct Location<'a> {
    pub city: Option<&'a str>,
    pub country: Option<&'a str>,
    pub country_iso: Option<&'a str>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<&'a str>,
    pub continent: Option<&'a str>,
    pub continent_code: Option<&'a str>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Isp<'a> {
    pub name: Option<&'a str>,
    pub asn: Option<u32>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Ifconfig<'a> {
    pub host: Option<Host>,
    pub ip: Ip<'a>,
    pub tcp: Tcp,
    pub location: Option<Location<'a>>,
    pub isp: Option<Isp<'a>>,
    pub is_tor: Option<bool>,
    pub user_agent: Option<UserAgent>,
    pub user_agent_header: Option<&'a str>,
}

pub struct IfconfigParam<'a> {
    pub remote: &'a SocketAddr,
    pub user_agent_header: &'a Option<&'a str>,
    pub user_agent_parser: &'a UserAgentParser,
    pub geoip_city_db: &'a GeoIpCityDb,
    pub geoip_asn_db: &'a GeoIpAsnDb,
    pub tor_exit_nodes: &'a TorExitNodes,
}

pub fn get_ifconfig<'a>(param: &IfconfigParam<'a>) -> Ifconfig<'a> {
    let host = dns_lookup::lookup_addr(&param.remote.ip())
        .ok()
        .map(|h| Host { name: h });

    let ip_addr = param.remote.ip().to_string();
    let ip_version = if param.remote.is_ipv4() { "4" } else { "6" };
    let ip = Ip {
        addr: ip_addr,
        version: ip_version,
    };

    let tcp = Tcp {
        port: param.remote.port(),
    };

    let geo_city = param.geoip_city_db.lookup(param.remote.ip());
    let location = geo_city.map(|c| Location {
        city: c.city.names.english,
        country: c.country.names.english,
        country_iso: c.country.iso_code,
        latitude: c.location.latitude,
        longitude: c.location.longitude,
        timezone: c.location.time_zone,
        continent: c.continent.names.english,
        continent_code: c.continent.code,
    });

    let geo_isp = param.geoip_asn_db.lookup(param.remote.ip());
    let isp = geo_isp.map(|isp| Isp {
        name: isp.autonomous_system_organization,
        asn: isp.autonomous_system_number,
    });

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
        user_agent_header: *param.user_agent_header,
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
