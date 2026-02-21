use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ifconfig_rs::backend::asn_heuristic::classify_asn;

fn bench_classify_asn(c: &mut Criterion) {
    c.bench_function("classify_asn_hit_hetzner", |b| {
        b.iter(|| classify_asn(black_box("Hetzner Online GmbH")))
    });

    c.bench_function("classify_asn_hit_mullvad", |b| {
        b.iter(|| classify_asn(black_box("31173 Services AB")))
    });

    c.bench_function("classify_asn_hit_generic_hosting", |b| {
        b.iter(|| classify_asn(black_box("Example Hosting Ltd")))
    });

    c.bench_function("classify_asn_miss", |b| {
        b.iter(|| classify_asn(black_box("Deutsche Telekom AG")))
    });

    c.bench_function("classify_asn_miss_long_name", |b| {
        b.iter(|| {
            classify_asn(black_box(
                "Very Long ISP Name That Doesn't Match Any Pattern International Telecommunications Group",
            ))
        })
    });
}

criterion_group!(benches, bench_classify_asn);
criterion_main!(benches);
