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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ifconfig(host: Option<&str>, tcp_port: Option<u16>, network: Option<Network>) -> Ifconfig {
        Ifconfig {
            host: host.map(|n| Host { name: n.to_string() }),
            ip: Ip { addr: "203.0.113.42".to_string(), version: "4".to_string() },
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
            },
            isp: Isp { name: Some("Example Telecom".to_string()), asn: Some(64496) },
            network,
            user_agent: None,
            user_agent_header: None,
        }
    }

    fn residential_network() -> Network {
        Network {
            network_type: "residential".to_string(),
            provider: None,
            service: None,
            region: None,
            is_datacenter: false,
            is_vpn: false,
            is_tor: false,
            is_proxy: false,
            is_bot: false,
            is_threat: false,
        }
    }

    // --- root ---

    #[test]
    fn root_to_plain_returns_ip() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(12345), Some(residential_network()));
        assert_eq!(root::to_plain(&ifc), "203.0.113.42\n");
    }

    #[test]
    fn root_to_json_has_all_fields() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(12345), Some(residential_network()));
        let val = root::to_json(&ifc).unwrap();
        assert!(val["ip"]["addr"].is_string());
        assert!(val["host"]["name"].is_string());
        assert!(val["location"]["city"].is_string());
    }

    // --- tcp ---

    #[test]
    fn tcp_to_plain_with_port() {
        let ifc = make_ifconfig(None, Some(54321), None);
        assert_eq!(tcp::to_plain(&ifc), "54321\n");
    }

    #[test]
    fn tcp_to_plain_none() {
        let ifc = make_ifconfig(None, None, None);
        assert_eq!(tcp::to_plain(&ifc), "\n");
    }

    #[test]
    fn tcp_to_json_none() {
        let ifc = make_ifconfig(None, None, None);
        assert!(tcp::to_json(&ifc).is_none());
    }

    #[test]
    fn tcp_to_json_some() {
        let ifc = make_ifconfig(None, Some(8080), None);
        let val = tcp::to_json(&ifc).unwrap();
        assert_eq!(val["port"], 8080);
    }

    // --- host ---

    #[test]
    fn host_to_plain_some() {
        let ifc = make_ifconfig(Some("dns.example.com"), None, None);
        assert_eq!(host::to_plain(&ifc), "dns.example.com\n");
    }

    #[test]
    fn host_to_plain_none() {
        let ifc = make_ifconfig(None, None, None);
        assert_eq!(host::to_plain(&ifc), "unknown\n");
    }

    #[test]
    fn host_to_json_none() {
        let ifc = make_ifconfig(None, None, None);
        let val = host::to_json(&ifc).unwrap();
        assert!(val.is_null());
    }

    // --- isp ---

    #[test]
    fn isp_to_plain_with_asn() {
        let ifc = make_ifconfig(None, None, None);
        assert_eq!(isp::to_plain(&ifc), "Example Telecom (AS64496)\n");
    }

    #[test]
    fn isp_to_plain_no_asn() {
        let mut ifc = make_ifconfig(None, None, None);
        ifc.isp.asn = None;
        assert_eq!(isp::to_plain(&ifc), "Example Telecom\n");
    }

    #[test]
    fn isp_to_plain_unknown() {
        let mut ifc = make_ifconfig(None, None, None);
        ifc.isp = Isp::unknown();
        assert_eq!(isp::to_plain(&ifc), "unknown\n");
    }

    // --- location ---

    #[test]
    fn location_to_plain_full() {
        let ifc = make_ifconfig(None, None, None);
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("Berlin"));
        assert!(plain.contains("Germany"));
        assert!(plain.contains("DE"));
        assert!(plain.contains("Europe"));
        assert!(plain.contains("~100km"));
    }

    #[test]
    fn location_to_plain_no_region() {
        let mut ifc = make_ifconfig(None, None, None);
        ifc.location.region = None;
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("Berlin, Germany (DE)"));
    }

    #[test]
    fn location_to_plain_no_accuracy() {
        let mut ifc = make_ifconfig(None, None, None);
        ifc.location.accuracy_radius_km = None;
        let plain = location::to_plain(&ifc);
        assert!(!plain.contains("~"));
        assert!(plain.ends_with("Europe/Berlin\n"));
    }

    #[test]
    fn location_to_plain_unknown() {
        let mut ifc = make_ifconfig(None, None, None);
        ifc.location = Location::unknown();
        let plain = location::to_plain(&ifc);
        assert!(plain.contains("unknown"));
    }

    // --- network ---

    #[test]
    fn network_to_plain_residential() {
        let ifc = make_ifconfig(None, None, Some(residential_network()));
        let plain = network::to_plain(&ifc);
        assert!(plain.contains("type:       residential"));
        assert!(plain.contains("datacenter: false"));
        assert!(!plain.contains("provider:"));
    }

    #[test]
    fn network_to_plain_cloud_with_provider() {
        let n = Network {
            network_type: "cloud".to_string(),
            provider: Some("AWS".to_string()),
            service: Some("EC2".to_string()),
            region: Some("us-east-1".to_string()),
            is_datacenter: true,
            is_vpn: false,
            is_tor: false,
            is_proxy: false,
            is_bot: false,
            is_threat: false,
        };
        let ifc = make_ifconfig(None, None, Some(n));
        let plain = network::to_plain(&ifc);
        assert!(plain.contains("type:       cloud"));
        assert!(plain.contains("provider:   AWS"));
        assert!(plain.contains("service:    EC2"));
        assert!(plain.contains("region:     us-east-1"));
        assert!(plain.contains("datacenter: true"));
    }

    #[test]
    fn network_to_plain_none() {
        let ifc = make_ifconfig(None, None, None);
        assert_eq!(network::to_plain(&ifc), "unknown\n");
    }

    #[test]
    fn network_to_json_none() {
        let ifc = make_ifconfig(None, None, None);
        let val = network::to_json(&ifc).unwrap();
        assert!(val.is_null());
    }

    // --- user_agent ---

    #[test]
    fn user_agent_to_plain_none() {
        let ifc = make_ifconfig(None, None, None);
        assert_eq!(user_agent::to_plain(&ifc), "unknown\n");
    }

    // --- all ---

    #[test]
    fn all_to_plain_minimal() {
        let ifc = make_ifconfig(None, None, None);
        let plain = all::to_plain(&ifc);
        assert!(plain.contains("ip:         203.0.113.42"));
        assert!(plain.contains("version:    4"));
        assert!(!plain.contains("hostname:"));
        assert!(!plain.contains("port:"));
    }

    #[test]
    fn all_to_plain_full() {
        let ifc = make_ifconfig(Some("dns.example.com"), Some(8080), Some(residential_network()));
        let plain = all::to_plain(&ifc);
        assert!(plain.contains("hostname:   dns.example.com"));
        assert!(plain.contains("port:       8080"));
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
