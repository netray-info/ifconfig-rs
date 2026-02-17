use regex::RegexSet;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{self, FromRequest};
use rocket::Request;
use std::net::SocketAddr;
use std::sync::LazyLock;

pub struct RequestHeaders {
    pub headers: Vec<(String, String)>,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for RequestHeaders {
    type Error = ();

    async fn from_request(req: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let headers = req
            .headers()
            .iter()
            .map(|h| (h.name().to_string(), h.value().to_string()))
            .collect();
        Outcome::Success(RequestHeaders { headers })
    }
}

pub struct RequesterInfo<'a> {
    pub remote: SocketAddr,
    pub user_agent: Option<&'a str>,
    pub uri: String,
}

#[rocket::async_trait]
impl<'a> FromRequest<'a> for RequesterInfo<'a> {
    type Error = ();

    async fn from_request(req: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let remote = if let Some(remote) = req.remote() {
            remote
        } else {
            return Outcome::Error((Status::InternalServerError, ()));
        };
        let user_agent = req.headers().get_one("User-Agent");

        let request_info = RequesterInfo {
            remote,
            user_agent,
            uri: req.uri().to_string(),
        };
        Outcome::Success(request_info)
    }
}

/// `CliClient` checks if a known CLI client sends a "plain" text request
///
/// At least curl, httpie, wget send "Accept: */*" by default. This makes it difficult to dispatch the request. This
/// request guard tries to guess, if a request is plain text request by a CLI client. The the heuristic goes like this:
/// 1. Is this as known CLI client, cf. `KNOWN_CLI_CLIENTS`?
/// 2. If yes, is the default Accept header set, i.e., */* set?
/// 3. If yes, then this is a plain text request by a CLI client
/// 4. In any other case, the request is forwarded to higher ranked routes.
pub struct CliClientRequest<'a> {
    pub user_agent_header: &'a str,
}

static KNOWN_CLI_CLIENTS: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new([r"curl", r"HTTPie", r"HTTP Library", r"Wget"]).unwrap());

#[rocket::async_trait]
impl<'a> FromRequest<'a> for CliClientRequest<'a> {
    type Error = ();

    async fn from_request(req: &'a Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user_agent_header = req.headers().get_one("User-Agent");
        let accept_header = req.headers().get_one("Accept");

        match (user_agent_header, accept_header) {
            (Some(uah), Some("*/*")) if KNOWN_CLI_CLIENTS.is_match(uah) => {
                Outcome::Success(CliClientRequest { user_agent_header: uah })
            }
            _ => Outcome::Forward(Status::Ok),
        }
    }
}
