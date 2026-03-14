// Snapshot tests for ifconfig-rs serialization output.
//
// These tests verify that serialization of the `Ifconfig` struct does not change
// unexpectedly. They are particularly valuable for CSV, which has no schema and
// is easy to accidentally break by adding/removing/reordering fields.
//
// First run: cargo test --test snapshots_test
// Snapshots are written to tests/snapshots/ with `.snap.new` extension.
// Accept them with: cargo insta review
//   or: INSTA_UPDATE=always cargo test --test snapshots_test

use ifconfig_rs::backend::{CloudInfo, Ifconfig, Ip, Location, Network, NetworkBot, Tcp, VpnInfo};
use ifconfig_rs::format::OutputFormat;

fn make_test_ifconfig() -> Ifconfig {
    Ifconfig {
        ip: Ip {
            addr: "8.8.8.8".to_string(),
            version: "4".to_string(),
            hostname: Some("dns.google".to_string()),
        },
        tcp: Some(Tcp { port: 54321 }),
        location: Location {
            city: Some("Mountain View".to_string()),
            region: Some("California".to_string()),
            region_code: Some("CA".to_string()),
            country: Some("United States".to_string()),
            country_iso: Some("US".to_string()),
            postal_code: Some("94043".to_string()),
            is_eu: Some(false),
            latitude: Some(37.386),
            longitude: Some(-122.0838),
            timezone: Some("America/Los_Angeles".to_string()),
            continent: Some("North America".to_string()),
            continent_code: Some("NA".to_string()),
            accuracy_radius_km: Some(1000),
            registered_country: Some("United States".to_string()),
            registered_country_iso: Some("US".to_string()),
        },
        network: Network {
            asn: Some(15169),
            org: Some("Google LLC".to_string()),
            prefix: Some("8.8.8.0/24".to_string()),
            asn_category: Some("hosting".to_string()),
            network_role: Some("tier1_transit".to_string()),
            asn_registered: Some("2000-03-30".to_string()),
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
                provider: "gcp".to_string(),
                service: Some("DNS".to_string()),
                region: Some("us-central1".to_string()),
            }),
            vpn: None,
            bot: None,
        },
        user_agent: None,
    }
}

fn make_test_ifconfig_with_vpn() -> Ifconfig {
    Ifconfig {
        ip: Ip {
            addr: "198.51.100.1".to_string(),
            version: "4".to_string(),
            hostname: None,
        },
        tcp: None,
        location: Location {
            city: Some("Stockholm".to_string()),
            region: Some("Stockholm County".to_string()),
            region_code: Some("AB".to_string()),
            country: Some("Sweden".to_string()),
            country_iso: Some("SE".to_string()),
            postal_code: None,
            is_eu: Some(true),
            latitude: Some(59.3293),
            longitude: Some(18.0686),
            timezone: Some("Europe/Stockholm".to_string()),
            continent: Some("Europe".to_string()),
            continent_code: Some("EU".to_string()),
            accuracy_radius_km: Some(500),
            registered_country: Some("Sweden".to_string()),
            registered_country_iso: Some("SE".to_string()),
        },
        network: Network {
            asn: Some(39351),
            org: Some("31173 Services AB".to_string()),
            prefix: Some("198.51.100.0/24".to_string()),
            asn_category: None,
            network_role: None,
            asn_registered: None,
            network_type: "vpn".to_string(),
            infra_type: "datacenter".to_string(),
            is_internal: false,
            is_datacenter: true,
            is_vpn: true,
            is_tor: false,
            is_bot: false,
            is_c2: false,
            is_spamhaus: false,
            cloud: None,
            vpn: Some(VpnInfo {
                provider: Some("Mullvad".to_string()),
            }),
            bot: None,
        },
        user_agent: None,
    }
}

fn make_test_ifconfig_with_bot() -> Ifconfig {
    Ifconfig {
        ip: Ip {
            addr: "66.249.64.1".to_string(),
            version: "4".to_string(),
            hostname: Some("crawl-66-249-64-1.googlebot.com".to_string()),
        },
        tcp: None,
        location: Location::unknown(),
        network: Network {
            asn: Some(15169),
            org: Some("Google LLC".to_string()),
            prefix: Some("66.249.64.0/19".to_string()),
            asn_category: Some("hosting".to_string()),
            network_role: None,
            asn_registered: Some("2000-03-30".to_string()),
            network_type: "bot".to_string(),
            infra_type: "cloud".to_string(),
            is_internal: false,
            is_datacenter: true,
            is_vpn: false,
            is_tor: false,
            is_bot: true,
            is_c2: false,
            is_spamhaus: false,
            cloud: None,
            vpn: None,
            bot: Some(NetworkBot {
                provider: "googlebot".to_string(),
            }),
        },
        user_agent: None,
    }
}

// ---------------------------------------------------------------------------
// JSON snapshots
// ---------------------------------------------------------------------------

#[test]
fn json_serialization_snapshot() {
    let response = make_test_ifconfig();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    insta::assert_json_snapshot!(value);
}

#[test]
fn json_serialization_vpn_snapshot() {
    let response = make_test_ifconfig_with_vpn();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    insta::assert_json_snapshot!(value);
}

#[test]
fn json_serialization_bot_snapshot() {
    let response = make_test_ifconfig_with_bot();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    insta::assert_json_snapshot!(value);
}

// ---------------------------------------------------------------------------
// YAML snapshots
// ---------------------------------------------------------------------------

#[test]
fn yaml_serialization_snapshot() {
    let response = make_test_ifconfig();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    let yaml = OutputFormat::Yaml
        .serialize_body(&value)
        .expect("YAML serialization must not fail");
    insta::assert_snapshot!(yaml);
}

// ---------------------------------------------------------------------------
// TOML snapshots
// ---------------------------------------------------------------------------

#[test]
fn toml_serialization_snapshot() {
    let response = make_test_ifconfig();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    let toml = OutputFormat::Toml
        .serialize_body(&value)
        .expect("TOML serialization must not fail");
    insta::assert_snapshot!(toml);
}

// ---------------------------------------------------------------------------
// CSV snapshots
// ---------------------------------------------------------------------------

#[test]
fn csv_serialization_snapshot() {
    let response = make_test_ifconfig();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    let csv = OutputFormat::Csv
        .serialize_body(&value)
        .expect("CSV serialization must not fail");
    insta::assert_snapshot!(csv);
}

#[test]
fn csv_serialization_vpn_snapshot() {
    let response = make_test_ifconfig_with_vpn();
    let value = serde_json::to_value(&response).expect("serialization must not fail");
    let csv = OutputFormat::Csv
        .serialize_body(&value)
        .expect("CSV serialization must not fail");
    insta::assert_snapshot!(csv);
}
