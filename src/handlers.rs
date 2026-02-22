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
            None => format!("{}\n", UNKNOWN_STR),
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
        let n = &ifconfig.network;
        let mut lines = Vec::new();
        if let Some(ref org) = n.org {
            lines.push(format!("org:        {}", org));
        }
        if let Some(asn) = n.asn {
            lines.push(format!("asn:        AS{}", asn));
        }
        if let Some(ref prefix) = n.prefix {
            lines.push(format!("prefix:     {}", prefix));
        }
        lines.push(format!("type:       {}", n.classification.network_type));
        if let Some(ref provider) = n.provider {
            lines.push(format!("provider:   {}", provider));
        }
        if let Some(ref service) = n.service {
            lines.push(format!("service:    {}", service));
        }
        if let Some(ref region) = n.region {
            lines.push(format!("region:     {}", region));
        }
        lines.push(format!("datacenter: {}", n.classification.is_datacenter));
        lines.push(format!("vpn:        {}", n.classification.is_vpn));
        lines.push(format!("tor:        {}", n.classification.is_tor));
        lines.push(format!("proxy:      {}", n.classification.is_proxy));
        lines.push(format!("bot:        {}", n.classification.is_bot));
        lines.push(format!("threat:     {}", n.classification.is_threat));
        lines.join("\n") + "\n"
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
        if let Some(ref hostname) = ifconfig.ip.hostname {
            lines.push(format!("hostname:   {}", hostname));
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
        {
            let n = &ifconfig.network;
            if let Some(ref org) = n.org {
                lines.push(format!("org:        {}", org));
            }
            if let Some(asn) = n.asn {
                lines.push(format!("asn:        AS{}", asn));
            }
            if let Some(ref prefix) = n.prefix {
                lines.push(format!("prefix:     {}", prefix));
            }
            lines.push(format!("network:    {}", n.classification.network_type));
            if let Some(ref provider) = n.provider {
                lines.push(format!("provider:   {}", provider));
            }
            if let Some(ref service) = n.service {
                lines.push(format!("service:    {}", service));
            }
            if let Some(ref region) = n.region {
                lines.push(format!("region:     {}", region));
            }
            lines.push(format!("datacenter: {}", n.classification.is_datacenter));
            lines.push(format!("vpn:        {}", n.classification.is_vpn));
            lines.push(format!("tor:        {}", n.classification.is_tor));
            lines.push(format!("proxy:      {}", n.classification.is_proxy));
            lines.push(format!("bot:        {}", n.classification.is_bot));
            lines.push(format!("threat:     {}", n.classification.is_threat));
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

pub mod country {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.location.country).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.location.country.as_deref().unwrap_or(UNKNOWN_STR))
    }
}

pub mod city {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.location.city).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.location.city.as_deref().unwrap_or(UNKNOWN_STR))
    }
}

pub mod asn {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(ifconfig.network.asn).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        match ifconfig.network.asn {
            Some(n) => format!("AS{}\n", n),
            None => format!("{}\n", UNKNOWN_STR),
        }
    }
}

pub mod timezone {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.location.timezone).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.location.timezone.as_deref().unwrap_or(UNKNOWN_STR))
    }
}

pub mod latitude {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(ifconfig.location.latitude).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        match ifconfig.location.latitude {
            Some(lat) => format!("{:.4}\n", lat),
            None => format!("{}\n", UNKNOWN_STR),
        }
    }
}

pub mod longitude {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(ifconfig.location.longitude).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        match ifconfig.location.longitude {
            Some(lon) => format!("{:.4}\n", lon),
            None => format!("{}\n", UNKNOWN_STR),
        }
    }
}

pub mod region {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.location.region).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.location.region.as_deref().unwrap_or(UNKNOWN_STR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ifconfig(hostname: Option<&str>, tcp_port: Option<u16>, network: Network) -> Ifconfig {
        Ifconfig {
            ip: Ip { addr: "203.0.113.42".to_string(), version: "4".to_string(), hostname: hostname.map(|s| s.to_string()) },
            tcp: tcp_port.map(|p| Tcp { port: p }),
            location: Location {
                city: Some("Berlin".to_string()),
                region: Some("Berlin".to_string()),
                region_code: Some("BE".to_string()),
                country: Some("Germany".to_string()),
                country_iso: Some("DE".to_string()),
                postal_code: Some("10115".to_string()),
                is_eu: Some(true),
                latitude: Some(52.52),
                longitude: Some(13.405),
                timezone: Some("Europe/Berlin".to_string()),
                continent: Some("Europe".to_string()),
                continent_code: Some("EU".to_string()),
                accuracy_radius_km: Some(100),
                registered_country: None,
                registered_country_iso: None,
            },
            network,
            user_agent: None,
        }
    }

    fn residential_network() -> Network {
        Network {
            asn: Some(64496),
            org: Some("Example Telecom".to_string()),
            prefix: None,
            provider: None,
            service: None,
            region: None,
            classification: Classification {
                network_type: "residential".to_string(),
                is_datacenter: false,
                is_vpn: false,
                is_tor: false,
                is_proxy: false,
                is_bot: false,
                is_threat: false,
            },
        }
    }

    // --- root ---

    #[test]
    fn root_to_plain_returns_ip() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(12345), residential_network());
        assert_eq!(root::to_plain(&ifc), "203.0.113.42\n");
    }

    #[test]
    fn root_to_json_has_all_fields() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(12345), residential_network());
        let val = root::to_json(&ifc).unwrap();
        assert!(val["ip"]["addr"].is_string());
        assert!(val["ip"]["hostname"].is_string());
        assert!(val["location"]["city"].is_string());
    }

    // --- tcp ---

    #[test]
    fn tcp_to_plain_with_port() {
        let ifc = make_ifconfig(None, Some(54321), residential_network());
        assert_eq!(tcp::to_plain(&ifc), "54321\n");
    }

    #[test]
    fn tcp_to_plain_none() {
        let ifc = make_ifconfig(None, None, residential_network());
        assert_eq!(tcp::to_plain(&ifc), "unknown\n");
    }

    #[test]
    fn tcp_to_json_none() {
        let ifc = make_ifconfig(None, None, residential_network());
        assert!(tcp::to_json(&ifc).is_none());
    }

    #[test]
    fn tcp_to_json_some() {
        let ifc = make_ifconfig(None, Some(8080), residential_network());
        let val = tcp::to_json(&ifc).unwrap();
        assert_eq!(val["port"], 8080);
    }

    // --- location ---

    #[test]
    fn location_to_plain_full() {
        let ifc = make_ifconfig(None, None, residential_network());
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("Berlin"));
        assert!(plain.contains("Germany"));
        assert!(plain.contains("DE"));
        assert!(plain.contains("Europe"));
        assert!(plain.contains("~100km"));
    }

    #[test]
    fn location_to_plain_no_region() {
        let mut ifc = make_ifconfig(None, None, residential_network());
        ifc.location.region = None;
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("Berlin, Germany (DE)"));
    }

    #[test]
    fn location_to_plain_no_accuracy() {
        let mut ifc = make_ifconfig(None, None, residential_network());
        ifc.location.accuracy_radius_km = None;
        let plain = location::to_plain(&ifc);
        assert!(!plain.contains("~"));
        assert!(plain.ends_with("Europe/Berlin\n"));
    }

    #[test]
    fn location_to_plain_unknown() {
        let mut ifc = make_ifconfig(None, None, residential_network());
        ifc.location = Location::unknown();
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("unknown"));
    }

    // --- network ---

    #[test]
    fn network_to_plain_residential() {
        let ifc = make_ifconfig(None, None, residential_network());
        let plain = network::to_plain(&ifc);
        assert!(plain.contains("org:        Example Telecom"));
        assert!(plain.contains("asn:        AS64496"));
        assert!(plain.contains("type:       residential"));
        assert!(plain.contains("datacenter: false"));
        assert!(!plain.contains("provider:"));
        assert!(!plain.contains("prefix:"));
    }

    #[test]
    fn network_to_plain_cloud_with_provider() {
        let n = Network {
            asn: Some(16509),
            org: Some("Amazon.com".to_string()),
            prefix: Some("54.0.0.0/8".to_string()),
            provider: Some("AWS".to_string()),
            service: Some("EC2".to_string()),
            region: Some("us-east-1".to_string()),
            classification: Classification {
                network_type: "cloud".to_string(),
                is_datacenter: true,
                is_vpn: false,
                is_tor: false,
                is_proxy: false,
                is_bot: false,
                is_threat: false,
            },
        };
        let ifc = make_ifconfig(None, None, n);
        let plain = network::to_plain(&ifc);
        assert!(plain.contains("org:        Amazon.com"));
        assert!(plain.contains("asn:        AS16509"));
        assert!(plain.contains("prefix:     54.0.0.0/8"));
        assert!(plain.contains("type:       cloud"));
        assert!(plain.contains("provider:   AWS"));
        assert!(plain.contains("service:    EC2"));
        assert!(plain.contains("region:     us-east-1"));
        assert!(plain.contains("datacenter: true"));
    }

    // --- user_agent ---

    #[test]
    fn user_agent_to_plain_none() {
        let ifc = make_ifconfig(None, None, residential_network());
        assert_eq!(user_agent::to_plain(&ifc), "unknown\n");
    }

    // --- all ---

    #[test]
    fn all_to_plain_minimal() {
        let ifc = make_ifconfig(None, None, residential_network());
        let plain = all::to_plain(&ifc);
        assert!(plain.contains("ip:         203.0.113.42"));
        assert!(plain.contains("version:    4"));
        assert!(!plain.contains("hostname:"));
        assert!(!plain.contains("port:"));
    }

    #[test]
    fn all_to_plain_full() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(8080), residential_network());
        let plain = all::to_plain(&ifc);
        assert!(plain.contains("hostname:   dns.example.com"), "hostname missing: {plain}");
        assert!(plain.contains("port:       8080"));
        assert!(plain.contains("org:        Example Telecom"));
        assert!(plain.contains("asn:        AS64496"));
        assert!(plain.contains("network:    residential"));
    }

    // --- headers ---

    #[test]
    fn headers_to_plain() {
        let h = vec![
            ("host".to_string(), "example.com".to_string()),
            ("accept".to_string(), "*/*".to_string()),
        ];
        let plain = headers::to_plain(&h);
        assert_eq!(plain, "host: example.com\naccept: */*\n");
    }

    #[test]
    fn headers_to_json_value() {
        let h = vec![
            ("host".to_string(), "example.com".to_string()),
        ];
        let val = headers::to_json_value(&h);
        assert_eq!(val["host"], "example.com");
    }
}
