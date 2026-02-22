use criterion::{black_box, criterion_group, criterion_main, Criterion};

use ifconfig_rs::backend::asn_heuristic::{classify_asn, AsnPatterns};

fn bench_classify_asn(c: &mut Criterion) {
    let patterns = AsnPatterns::builtin();

    c.bench_function("classify_asn_hit_hetzner", |b| {
        b.iter(|| classify_asn(black_box(None), Some(black_box("Hetzner Online GmbH")), &patterns))
    });

    c.bench_function("classify_asn_hit_mullvad", |b| {
        b.iter(|| classify_asn(black_box(None), Some(black_box("31173 Services AB")), &patterns))
    });

    c.bench_function("classify_asn_hit_generic_hosting", |b| {
        b.iter(|| classify_asn(black_box(None), Some(black_box("Example Hosting Ltd")), &patterns))
    });

    c.bench_function("classify_asn_miss", |b| {
        b.iter(|| classify_asn(black_box(None), Some(black_box("Deutsche Telekom AG")), &patterns))
    });

    c.bench_function("classify_asn_miss_long_name", |b| {
        b.iter(|| {
            classify_asn(
                black_box(None),
                Some(black_box(
                    "Very Long ISP Name That Doesn't Match Any Pattern International Telecommunications Group",
                )),
                &patterns,
            )
        })
    });

    c.bench_function("classify_asn_by_number_hit", |b| {
        b.iter(|| classify_asn(black_box(Some(13335u32)), None, &patterns))
    });

    c.bench_function("classify_asn_by_number_miss", |b| {
        b.iter(|| classify_asn(black_box(Some(64496u32)), None, &patterns))
    });
}

criterion_group!(benches, bench_classify_asn);
criterion_main!(benches);
