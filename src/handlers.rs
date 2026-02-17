use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use std::net::SocketAddr;

pub(crate) static UNKNOWN_STR: &str = "unknown";

pub(crate) fn make_ifconfig<'a>(
    remote: &'a SocketAddr,
    user_agent: &'a Option<&'a str>,
    user_agent_parser: &'a UserAgentParser,
    geoip_city_db: &'a GeoIpCityDb,
    geoip_asn_db: &'a GeoIpAsnDb,
    tor_exit_nodes: &'a TorExitNodes,
) -> Ifconfig<'a> {
    let param = IfconfigParam {
        remote,
        user_agent_header: user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
    };
    get_ifconfig(&param)
}

macro_rules! handler {
    ($name:ident, $ifconfig:ident, $json:block, $ty:ty, $plain:block) => {
        pub mod $name {
            use crate::backend::*;
            use crate::format::OutputFormat;
            use crate::handlers::make_ifconfig;
            #[allow(unused_imports)]
            use crate::handlers::UNKNOWN_STR;
            use serde_json::Value as JsonValue;
            use std::net::SocketAddr;

            fn to_json($ifconfig: Ifconfig) -> $ty {
                $json
            }

            pub fn json(
                remote: &SocketAddr,
                user_agent: &Option<&str>,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<JsonValue> {
                let ifconfig = make_ifconfig(
                    remote,
                    user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                );
                serde_json::to_value(to_json(ifconfig)).ok()
            }

            fn to_plain($ifconfig: Ifconfig) -> String {
                $plain
            }

            pub fn plain(
                remote: &SocketAddr,
                user_agent: &Option<&str>,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<String> {
                let ifconfig = make_ifconfig(
                    remote,
                    user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                );
                Some(to_plain(ifconfig))
            }

            pub fn formatted(
                format: &OutputFormat,
                remote: &SocketAddr,
                user_agent: &Option<&str>,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<String> {
                let ifconfig = make_ifconfig(
                    remote,
                    user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                );
                let json_val = serde_json::to_value(to_json(ifconfig)).ok()?;
                format.serialize_body(&json_val)
            }
        }
    };
}

handler!(root, ifconfig, { ifconfig }, Ifconfig, {
    format!("{}\n", ifconfig.ip.addr)
});

handler!(ip, ifconfig, { ifconfig.ip }, Ip, { format!("{}\n", ifconfig.ip.addr) });

handler!(tcp, ifconfig, { ifconfig.tcp }, Tcp, {
    format!("{}\n", ifconfig.tcp.port)
});

handler!(host, ifconfig, { ifconfig.host }, Option<Host>, {
    format!(
        "{}\n",
        ifconfig.host.map(|h| h.name).unwrap_or_else(|| UNKNOWN_STR.to_string())
    )
});

handler!(isp, ifconfig, { ifconfig.isp }, Isp, {
    let name = ifconfig.isp.name.unwrap_or(UNKNOWN_STR);
    match ifconfig.isp.asn {
        Some(n) => format!("{} (AS{})\n", name, n),
        None => format!("{}\n", name),
    }
});

handler!(location, ifconfig, { ifconfig.location }, Location, {
    let city = ifconfig.location.city.unwrap_or(UNKNOWN_STR);
    let country = ifconfig.location.country.unwrap_or(UNKNOWN_STR);
    let iso = ifconfig.location.country_iso.unwrap_or(UNKNOWN_STR);
    let continent = ifconfig.location.continent.unwrap_or(UNKNOWN_STR);
    let timezone = ifconfig.location.timezone.unwrap_or(UNKNOWN_STR);
    format!("{}, {} ({}), {}, {}\n", city, country, iso, continent, timezone)
});

handler!(all, ifconfig, { ifconfig }, Ifconfig, {
    let mut lines = Vec::new();
    lines.push(format!("ip:         {}", ifconfig.ip.addr));
    lines.push(format!("version:    {}", ifconfig.ip.version));
    if let Some(ref host) = ifconfig.host {
        lines.push(format!("hostname:   {}", host.name));
    }
    if let Some(city) = ifconfig.location.city {
        lines.push(format!("city:       {}", city));
    }
    if let Some(country) = ifconfig.location.country {
        lines.push(format!("country:    {}", country));
    }
    if let Some(iso) = ifconfig.location.country_iso {
        lines.push(format!("country_iso: {}", iso));
    }
    if let Some(continent) = ifconfig.location.continent {
        lines.push(format!("continent:  {}", continent));
    }
    if let Some(tz) = ifconfig.location.timezone {
        lines.push(format!("timezone:   {}", tz));
    }
    if let Some(lat) = ifconfig.location.latitude {
        lines.push(format!("latitude:   {}", lat));
    }
    if let Some(lon) = ifconfig.location.longitude {
        lines.push(format!("longitude:  {}", lon));
    }
    if let Some(name) = ifconfig.isp.name {
        lines.push(format!("isp:        {}", name));
    }
    if let Some(asn) = ifconfig.isp.asn {
        lines.push(format!("asn:        AS{}", asn));
    }
    if let Some(is_tor) = ifconfig.is_tor {
        lines.push(format!("tor:        {}", is_tor));
    }
    lines.push(format!("port:       {}", ifconfig.tcp.port));
    if let Some(ref ua) = ifconfig.user_agent {
        lines.push(format!("browser:    {} {}", ua.browser.family, ua.browser.version));
        lines.push(format!("os:         {} {}", ua.os.family, ua.os.version));
    }
    lines.join("\n") + "\n"
});

handler!(user_agent, ifconfig, { ifconfig.user_agent }, Option<UserAgent>, {
    format!(
        "{}\n",
        ifconfig
            .user_agent
            .map(|ua| format!(
                "{}, {}, {}, {}",
                ua.browser.family, ua.browser.version, ua.os.family, ua.os.version
            ))
            .unwrap_or_else(|| UNKNOWN_STR.to_string())
    )
});

pub mod headers {
    use crate::format::OutputFormat;
    use serde_json::Value as JsonValue;
    use std::collections::BTreeMap;

    pub fn to_plain(headers: &[(String, String)]) -> String {
        headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    pub fn to_json_value(headers: &[(String, String)]) -> JsonValue {
        let map: BTreeMap<&str, &str> = headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        serde_json::to_value(map).unwrap_or(JsonValue::Null)
    }

    pub fn formatted(format: &OutputFormat, headers: &[(String, String)]) -> Option<String> {
        let json_val = to_json_value(headers);
        format.serialize_body(&json_val)
    }
}

pub mod ip_version {
    use crate::backend::*;
    use crate::format::OutputFormat;
    use crate::handlers::make_ifconfig;
    use serde_json::Value as JsonValue;
    use std::net::SocketAddr;

    fn lookup_ip<'a>(
        version: &str,
        remote: &'a SocketAddr,
        user_agent: &'a Option<&'a str>,
        user_agent_parser: &'a UserAgentParser,
        geoip_city_db: &'a GeoIpCityDb,
        geoip_asn_db: &'a GeoIpAsnDb,
        tor_exit_nodes: &'a TorExitNodes,
    ) -> Option<Ip<'a>> {
        let ifconfig = make_ifconfig(
            remote,
            user_agent,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        );
        if ifconfig.ip.version != version {
            return None;
        }
        Some(ifconfig.ip)
    }

    pub fn json(
        version: &str,
        remote: &SocketAddr,
        user_agent: &Option<&str>,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<JsonValue> {
        let ip = lookup_ip(
            version,
            remote,
            user_agent,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        serde_json::to_value(&ip).ok()
    }

    pub fn plain(
        version: &str,
        remote: &SocketAddr,
        user_agent: &Option<&str>,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<String> {
        let ip = lookup_ip(
            version,
            remote,
            user_agent,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        Some(format!("{}\n", ip.addr))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn formatted(
        version: &str,
        format: &OutputFormat,
        remote: &SocketAddr,
        user_agent: &Option<&str>,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<String> {
        let ip = lookup_ip(
            version,
            remote,
            user_agent,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        let json_val = serde_json::to_value(&ip).ok()?;
        format.serialize_body(&json_val)
    }
}
