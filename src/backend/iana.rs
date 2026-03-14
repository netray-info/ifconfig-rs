use std::net::IpAddr;

struct V4Entry {
    prefix: std::net::Ipv4Addr,
    prefix_len: u8,
    label: &'static str,
}

struct V6Entry {
    prefix: std::net::Ipv6Addr,
    prefix_len: u8,
    label: &'static str,
}

static V4_TABLE: &[V4Entry] = &[
    V4Entry { prefix: std::net::Ipv4Addr::new(0, 0, 0, 0), prefix_len: 8, label: "This host on this network" },
    V4Entry { prefix: std::net::Ipv4Addr::new(100, 64, 0, 0), prefix_len: 10, label: "Shared Address Space" },
    V4Entry { prefix: std::net::Ipv4Addr::new(192, 0, 0, 0), prefix_len: 24, label: "IETF Protocol Assignments" },
    V4Entry { prefix: std::net::Ipv4Addr::new(192, 0, 2, 0), prefix_len: 24, label: "Documentation (TEST-NET-1)" },
    V4Entry { prefix: std::net::Ipv4Addr::new(198, 18, 0, 0), prefix_len: 15, label: "Benchmarking" },
    V4Entry { prefix: std::net::Ipv4Addr::new(198, 51, 100, 0), prefix_len: 24, label: "Documentation (TEST-NET-2)" },
    V4Entry { prefix: std::net::Ipv4Addr::new(203, 0, 113, 0), prefix_len: 24, label: "Documentation (TEST-NET-3)" },
    V4Entry { prefix: std::net::Ipv4Addr::new(240, 0, 0, 0), prefix_len: 4, label: "Reserved" },
];

static V6_TABLE: &[V6Entry] = &[
    V6Entry { prefix: std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1), prefix_len: 128, label: "Loopback" },
    V6Entry { prefix: std::net::Ipv6Addr::new(0x2001, 0x0db8, 0, 0, 0, 0, 0, 0), prefix_len: 32, label: "Documentation" },
    V6Entry { prefix: std::net::Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0), prefix_len: 7, label: "Unique Local" },
    V6Entry { prefix: std::net::Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), prefix_len: 10, label: "Link-Local" },
];

fn v4_matches(addr: std::net::Ipv4Addr, prefix: std::net::Ipv4Addr, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let shift = 32 - prefix_len as u32;
    u32::from(addr) >> shift == u32::from(prefix) >> shift
}

fn v6_matches(addr: std::net::Ipv6Addr, prefix: std::net::Ipv6Addr, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    let addr_bits = u128::from(addr);
    let prefix_bits = u128::from(prefix);
    let shift = 128 - prefix_len as u32;
    addr_bits >> shift == prefix_bits >> shift
}

pub fn lookup_iana_label(ip: IpAddr) -> Option<String> {
    match ip {
        IpAddr::V4(v4) => {
            for entry in V4_TABLE {
                if v4_matches(v4, entry.prefix, entry.prefix_len) {
                    return Some(entry.label.to_string());
                }
            }
            None
        }
        IpAddr::V6(v6) => {
            for entry in V6_TABLE {
                if v6_matches(v6, entry.prefix, entry.prefix_len) {
                    return Some(entry.label.to_string());
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgnat_shared_address_space() {
        assert_eq!(
            lookup_iana_label("100.64.0.1".parse().unwrap()),
            Some("Shared Address Space".to_string())
        );
        assert_eq!(
            lookup_iana_label("100.127.255.255".parse().unwrap()),
            Some("Shared Address Space".to_string())
        );
        assert!(lookup_iana_label("100.128.0.1".parse().unwrap()).is_none());
    }

    #[test]
    fn doc_prefixes() {
        assert_eq!(
            lookup_iana_label("192.0.2.1".parse().unwrap()),
            Some("Documentation (TEST-NET-1)".to_string())
        );
        assert_eq!(
            lookup_iana_label("198.51.100.1".parse().unwrap()),
            Some("Documentation (TEST-NET-2)".to_string())
        );
        assert_eq!(
            lookup_iana_label("203.0.113.1".parse().unwrap()),
            Some("Documentation (TEST-NET-3)".to_string())
        );
    }

    #[test]
    fn benchmarking() {
        assert_eq!(
            lookup_iana_label("198.18.0.1".parse().unwrap()),
            Some("Benchmarking".to_string())
        );
        assert_eq!(
            lookup_iana_label("198.19.255.255".parse().unwrap()),
            Some("Benchmarking".to_string())
        );
    }

    #[test]
    fn reserved() {
        assert_eq!(
            lookup_iana_label("240.0.0.1".parse().unwrap()),
            Some("Reserved".to_string())
        );
        assert_eq!(
            lookup_iana_label("255.255.255.255".parse().unwrap()),
            Some("Reserved".to_string())
        );
    }

    #[test]
    fn ietf_protocol_assignments() {
        assert_eq!(
            lookup_iana_label("192.0.0.1".parse().unwrap()),
            Some("IETF Protocol Assignments".to_string())
        );
    }

    #[test]
    fn this_host() {
        assert_eq!(
            lookup_iana_label("0.0.0.1".parse().unwrap()),
            Some("This host on this network".to_string())
        );
    }

    #[test]
    fn public_ip_no_label() {
        assert!(lookup_iana_label("8.8.8.8".parse().unwrap()).is_none());
        assert!(lookup_iana_label("1.1.1.1".parse().unwrap()).is_none());
    }

    #[test]
    fn ipv6_loopback() {
        assert_eq!(
            lookup_iana_label("::1".parse().unwrap()),
            Some("Loopback".to_string())
        );
    }

    #[test]
    fn ipv6_documentation() {
        assert_eq!(
            lookup_iana_label("2001:db8::1".parse().unwrap()),
            Some("Documentation".to_string())
        );
    }

    #[test]
    fn ipv6_unique_local() {
        assert_eq!(
            lookup_iana_label("fc00::1".parse().unwrap()),
            Some("Unique Local".to_string())
        );
        assert_eq!(
            lookup_iana_label("fd12::1".parse().unwrap()),
            Some("Unique Local".to_string())
        );
    }

    #[test]
    fn ipv6_link_local() {
        assert_eq!(
            lookup_iana_label("fe80::1".parse().unwrap()),
            Some("Link-Local".to_string())
        );
    }

    #[test]
    fn ipv6_public_no_label() {
        assert!(lookup_iana_label("2606:4700::1111".parse().unwrap()).is_none());
    }
}
