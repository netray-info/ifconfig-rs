use crate::backend::asn_heuristic::AsnPatterns;
use crate::backend::asn_info::AsnInfo;
use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use mhost::resolver::ResolverGroup;
use std::net::SocketAddr;

pub(crate) static UNKNOWN_STR: &str = "unknown";

#[allow(clippy::too_many_arguments)]
pub(crate) async fn make_ifconfig(
    remote: &SocketAddr,
    user_agent: &Option<&str>,
    user_agent_parser: Option<&UserAgentParser>,
    geoip_city_db: Option<&GeoIpCityDb>,
    geoip_asn_db: Option<&GeoIpAsnDb>,
    tor_exit_nodes: &TorExitNodes,
    feodo_botnet_ips: Option<&FeodoBotnetIps>,
    cins_army_ips: Option<&CinsArmyIps>,
    vpn_ranges: Option<&VpnRanges>,
    cloud_provider_db: Option<&CloudProviderDb>,
    datacenter_ranges: Option<&DatacenterRanges>,
    bot_db: Option<&BotDb>,
    spamhaus_drop: Option<&SpamhausDrop>,
    dns_resolver: &ResolverGroup,
    dns_cache: &DnsCache,
    skip_dns: bool,
    asn_patterns: &AsnPatterns,
    asn_info: Option<&AsnInfo>,
) -> Ifconfig {
    make_ifconfig_lang(
        remote,
        user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
        feodo_botnet_ips,
        cins_army_ips,
        vpn_ranges,
        cloud_provider_db,
        datacenter_ranges,
        bot_db,
        spamhaus_drop,
        dns_resolver,
        dns_cache,
        skip_dns,
        asn_patterns,
        asn_info,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn make_ifconfig_lang(
    remote: &SocketAddr,
    user_agent: &Option<&str>,
    user_agent_parser: Option<&UserAgentParser>,
    geoip_city_db: Option<&GeoIpCityDb>,
    geoip_asn_db: Option<&GeoIpAsnDb>,
    tor_exit_nodes: &TorExitNodes,
    feodo_botnet_ips: Option<&FeodoBotnetIps>,
    cins_army_ips: Option<&CinsArmyIps>,
    vpn_ranges: Option<&VpnRanges>,
    cloud_provider_db: Option<&CloudProviderDb>,
    datacenter_ranges: Option<&DatacenterRanges>,
    bot_db: Option<&BotDb>,
    spamhaus_drop: Option<&SpamhausDrop>,
    dns_resolver: &ResolverGroup,
    dns_cache: &DnsCache,
    skip_dns: bool,
    asn_patterns: &AsnPatterns,
    asn_info: Option<&AsnInfo>,
    lang: Option<String>,
) -> Ifconfig {
    let param = IfconfigParam {
        remote,
        user_agent_header: user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
        feodo_botnet_ips,
        cins_army_ips,
        vpn_ranges,
        cloud_provider_db,
        datacenter_ranges,
        bot_db,
        spamhaus_drop,
        asn_patterns,
        asn_info,
        dns_resolver,
        dns_cache,
        skip_dns,
        lang,
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
        if let Some(asn) = n.asn {
            let asn_str = if let Some(ref prefix) = n.prefix {
                format!("AS{} · {}", asn, prefix)
            } else {
                format!("AS{}", asn)
            };
            lines.push(format!("asn:        {}", asn_str));
        }
        if let Some(ref org) = n.org {
            lines.push(format!("org:        {}", org));
        }
        if let Some(ref cat) = n.asn_category {
            lines.push(format!("category:   {}", cat));
        }
        if let Some(ref role) = n.network_role {
            lines.push(format!("role:       {}", role));
        }
        if let Some(ref reg) = n.asn_registered {
            lines.push(format!("registered: {}", reg));
        }
        lines.push(format!("type:       {}", n.network_type));
        lines.push(format!("infra:      {}", n.infra_type));
        if let Some(ref c) = n.cloud {
            let parts: Vec<&str> = [Some(c.provider.as_str()), c.service.as_deref(), c.region.as_deref()]
                .iter()
                .filter_map(|x| *x)
                .collect();
            lines.push(format!("cloud:      {}", parts.join(" · ")));
        }
        if let Some(ref v) = n.vpn
            && let Some(ref p) = v.provider
        {
            lines.push(format!("vpn:        {}", p));
        }
        if let Some(ref b) = n.bot {
            lines.push(format!("bot:        {}", b.provider));
        }
        lines.push(format!("is_vpn:     {}", n.is_vpn));
        lines.push(format!("is_tor:     {}", n.is_tor));
        lines.push(format!("is_bot:     {}", n.is_bot));
        lines.push(format!("is_c2:      {}", n.is_c2));
        lines.push(format!("is_spamhaus: {}", n.is_spamhaus));
        lines.push(format!("is_datacenter: {}", n.is_datacenter));
        lines.push(format!("is_internal: {}", n.is_internal));
        lines.push(format!("is_anycast: {}", n.is_anycast));
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
            if let Some(asn) = n.asn {
                lines.push(format!("asn:        AS{}", asn));
            }
            if let Some(ref prefix) = n.prefix {
                lines.push(format!("prefix:     {}", prefix));
            }
            if let Some(ref org) = n.org {
                lines.push(format!("org:        {}", org));
            }
            if let Some(ref cat) = n.asn_category {
                lines.push(format!("category:   {}", cat));
            }
            if let Some(ref role) = n.network_role {
                lines.push(format!("role:       {}", role));
            }
            if let Some(ref reg) = n.asn_registered {
                lines.push(format!("registered: {}", reg));
            }
            lines.push(format!("network:    {}", n.network_type));
            lines.push(format!("infra:      {}", n.infra_type));
            if let Some(ref c) = n.cloud {
                let parts: Vec<&str> = [Some(c.provider.as_str()), c.service.as_deref(), c.region.as_deref()]
                    .iter()
                    .filter_map(|x| *x)
                    .collect();
                lines.push(format!("cloud:      {}", parts.join(" · ")));
            }
            if let Some(ref v) = n.vpn
                && let Some(ref p) = v.provider
            {
                lines.push(format!("vpn:        {}", p));
            }
            if let Some(ref b) = n.bot {
                lines.push(format!("bot:        {}", b.provider));
            }
            lines.push(format!("is_vpn:     {}", n.is_vpn));
            lines.push(format!("is_tor:     {}", n.is_tor));
            lines.push(format!("is_bot:     {}", n.is_bot));
            lines.push(format!("is_c2:      {}", n.is_c2));
            lines.push(format!("is_spamhaus: {}", n.is_spamhaus));
            lines.push(format!("is_datacenter: {}", n.is_datacenter));
            lines.push(format!("is_internal: {}", n.is_internal));
            lines.push(format!("is_anycast: {}", n.is_anycast));
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

    pub fn to_json_value_with_xff(headers: &[(String, String)], xff_chain: &[String]) -> JsonValue {
        let map: BTreeMap<&str, &str> = headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let mut obj = serde_json::to_value(map).unwrap_or(JsonValue::Null);
        if let JsonValue::Object(ref mut m) = obj {
            m.insert(
                "x_forwarded_for_chain".to_string(),
                serde_json::to_value(xff_chain).unwrap_or(JsonValue::Array(vec![])),
            );
        }
        obj
    }

    pub fn formatted(format: &OutputFormat, headers: &[(String, String)]) -> Option<String> {
        let json_val = to_json_value(headers);
        format.serialize_body(&json_val)
    }

    pub fn formatted_with_xff(
        format: &OutputFormat,
        headers: &[(String, String)],
        xff_chain: &[String],
    ) -> Option<String> {
        let json_val = to_json_value_with_xff(headers, xff_chain);
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

pub mod host {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(serde_json::json!({ "hostname": ifconfig.ip.hostname })).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        format!("{}\n", ifconfig.ip.hostname.as_deref().unwrap_or(UNKNOWN_STR))
    }
}

pub mod isp {
    use super::*;

    pub fn to_json(ifconfig: &Ifconfig) -> Option<serde_json::Value> {
        serde_json::to_value(&ifconfig.network).ok()
    }

    pub fn to_plain(ifconfig: &Ifconfig) -> String {
        network::to_plain(ifconfig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ifconfig(hostname: Option<&str>, tcp_port: Option<u16>, network: Network) -> Ifconfig {
        Ifconfig {
            ip: Ip {
                addr: "203.0.113.42".to_string(),
                version: "4".to_string(),
                hostname: hostname.map(|s| s.to_string()),
            },
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
                city_localized: None,
                country_localized: None,
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
            asn_category: None,
            network_role: None,
            asn_registered: None,
            network_type: "residential".to_string(),
            infra_type: "residential".to_string(),
            is_internal: false,
            is_datacenter: false,
            is_vpn: false,
            is_tor: false,
            is_bot: false,
            is_c2: false,
            is_spamhaus: false,
            cloud: None,
            vpn: None,
            bot: None,
            is_anycast: false,
            is_cins: false,
            iana_label: None,
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
        assert!(plain.contains("AS64496"));
        assert!(plain.contains("type:       residential"));
        assert!(plain.contains("infra:      residential"));
        assert!(plain.contains("is_datacenter: false"));
        assert!(!plain.contains("\ncloud:"));
    }

    #[test]
    fn network_to_plain_cloud_with_provider() {
        let n = Network {
            asn: Some(16509),
            org: Some("Amazon.com".to_string()),
            prefix: Some("54.0.0.0/8".to_string()),
            asn_category: Some("hosting".to_string()),
            network_role: None,
            asn_registered: None,
            network_type: "cloud".to_string(),
            infra_type: "cloud".to_string(),
            is_internal: false,
            is_datacenter: true,
            is_vpn: false,
            is_tor: false,
            is_bot: false,
            is_c2: false,
            is_spamhaus: false,
            cloud: Some(CloudInfo {
                provider: "aws".to_string(),
                service: Some("EC2".to_string()),
                region: Some("us-east-1".to_string()),
            }),
            vpn: None,
            bot: None,
            is_anycast: false,
            is_cins: false,
            iana_label: None,
        };
        let ifc = make_ifconfig(None, None, n);
        let plain = network::to_plain(&ifc);
        assert!(plain.contains("org:        Amazon.com"));
        assert!(plain.contains("AS16509"));
        assert!(plain.contains("type:       cloud"));
        assert!(plain.contains("infra:      cloud"));
        assert!(plain.contains("cloud:      aws · EC2 · us-east-1"));
        assert!(plain.contains("is_datacenter: true"));
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
        assert!(
            plain.contains("hostname:   dns.example.com"),
            "hostname missing: {plain}"
        );
        assert!(plain.contains("port:       8080"));
        assert!(plain.contains("org:        Example Telecom"));
        assert!(plain.contains("asn:        AS64496"));
        assert!(
            plain.contains("network:    residential"),
            "network type missing: {plain}"
        );
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
        let h = vec![("host".to_string(), "example.com".to_string())];
        let val = headers::to_json_value(&h);
        assert_eq!(val["host"], "example.com");
    }
}
