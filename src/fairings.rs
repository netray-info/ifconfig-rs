use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Data, Request, Response};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Default)]
pub struct XForwardedFor;

#[rocket::async_trait]
impl Fairing for XForwardedFor {
    fn info(&self) -> Info {
        Info {
            name: "Set the request remote from left most IP in X-Forwarded-For",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let new_remote = request
            .headers()
            .get_one("X-Forwarded-For")
            .and_then(|xfr| xfr.split(',').next().map(str::trim))
            .and_then(|ip_str| IpAddr::from_str(ip_str).ok())
            .and_then(|ip| request.remote().map(|r| SocketAddr::new(ip, r.port())));
        if let Some(remote) = new_remote {
            request.set_remote(remote);
        }
    }
}

pub struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Add security response headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("X-Frame-Options", "DENY"));
        res.set_header(Header::new("Referrer-Policy", "strict-origin-when-cross-origin"));
    }
}
