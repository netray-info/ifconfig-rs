extern crate ifconfig_rs;
extern crate rocket;

use rocket::http::{Accept, Status};
use rocket::local::blocking::Client;

#[test]
fn rate_limit_returns_429_when_exceeded() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");

    // Default limit is 60 requests per 60-second window
    for i in 0..60 {
        let response = client
            .get("/")
            .remote("10.0.0.1:8000".parse().unwrap())
            .header(Accept::Plain)
            .dispatch();
        assert_eq!(response.status(), Status::Ok, "request {} should succeed", i + 1);
    }

    // 61st request should be rate limited
    let response = client
        .get("/")
        .remote("10.0.0.1:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::TooManyRequests);
    assert_eq!(response.into_string(), Some("rate limit exceeded\n".into()));
}

#[test]
fn rate_limit_separate_ips_are_independent() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");

    // Exhaust limit for 10.0.0.1
    for _ in 0..60 {
        let response = client
            .get("/")
            .remote("10.0.0.1:8000".parse().unwrap())
            .header(Accept::Plain)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
    let response = client
        .get("/")
        .remote("10.0.0.1:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::TooManyRequests);

    // 10.0.0.2 should still be allowed
    let response = client
        .get("/")
        .remote("10.0.0.2:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
}

#[test]
fn health_endpoint_not_rate_limited() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");

    // Exhaust the rate limit
    for _ in 0..60 {
        let response = client
            .get("/")
            .remote("10.0.0.3:8000".parse().unwrap())
            .header(Accept::Plain)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }

    // Verify rate limit is active
    let response = client
        .get("/")
        .remote("10.0.0.3:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::TooManyRequests);

    // /health should still work since it has no RateLimited guard
    let response = client
        .get("/health")
        .remote("10.0.0.3:8000".parse().unwrap())
        .dispatch();
    assert_ne!(response.status(), Status::TooManyRequests);
}
