use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use crate::guards::*;
use crate::ProjectInfo;
use serde::Serialize;
use serde_json::Value as JsonValue;

pub(crate) static UNKNOWN_STR: &str = "unknown";

pub(crate) fn make_ifconfig<'a>(
    req_info: &'a RequesterInfo<'a>,
    user_agent_parser: &'a UserAgentParser,
    geoip_city_db: &'a GeoIpCityDb,
    geoip_asn_db: &'a GeoIpAsnDb,
    tor_exit_nodes: &'a TorExitNodes,
) -> Ifconfig<'a> {
    let param = IfconfigParam {
        remote: &req_info.remote,
        user_agent_header: &req_info.user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
    };
    get_ifconfig(&param)
}

#[derive(Serialize)]
pub struct RootHtmlContext {
    pub ifconfig: JsonValue,
    pub project: JsonValue,
    pub uri: String,
}

pub fn root_html(
    project_info: &ProjectInfo,
    req_info: &RequesterInfo,
    user_agent_parser: &UserAgentParser,
    geoip_city_db: &GeoIpCityDb,
    geoip_asn_db: &GeoIpAsnDb,
    tor_exit_nodes: &TorExitNodes,
) -> RootHtmlContext {
    let ifconfig = make_ifconfig(req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes);

    RootHtmlContext {
        ifconfig: serde_json::to_value(&ifconfig).unwrap_or_default(),
        project: serde_json::to_value(project_info).unwrap_or_default(),
        uri: req_info.uri.clone(),
    }
}

macro_rules! handler {
    ($name:ident, $ifconfig:ident, $json:block, $ty:ty, $plain:block) => {
        pub mod $name {
            use crate::backend::*;
            use crate::format::OutputFormat;
            use crate::guards::*;
            #[allow(unused_imports)]
            use crate::handlers::UNKNOWN_STR;
            use crate::handlers::make_ifconfig;
            use serde_json::Value as JsonValue;

            fn to_json($ifconfig: Ifconfig) -> $ty {
                $json
            }

            pub fn json(
                req_info: &RequesterInfo,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<JsonValue> {
                let ifconfig = make_ifconfig(req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes);
                serde_json::to_value(to_json(ifconfig)).ok()
            }

            fn to_plain($ifconfig: Ifconfig) -> String {
                $plain
            }

            pub fn plain(
                req_info: &RequesterInfo,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<String> {
                let ifconfig = make_ifconfig(req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes);
                Some(to_plain(ifconfig))
            }

            pub fn formatted(
                format: &OutputFormat,
                req_info: &RequesterInfo,
                user_agent_parser: &UserAgentParser,
                geoip_city_db: &GeoIpCityDb,
                geoip_asn_db: &GeoIpAsnDb,
                tor_exit_nodes: &TorExitNodes,
            ) -> Option<String> {
                let ifconfig = make_ifconfig(req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes);
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

handler!(isp, ifconfig, { ifconfig.isp }, Option<Isp>, {
    let name = ifconfig.isp.as_ref().and_then(|isp| isp.name).unwrap_or(UNKNOWN_STR);
    let asn = ifconfig.isp.as_ref().and_then(|isp| isp.asn);
    match asn {
        Some(n) => format!("{} (AS{})\n", name, n),
        None => format!("{}\n", name),
    }
});

handler!(location, ifconfig, { ifconfig.location }, Option<Location>, {
    let city = ifconfig.location.as_ref().and_then(|l| l.city).unwrap_or(UNKNOWN_STR);
    let country = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.country)
        .unwrap_or(UNKNOWN_STR);
    let iso = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.country_iso)
        .unwrap_or(UNKNOWN_STR);
    let continent = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.continent)
        .unwrap_or(UNKNOWN_STR);
    let timezone = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.timezone)
        .unwrap_or(UNKNOWN_STR);
    format!("{}, {} ({}), {}, {}\n", city, country, iso, continent, timezone)
});

handler!(all, ifconfig, { ifconfig }, Ifconfig, {
    let mut lines = Vec::new();
    lines.push(format!("ip:         {}", ifconfig.ip.addr));
    lines.push(format!("version:    {}", ifconfig.ip.version));
    if let Some(ref host) = ifconfig.host {
        lines.push(format!("hostname:   {}", host.name));
    }
    if let Some(ref loc) = ifconfig.location {
        if let Some(city) = loc.city {
            lines.push(format!("city:       {}", city));
        }
        if let Some(country) = loc.country {
            lines.push(format!("country:    {}", country));
        }
        if let Some(iso) = loc.country_iso {
            lines.push(format!("country_iso: {}", iso));
        }
        if let Some(continent) = loc.continent {
            lines.push(format!("continent:  {}", continent));
        }
        if let Some(tz) = loc.timezone {
            lines.push(format!("timezone:   {}", tz));
        }
        if let Some(lat) = loc.latitude {
            lines.push(format!("latitude:   {}", lat));
        }
        if let Some(lon) = loc.longitude {
            lines.push(format!("longitude:  {}", lon));
        }
    }
    if let Some(ref isp) = ifconfig.isp {
        if let Some(name) = isp.name {
            lines.push(format!("isp:        {}", name));
        }
        if let Some(asn) = isp.asn {
            lines.push(format!("asn:        AS{}", asn));
        }
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
    use crate::guards::RequestHeaders;
    use serde_json::Value as JsonValue;
    use std::collections::BTreeMap;

    pub fn to_plain(req_headers: &RequestHeaders) -> String {
        req_headers
            .headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    pub fn to_json_value(req_headers: &RequestHeaders) -> JsonValue {
        let map: BTreeMap<&str, &str> = req_headers
            .headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        serde_json::to_value(map).unwrap_or(JsonValue::Null)
    }

    pub fn formatted(format: &OutputFormat, req_headers: &RequestHeaders) -> Option<String> {
        let json_val = to_json_value(req_headers);
        format.serialize_body(&json_val)
    }
}

pub mod ip_version {
    use crate::backend::*;
    use crate::format::OutputFormat;
    use crate::guards::*;
    use crate::handlers::make_ifconfig;
    use serde_json::Value as JsonValue;

    fn lookup_ip<'a>(
        version: &str,
        req_info: &'a RequesterInfo<'a>,
        user_agent_parser: &'a UserAgentParser,
        geoip_city_db: &'a GeoIpCityDb,
        geoip_asn_db: &'a GeoIpAsnDb,
        tor_exit_nodes: &'a TorExitNodes,
    ) -> Option<Ip<'a>> {
        let ifconfig = make_ifconfig(req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes);
        if ifconfig.ip.version != version {
            return None;
        }
        Some(ifconfig.ip)
    }

    pub fn json(
        version: &str,
        req_info: &RequesterInfo,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<JsonValue> {
        let ip = lookup_ip(version, req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
        serde_json::to_value(&ip).ok()
    }

    pub fn plain(
        version: &str,
        req_info: &RequesterInfo,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<String> {
        let ip = lookup_ip(version, req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
        Some(format!("{}\n", ip.addr))
    }

    pub fn formatted(
        version: &str,
        format: &OutputFormat,
        req_info: &RequesterInfo,
        user_agent_parser: &UserAgentParser,
        geoip_city_db: &GeoIpCityDb,
        geoip_asn_db: &GeoIpAsnDb,
        tor_exit_nodes: &TorExitNodes,
    ) -> Option<String> {
        let ip = lookup_ip(version, req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
        let json_val = serde_json::to_value(&ip).ok()?;
        format.serialize_body(&json_val)
    }
}
