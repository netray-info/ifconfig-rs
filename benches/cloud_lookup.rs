use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ifconfig_rs::backend::cloud_provider::CloudProviderDb;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn make_bench_db() -> CloudProviderDb {
    // Build a small but realistic CIDR table with ~20 entries.
    let entries = [
        r#"{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}"#,
        r#"{"cidr":"3.5.0.0/19","provider":"aws","service":"S3","region":"us-east-1"}"#,
        r#"{"cidr":"13.34.0.0/16","provider":"aws","service":"AMAZON","region":"ap-southeast-1"}"#,
        r#"{"cidr":"52.0.0.0/11","provider":"aws","service":"EC2","region":"us-east-1"}"#,
        r#"{"cidr":"35.190.0.0/17","provider":"gcp","service":"Google Cloud","region":"us-central1"}"#,
        r#"{"cidr":"35.200.0.0/14","provider":"gcp","service":"Google Cloud","region":"asia-south1"}"#,
        r#"{"cidr":"104.16.0.0/13","provider":"cloudflare","service":null,"region":null}"#,
        r#"{"cidr":"172.64.0.0/13","provider":"cloudflare","service":null,"region":null}"#,
        r#"{"cidr":"20.33.0.0/16","provider":"azure","service":"AzureCloud","region":"global"}"#,
        r#"{"cidr":"40.74.0.0/15","provider":"azure","service":"AzureCloud","region":"eastus"}"#,
        r#"{"cidr":"13.64.0.0/11","provider":"azure","service":"AzureCloud","region":"westus"}"#,
        r#"{"cidr":"2600:1f00::/24","provider":"aws","service":"EC2","region":"us-east-1"}"#,
        r#"{"cidr":"2a06:98c0::/29","provider":"cloudflare","service":null,"region":null}"#,
    ];
    let jsonl = entries.join("\n");
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("ifconfig_bench_cloud_{}.jsonl", id));
    std::fs::write(&path, &jsonl).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let db = rt.block_on(CloudProviderDb::from_file(path.to_str().unwrap())).unwrap();
    let _ = std::fs::remove_file(&path);
    db
}

fn bench_cloud_lookup(c: &mut Criterion) {
    let db = make_bench_db();

    c.bench_function("cloud_lookup_hit_aws", |b| {
        b.iter(|| db.lookup(black_box("3.2.34.1".parse().unwrap())))
    });

    c.bench_function("cloud_lookup_hit_cloudflare", |b| {
        b.iter(|| db.lookup(black_box("104.17.0.1".parse().unwrap())))
    });

    c.bench_function("cloud_lookup_hit_gcp", |b| {
        b.iter(|| db.lookup(black_box("35.190.1.1".parse().unwrap())))
    });

    c.bench_function("cloud_lookup_miss", |b| {
        b.iter(|| db.lookup(black_box("203.0.113.42".parse().unwrap())))
    });

    c.bench_function("cloud_lookup_hit_ipv6", |b| {
        b.iter(|| db.lookup(black_box("2600:1f00::1".parse().unwrap())))
    });
}

fn bench_cloud_construction(c: &mut Criterion) {
    let entries = [
        r#"{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}"#,
        r#"{"cidr":"52.0.0.0/11","provider":"aws","service":"EC2","region":"us-east-1"}"#,
        r#"{"cidr":"35.190.0.0/17","provider":"gcp","service":"Google Cloud","region":"us-central1"}"#,
        r#"{"cidr":"104.16.0.0/13","provider":"cloudflare","service":null,"region":null}"#,
        r#"{"cidr":"20.33.0.0/16","provider":"azure","service":"AzureCloud","region":"global"}"#,
    ];
    let jsonl = entries.join("\n");
    let path = std::env::temp_dir().join("ifconfig_bench_cloud_construct.jsonl");
    std::fs::write(&path, &jsonl).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    c.bench_function("cloud_db_construct_5_entries", |b| {
        b.iter(|| rt.block_on(CloudProviderDb::from_file(black_box(path.to_str().unwrap()))))
    });

    let _ = std::fs::remove_file(&path);
}

criterion_group!(benches, bench_cloud_lookup, bench_cloud_construction);
criterion_main!(benches);
