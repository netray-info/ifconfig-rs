use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use mhost::resolver::ResolverGroup;
use std::net::SocketAddr;

pub(crate) static UNKNOWN_STR: &str = "unknown";

#[allow(clippy::too_many_arguments)]
pub(crate) async fn make_ifconfig(
    remote: &SocketAddr,
    user_agent: &Option<&str>,
    user_agent_parser: &UserAgentParser,
    geoip_city_db: &GeoIpCityDb,
    geoip_asn_db: &GeoIpAsnDb,
    tor_exit_nodes: &TorExitNodes,
    feodo_botnet_ips: Option<&FeodoBotnetIps>,
    vpn_ranges: Option<&VpnRanges>,
    cloud_provider_db: Option<&CloudProviderDb>,
    datacenter_ranges: Option<&DatacenterRanges>,
    bot_db: Option<&BotDb>,
    spamhaus_drop: Option<&SpamhausDrop>,
    dns_resolver: &ResolverGroup,
    skip_dns: bool,
) -> Ifconfig {
    let param = IfconfigParam {
        remote,
        user_agent_header: user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
        feodo_botnet_ips,
        vpn_ranges,
        cloud_provider_db,
        datacenter_ranges,
        bot_db,
        spamhaus_drop,
        dns_resolver,
        skip_dns,
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
        ifconfig.tcp.as_ref().and_then(|t| serde_json::to_value(t).ok())
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        match &ifconfig.tcp {
            Some(t) => format!("{}\n", t.port),
            None => "\n".to_string(),
        }
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
        let region = ifconfig.location.region.as_deref();
        let country = ifconfig.location.country.as_deref().unwrap_or(UNKNOWN_STR);
        let iso = ifconfig.location.country_iso.as_deref().unwrap_or(UNKNOWN_STR);
        let continent = ifconfig.location.continent.as_deref().unwrap_or(UNKNOWN_STR);
        let timezone = ifconfig.location.timezone.as_deref().unwrap_or(UNKNOWN_STR);
        let location_part = match region {
            Some(r) => format!("{}, {}, {} ({})", city, r, country, iso),
            None => format!("{}, {} ({})", city, country, iso),
        };
        match ifconfig.location.accuracy_radius_km {
            Some(radius) => format!("{}, {}, {}, ~{}km\n", location_part, continent, timezone, radius),
            None => format!("{}, {}, {}\n", location_part, continent, timezone),
        }
    }
}

pub mod network {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.network).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        match ifconfig.network {
            Some(ref n) => {
                let mut lines = Vec::new();
                lines.push(format!("type:       {}", n.network_type));
                if let Some(ref provider) = n.provider {
                    lines.push(format!("provider:   {}", provider));
                }
                if let Some(ref service) = n.service {
                    lines.push(format!("service:    {}", service));
                }
                if let Some(ref region) = n.region {
                    lines.push(format!("region:     {}", region));
                }
                lines.push(format!("datacenter: {}", n.is_datacenter));
                lines.push(format!("vpn:        {}", n.is_vpn));
                lines.push(format!("tor:        {}", n.is_tor));
                lines.push(format!("proxy:      {}", n.is_proxy));
                lines.push(format!("bot:        {}", n.is_bot));
                lines.push(format!("threat:     {}", n.is_threat));
                lines.join("\n") + "\n"
            }
            None => format!("{}\n", UNKNOWN_STR),
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
        if let Some(ref region) = ifconfig.location.region {
            lines.push(format!("region:     {}", region));
        }
        if let Some(ref region_code) = ifconfig.location.region_code {
            lines.push(format!("region_code: {}", region_code));
        }
        if let Some(ref country) = ifconfig.location.country {
            lines.push(format!("country:    {}", country));
        }
        if let Some(ref iso) = ifconfig.location.country_iso {
            lines.push(format!("country_iso: {}", iso));
        }
        if let Some(ref postal_code) = ifconfig.location.postal_code {
            lines.push(format!("postal_code: {}", postal_code));
        }
        if let Some(is_eu) = ifconfig.location.is_eu {
            lines.push(format!("is_eu:      {}", is_eu));
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
        if let Some(ref n) = ifconfig.network {
            lines.push(format!("network:    {}", n.network_type));
            if let Some(ref provider) = n.provider {
                lines.push(format!("provider:   {}", provider));
            }
            if let Some(ref service) = n.service {
                lines.push(format!("service:    {}", service));
            }
            if let Some(ref region) = n.region {
                lines.push(format!("region:     {}", region));
            }
            lines.push(format!("datacenter: {}", n.is_datacenter));
            lines.push(format!("vpn:        {}", n.is_vpn));
            lines.push(format!("tor:        {}", n.is_tor));
            lines.push(format!("proxy:      {}", n.is_proxy));
            lines.push(format!("bot:        {}", n.is_bot));
            lines.push(format!("threat:     {}", n.is_threat));
        }
        if let Some(ref t) = ifconfig.tcp {
            lines.push(format!("port:       {}", t.port));
        }
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
