use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use mhost::resolver::ResolverGroup;
use std::net::SocketAddr;

pub(crate) static UNKNOWN_STR: &str = "unknown";

pub(crate) async fn make_ifconfig(
    remote: &SocketAddr,
    user_agent: &Option<&str>,
    user_agent_parser: &UserAgentParser,
    geoip_city_db: &GeoIpCityDb,
    geoip_asn_db: &GeoIpAsnDb,
    tor_exit_nodes: &TorExitNodes,
    dns_resolver: &ResolverGroup,
) -> Ifconfig {
    let param = IfconfigParam {
        remote,
        user_agent_header: user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
        dns_resolver,
    };
    get_ifconfig(&param).await
}

pub mod root {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(ifconfig).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.ip.addr)
    }
}

pub mod ip {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.ip).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.ip.addr)
    }
}

pub mod tcp {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.tcp).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.tcp.port)
    }
}

pub mod host {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.host).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!(
            "{}\n",
            ifconfig.host.as_ref().map(|h| h.name.as_str()).unwrap_or(UNKNOWN_STR)
        )
    }
}

pub mod isp {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.isp).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        let name = ifconfig.isp.name.as_deref().unwrap_or(UNKNOWN_STR);
        match ifconfig.isp.asn {
            Some(n) => format!("{} (AS{})\n", name, n),
            None => format!("{}\n", name),
        }
    }
}

pub mod location {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.location).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        let city = ifconfig.location.city.as_deref().unwrap_or(UNKNOWN_STR);
        let country = ifconfig.location.country.as_deref().unwrap_or(UNKNOWN_STR);
        let iso = ifconfig.location.country_iso.as_deref().unwrap_or(UNKNOWN_STR);
        let continent = ifconfig.location.continent.as_deref().unwrap_or(UNKNOWN_STR);
        let timezone = ifconfig.location.timezone.as_deref().unwrap_or(UNKNOWN_STR);
        match ifconfig.location.accuracy_radius_km {
            Some(radius) => format!(
                "{}, {} ({}), {}, {}, ~{}km\n",
                city, country, iso, continent, timezone, radius
            ),
            None => format!("{}, {} ({}), {}, {}\n", city, country, iso, continent, timezone),
        }
    }
}

pub mod all {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(ifconfig).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        let mut lines = Vec::new();
        lines.push(format!("ip:         {}", ifconfig.ip.addr));
        lines.push(format!("version:    {}", ifconfig.ip.version));
        if let Some(ref host) = ifconfig.host {
            lines.push(format!("hostname:   {}", host.name));
        }
        if let Some(ref city) = ifconfig.location.city {
            lines.push(format!("city:       {}", city));
        }
        if let Some(ref country) = ifconfig.location.country {
            lines.push(format!("country:    {}", country));
        }
        if let Some(ref iso) = ifconfig.location.country_iso {
            lines.push(format!("country_iso: {}", iso));
        }
        if let Some(ref continent) = ifconfig.location.continent {
            lines.push(format!("continent:  {}", continent));
        }
        if let Some(ref tz) = ifconfig.location.timezone {
            lines.push(format!("timezone:   {}", tz));
        }
        if let Some(lat) = ifconfig.location.latitude {
            lines.push(format!("latitude:   {}", lat));
        }
        if let Some(lon) = ifconfig.location.longitude {
            lines.push(format!("longitude:  {}", lon));
        }
        if let Some(radius) = ifconfig.location.accuracy_radius_km {
            lines.push(format!("accuracy:   {}km", radius));
        }
        if let Some(ref name) = ifconfig.isp.name {
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
    }
}

pub mod user_agent {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.user_agent).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!(
            "{}\n",
            ifconfig
                .user_agent
                .as_ref()
                .map(|ua| format!(
                    "{}, {}, {}, {}",
                    ua.browser.family, ua.browser.version, ua.os.family, ua.os.version
                ))
                .unwrap_or_else(|| UNKNOWN_STR.to_string())
        )
    }
}

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
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.ip).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.ip.addr)
    }
}
