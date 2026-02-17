pub mod user_agent;
pub use user_agent::*;

use maxminddb::{self, geoip2};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

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

pub struct GeoIpCityDb(pub maxminddb::Reader<Vec<u8>>);

impl GeoIpCityDb {
    pub fn new(db_path: &str) -> Option<Self> {
        maxminddb::Reader::open_readfile(db_path).ok().map(GeoIpCityDb)
    }
}

pub struct GeoIpAsnDb(pub maxminddb::Reader<Vec<u8>>);

impl GeoIpAsnDb {
    pub fn new(db_path: &str) -> Option<Self> {
        maxminddb::Reader::open_readfile(db_path).ok().map(GeoIpAsnDb)
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Location<'a> {
    pub city: Option<&'a str>,
    pub country: Option<&'a str>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Isp<'a> {
    pub name: Option<&'a str>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Ifconfig<'a> {
    pub host: Option<Host>,
    pub ip: Ip<'a>,
    pub tcp: Tcp,
    pub location: Option<Location<'a>>,
    pub isp: Option<Isp<'a>>,
    pub user_agent: Option<UserAgent>,
    pub user_agent_header: Option<&'a str>,
}

pub struct IfconfigParam<'a> {
    pub remote: &'a SocketAddr,
    pub user_agent_header: &'a Option<&'a str>,
    pub user_agent_parser: &'a UserAgentParser,
    pub geoip_city_db: &'a GeoIpCityDb,
    pub geoip_asn_db: &'a GeoIpAsnDb,
}

pub fn get_ifconfig<'a>(param: &'a IfconfigParam<'a>) -> Ifconfig<'a> {
    let host = dns_lookup::lookup_addr(&param.remote.ip())
        .ok()
        .map(|h| Host { name: h });

    let ip_addr = format!("{}", param.remote.ip());
    let ip_version = if param.remote.is_ipv4() { "4" } else { "6" };
    let ip = Ip {
        addr: ip_addr,
        version: ip_version,
    };

    let tcp = Tcp {
        port: param.remote.port(),
    };

    let geo_city: Option<geoip2::City> = param
        .geoip_city_db
        .0
        .lookup(param.remote.ip())
        .ok()
        .and_then(|r| r.decode().ok().flatten());
    let location = geo_city.map(|c| Location {
        city: c.city.names.english,
        country: c.country.names.english,
        latitude: c.location.latitude,
        longitude: c.location.longitude,
    });

    let geo_isp: Option<geoip2::Isp> = param
        .geoip_asn_db
        .0
        .lookup(param.remote.ip())
        .ok()
        .and_then(|r| r.decode().ok().flatten());
    let isp = geo_isp.map(|isp| Isp {
        name: isp.autonomous_system_organization,
    });

    let user_agent = param.user_agent_header.map(|s| param.user_agent_parser.parse(s));

    Ifconfig {
        host,
        ip,
        tcp,
        location,
        isp,
        user_agent,
        user_agent_header: *param.user_agent_header,
    }
}
