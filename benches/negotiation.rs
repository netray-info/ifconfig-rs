use criterion::{Criterion, black_box, criterion_group, criterion_main};

use axum::http::{HeaderMap, HeaderValue};
use ifconfig_rs::negotiate::negotiate;

fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        map.insert(
            axum::http::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
            HeaderValue::from_str(v).unwrap(),
        );
    }
    map
}

fn bench_negotiate(c: &mut Criterion) {
    let empty = HeaderMap::new();
    let curl_headers = headers_with(&[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
    let json_headers = headers_with(&[("accept", "application/json")]);
    let browser_headers = headers_with(&[
        ("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)"),
        (
            "accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
    ]);

    c.bench_function("negotiate_suffix_json", |b| {
        b.iter(|| negotiate(black_box(Some("json")), black_box(&empty)))
    });

    c.bench_function("negotiate_cli_curl", |b| {
        b.iter(|| negotiate(black_box(None), black_box(&curl_headers)))
    });

    c.bench_function("negotiate_accept_json", |b| {
        b.iter(|| negotiate(black_box(None), black_box(&json_headers)))
    });

    c.bench_function("negotiate_browser_html", |b| {
        b.iter(|| negotiate(black_box(None), black_box(&browser_headers)))
    });

    c.bench_function("negotiate_default_no_headers", |b| {
        b.iter(|| negotiate(black_box(None), black_box(&empty)))
    });
}

criterion_group!(benches, bench_negotiate);
criterion_main!(benches);
