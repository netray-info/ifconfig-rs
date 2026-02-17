extern crate ifconfig_rs;
extern crate rocket;
extern crate serde_json;

use ifconfig_rs::backend::Ifconfig;

use serde_json::json;

use rocket::http::hyper::header::USER_AGENT;
use rocket::http::{Accept, ContentType, Header, Status};
use rocket::local::blocking::Client;

#[test]
fn cors_header_present() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.headers().get_one("Access-Control-Allow-Origin"), Some("*"));
}

#[test]
fn handle_root_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_root_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_root_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let expected_json = json!({
        "host": {
            "name": "192.168.0.101"
        },
        "ip": {
            "addr": "192.168.0.101",
            "version": "4"
        },
        "isp": null,
        "location": null,
        "tcp": {
            "port": 8000
        },
        "user_agent": null,
        "user_agent_header": "Some browser that will ultimately win the war."
    });
    let expected_str = expected_json.to_string();
    let expected: Ifconfig = serde_json::from_str(&expected_str).unwrap();

    let body = response.into_string().unwrap();
    let answer: Ifconfig = serde_json::from_str(&body).unwrap();

    assert_eq!(answer.ip, expected.ip);
}

#[test]
fn handle_root_html() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::HTML)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::HTML));
    assert!(response.into_string().unwrap().contains("html"));
}

#[test]
fn handle_root_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));

    let expected_json = json!({
        "host": {
            "name": "192.168.0.101"
        },
        "ip": {
            "addr": "192.168.0.101",
            "version": "4"
        },
        "isp": null,
        "location": null,
        "tcp": {
            "port": 8000
        },
        "user_agent": null,
        "user_agent_header": "Some browser that will ultimately win the war."
    });
    let expected_str = expected_json.to_string();
    let expected: Ifconfig = serde_json::from_str(&expected_str).unwrap();

    let body = response.into_string().unwrap();
    let answer: Ifconfig = serde_json::from_str(&body).unwrap();

    assert_eq!(answer.ip, expected.ip);
}

#[test]
fn handle_ip_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_ip_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_ip_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"addr":"192.168.0.101","version":"4"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_ip_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"addr":"192.168.0.101","version":"4"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_tcp_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/tcp")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("8000\n".into()));
}

#[test]
fn handle_tcp_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/tcp")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("8000\n".into()));
}

#[test]
fn handle_tcp_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/tcp")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"port":8000}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_tcp_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/tcp/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"port":8000}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_host_plain_cli_curl() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("dns.google\n".into()));
}

#[test]
fn handle_host_plain_cli_httpie() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "HTTPie/0.9.9"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("dns.google\n".into()));
}

#[test]
fn handle_host_plain_cli_wget() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "Wget/1.19.5 (darwin17.5.0)"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("dns.google\n".into()));
}

#[test]
fn handle_host_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("dns.google\n".into()));
}

#[test]
fn handle_host_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"name":"dns.google"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_host_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/host/json")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"name":"dns.google"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_isp_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/isp")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("Google LLC (AS15169)\n".into()));
}

#[test]
fn handle_isp_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/isp")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("Google LLC (AS15169)\n".into()));
}

#[test]
fn handle_isp_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/isp")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"asn":15169,"name":"Google LLC"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_isp_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/isp/json")
        .remote("8.8.8.8:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"asn":15169,"name":"Google LLC"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_location_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/location")
        .remote("81.2.69.142:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(
        response.into_string(),
        Some("Kettering, United Kingdom (GB), Europe, Europe/London\n".into())
    );
}

#[test]
fn handle_location_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/location")
        .remote("81.2.69.142:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(
        response.into_string(),
        Some("Kettering, United Kingdom (GB), Europe, Europe/London\n".into())
    );
}

#[test]
fn handle_location_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/location")
        .remote("81.2.69.142:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"city":"Kettering","continent":"Europe","continent_code":"EU","country":"United Kingdom","country_iso":"GB","latitude":52.3966,"longitude":-0.7212,"timezone":"Europe/London"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_location_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/location/json")
        .remote("81.2.69.142:8000".parse().unwrap())
        .header(Header::new(
            USER_AGENT.as_str(),
            "Some browser that will ultimately win the war.",
        ))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"city":"Kettering","continent":"Europe","continent_code":"EU","country":"United Kingdom","country_iso":"GB","latitude":52.3966,"longitude":-0.7212,"timezone":"Europe/London"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_user_agent_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/user_agent")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "Wget/1.19.5 (darwin17.5.0)"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("Wget, 1.19.5, Other, \n".into()));
}

#[test]
fn handle_user_agent_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/user_agent")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .header(Header::new(USER_AGENT.as_str(),"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(
        response.into_string(),
        Some("Safari, 10.1.2, Mac OS X, 10.12.6\n".into())
    );
}

#[test]
fn handle_user_agent_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/user_agent")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(USER_AGENT.as_str(),"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"browser":{"family":"Safari","major":"10","minor":"1","patch":"2","version":"10.1.2"},"device":{"brand":"Apple","family":"Mac","model":"Mac"},"os":{"family":"Mac OS X","major":"10","minor":"12","patch":"6","patch_minor":null,"version":"10.12.6"}}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_user_agent_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/user_agent/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(USER_AGENT.as_str(),"Mozilla/5.0 (Macintosh; Intel Mac OS X 10_12_6) AppleWebKit/603.3.8 (KHTML, like Gecko) Version/10.1.2 Safari/603.3.8"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"browser":{"family":"Safari","major":"10","minor":"1","patch":"2","version":"10.1.2"},"device":{"brand":"Apple","family":"Mac","model":"Mac"},"os":{"family":"Mac OS X","major":"10","minor":"12","patch":"6","patch_minor":null,"version":"10.12.6"}}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_headers_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/headers")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .header(Header::new("X-Custom-Test", "hello"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    let body = response.into_string().unwrap();
    assert!(body.contains("user-agent: curl/7.54.0"));
    assert!(body.contains("X-Custom-Test: hello"));
}

#[test]
fn handle_headers_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/headers")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .header(Header::new("X-Custom-Test", "hello"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let body = response.into_string().unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["user-agent"], "curl/7.54.0");
    assert_eq!(json["X-Custom-Test"], "hello");
}

#[test]
fn handle_headers_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/headers")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .header(Header::new("X-Forwarded-For", "10.0.0.1"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    let body = response.into_string().unwrap();
    assert!(body.contains("X-Forwarded-For: 10.0.0.1"));
}

#[test]
fn handle_headers_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/headers/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .header(Header::new("X-Custom-Test", "world"))
        .dispatch();
    eprintln!("{:?}", response);
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let body = response.into_string().unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["X-Custom-Test"], "world");
}

#[test]
fn handle_ipv4_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv4")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_ipv4_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv4")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_string(), Some("192.168.0.101\n".into()));
}

#[test]
fn handle_ipv4_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv4")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"addr":"192.168.0.101","version":"4"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_ipv4_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv4/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"addr":"192.168.0.101","version":"4"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_ipv4_returns_404_for_ipv6() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv4")
        .remote("[::1]:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::NotFound);
}

#[test]
fn handle_ipv6_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv6")
        .remote("[::1]:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    assert_eq!(response.into_string(), Some("::1\n".into()));
}

#[test]
fn handle_ipv6_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv6")
        .remote("[::1]:8000".parse().unwrap())
        .header(Accept::JSON)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let expected = r#"{"addr":"::1","version":"6"}"#;
    assert_eq!(response.into_string(), Some(expected.into()));
}

#[test]
fn handle_ipv6_returns_404_for_ipv4() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ipv6")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::NotFound);
}

#[test]
fn handle_all_plain_cli() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/all")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Any)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    let body = response.into_string().unwrap();
    assert!(body.contains("ip:         192.168.0.101"));
    assert!(body.contains("version:    4"));
    assert!(body.contains("port:       8000"));
}

#[test]
fn handle_all_plain() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/all")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::Plain));
    let body = response.into_string().unwrap();
    assert!(body.contains("ip:         192.168.0.101"));
    assert!(body.contains("version:    4"));
}

#[test]
fn handle_all_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/all")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let body = response.into_string().unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"]["addr"], "192.168.0.101");
    assert_eq!(json["ip"]["version"], "4");
}

#[test]
fn handle_all_json_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/all/json")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Header::new(USER_AGENT.as_str(), "curl/7.54.0"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.content_type(), Some(ContentType::JSON));
    let body = response.into_string().unwrap();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["ip"]["addr"], "192.168.0.101");
}

#[test]
fn cache_control_plain_text() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::Plain)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("private, max-age=60")
    );
    assert_eq!(
        response.headers().get_one("Vary"),
        Some("Accept, User-Agent")
    );
}

#[test]
fn cache_control_json() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/ip")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::JSON)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("private, max-age=60")
    );
}

#[test]
fn cache_control_html() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client
        .get("/")
        .remote("192.168.0.101:8000".parse().unwrap())
        .header(Accept::HTML)
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-cache")
    );
}

#[test]
fn cache_control_health() {
    let client = Client::tracked(ifconfig_rs::rocket()).expect("valid rocket instance");
    let response = client.get("/health").dispatch();
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-cache")
    );
}
