// Rocket's proc-macro routes appear dead to static analysis but are invoked at runtime.
#![allow(dead_code)]

use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use crate::guards::*;
use crate::handlers;
use crate::ProjectInfo;
use rocket::fs::NamedFile;
use rocket::serde::json::Json;
use rocket::{Request, State};
use rocket_dyn_templates::Template;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

#[rocket::get("/", rank = 1)]
pub(crate) async fn root_plain_cli(
    req_info: RequesterInfo<'_>,
    _cli_req: CliClientRequest<'_>,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Option<String> {
    handlers::root::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
}

#[rocket::get("/", format = "text/plain", rank = 2)]
pub(crate) fn root_plain(
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Option<String> {
    handlers::root::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
}

#[rocket::get("/", format = "application/json", rank = 3)]
pub(crate) fn root_json(
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Option<Json<JsonValue>> {
    handlers::root::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
}

#[rocket::get("/", rank = 4)]
pub(crate) fn root_html(
    project_info: &State<ProjectInfo>,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Template {
    handlers::root_html(project_info, req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
}

#[rocket::get("/json")]
pub(crate) fn root_json_json(
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Option<Json<JsonValue>> {
    handlers::root::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
}

#[rocket::catch(404)]
pub(crate) fn not_found(_: &Request) -> &'static str {
    "not implemented"
}

macro_rules! route {
    ($name:ident, $route:tt, $route_json:tt) => {
        pub mod $name {
            use crate::backend::user_agent::UserAgentParser;
            use crate::backend::*;
            use crate::guards::*;
            use crate::handlers;
            use rocket::serde::json::Json;
            use rocket::State;
            use serde_json::Value as JsonValue;

            #[rocket::get($route, rank = 1)]
            pub(crate) fn plain_cli(
                req_info: RequesterInfo,
                _cli_req: CliClientRequest,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<String> {
                handlers::$name::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
            }

            #[rocket::get($route, format = "application/json", rank = 2)]
            pub(crate) fn json(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<Json<JsonValue>> {
                handlers::$name::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
            }

            #[rocket::get($route, rank = 3)]
            pub(crate) fn plain(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<String> {
                handlers::$name::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
            }

            #[rocket::get($route_json)]
            pub(crate) fn json_json(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<Json<JsonValue>> {
                handlers::$name::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
            }
        }
    };
}

route!(root, "/", "/json");

route!(ip, "/ip", "/ip/json");

route!(tcp, "/tcp", "/tcp/json");

route!(host, "/host", "/host/json");

route!(isp, "/isp", "/isp/json");

route!(location, "/location", "/location/json");

route!(user_agent, "/user_agent", "/user_agent/json");

pub mod headers {
    use crate::guards::*;
    use rocket::serde::json::Json;
    use serde_json::Value as JsonValue;
    use std::collections::BTreeMap;

    fn to_plain(req_headers: RequestHeaders) -> String {
        req_headers
            .headers
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    fn to_json(req_headers: RequestHeaders) -> Json<JsonValue> {
        let map: BTreeMap<&str, &str> = req_headers
            .headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        Json(serde_json::to_value(map).unwrap_or(JsonValue::Null))
    }

    #[rocket::get("/headers", rank = 1)]
    pub(crate) fn plain_cli(_cli_req: CliClientRequest, req_headers: RequestHeaders) -> String {
        to_plain(req_headers)
    }

    #[rocket::get("/headers", format = "application/json", rank = 2)]
    pub(crate) fn json(req_headers: RequestHeaders) -> Json<JsonValue> {
        to_json(req_headers)
    }

    #[rocket::get("/headers", rank = 3)]
    pub(crate) fn plain(req_headers: RequestHeaders) -> String {
        to_plain(req_headers)
    }

    #[rocket::get("/headers/json")]
    pub(crate) fn json_json(req_headers: RequestHeaders) -> Json<JsonValue> {
        to_json(req_headers)
    }
}

#[rocket::get("/<file..>", rank = 5)]
pub(crate) async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("htdocs/").join(file)).await.ok()
}
