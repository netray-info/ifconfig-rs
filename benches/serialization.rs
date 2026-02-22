use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ifconfig_rs::format::OutputFormat;
use serde_json::json;

fn representative_ifconfig() -> serde_json::Value {
    json!({
        "ip": {"addr": "203.0.113.42", "version": "4", "hostname": "dns.example.com"},
        "tcp": {"port": 54321},
        "location": {
            "city": "Berlin",
            "region": "Berlin",
            "region_code": "BE",
            "country": "Germany",
            "country_iso": "DE",
            "postal_code": "10115",
            "is_eu": true,
            "latitude": 52.5200,
            "longitude": 13.4050,
            "timezone": "Europe/Berlin",
            "continent": "Europe",
            "continent_code": "EU",
            "accuracy_radius_km": 100
        },
        "network": {
            "type": "residential",
            "asn": 64496,
            "org": "Example Telecom AG",
            "prefix": "203.0.113.0/24",
            "provider": null,
            "service": null,
            "region": null,
            "is_datacenter": false,
            "is_vpn": false,
            "is_tor": false,
            "is_proxy": false,
            "is_bot": false,
            "is_threat": false
        },
        "user_agent": {
            "raw": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            "device": {"family": "Other", "brand": null, "model": null},
            "os": {"family": "Mac OS X", "major": "14", "minor": "0", "patch": null, "patch_minor": null, "version": "14.0"},
            "browser": {"family": "Chrome", "major": "120", "minor": "0", "patch": null, "version": "120.0"}
        }
    })
}

fn bench_serialization(c: &mut Criterion) {
    let val = representative_ifconfig();

    c.bench_function("serialize_json", |b| {
        b.iter(|| OutputFormat::Json.serialize_body(black_box(&val)))
    });

    c.bench_function("serialize_yaml", |b| {
        b.iter(|| OutputFormat::Yaml.serialize_body(black_box(&val)))
    });

    c.bench_function("serialize_toml", |b| {
        b.iter(|| OutputFormat::Toml.serialize_body(black_box(&val)))
    });

    c.bench_function("serialize_csv", |b| {
        b.iter(|| OutputFormat::Csv.serialize_body(black_box(&val)))
    });
}

criterion_group!(benches, bench_serialization);
criterion_main!(benches);
