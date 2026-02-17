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

pub mod all {
    use crate::backend::user_agent::UserAgentParser;
    use crate::backend::*;
    use crate::guards::*;
    use crate::handlers;
    use rocket::serde::json::Json;
    use rocket::State;
    use serde_json::Value as JsonValue;

    #[rocket::get("/all", rank = 1)]
    pub(crate) fn plain_cli(
        req_info: RequesterInfo,
        _cli_req: CliClientRequest,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
    ) -> Option<String> {
        handlers::all::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
    }

    #[rocket::get("/all", format = "application/json", rank = 2)]
    pub(crate) fn json(
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
    ) -> Option<Json<JsonValue>> {
        handlers::root::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
    }

    #[rocket::get("/all", rank = 3)]
    pub(crate) fn plain(
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
    ) -> Option<String> {
        handlers::all::plain(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
    }

    #[rocket::get("/all/json")]
    pub(crate) fn json_json(
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
    ) -> Option<Json<JsonValue>> {
        handlers::root::json(req_info, user_agent_parser, geoip_city_db, geoip_asn_db)
    }
}

macro_rules! ip_version_route {
    ($name:ident, $version:tt, $route:tt, $route_json:tt) => {
        pub mod $name {
            use crate::backend::user_agent::UserAgentParser;
            use crate::backend::*;
            use crate::guards::*;
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
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);
                if ifconfig.ip.version != $version {
                    return None;
                }
                Some(format!("{}\n", ifconfig.ip.addr))
            }

            #[rocket::get($route, format = "application/json", rank = 2)]
            pub(crate) fn json(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<Json<JsonValue>> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);
                if ifconfig.ip.version != $version {
                    return None;
                }
                serde_json::to_value(&ifconfig.ip).ok().map(Json)
            }

            #[rocket::get($route, rank = 3)]
            pub(crate) fn plain(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<String> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);
                if ifconfig.ip.version != $version {
                    return None;
                }
                Some(format!("{}\n", ifconfig.ip.addr))
            }

            #[rocket::get($route_json)]
            pub(crate) fn json_json(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<Json<JsonValue>> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);
                if ifconfig.ip.version != $version {
                    return None;
                }
                serde_json::to_value(&ifconfig.ip).ok().map(Json)
            }
        }
    };
}

ip_version_route!(ipv4, "4", "/ipv4", "/ipv4/json");
ip_version_route!(ipv6, "6", "/ipv6", "/ipv6/json");

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

pub mod health {
    use crate::backend::*;
    use rocket::http::Status;
    use rocket::serde::json::Json;
    use rocket::State;
    use serde_json::{json, Value as JsonValue};

    #[rocket::get("/health")]
    pub(crate) fn check(
        geoip_city_db: Option<&State<GeoIpCityDb>>,
        geoip_asn_db: Option<&State<GeoIpAsnDb>>,
    ) -> (Status, Json<JsonValue>) {
        let has_city_db = geoip_city_db.is_some();
        let has_asn_db = geoip_asn_db.is_some();

        if has_city_db && has_asn_db {
            (Status::Ok, Json(json!({ "status": "ok" })))
        } else {
            let mut missing = Vec::new();
            if !has_city_db {
                missing.push("GeoIP City database not loaded");
            }
            if !has_asn_db {
                missing.push("GeoIP ASN database not loaded");
            }
            (
                Status::ServiceUnavailable,
                Json(json!({
                    "status": "unhealthy",
                    "reason": missing.join("; ")
                })),
            )
        }
    }
}

#[rocket::get("/<file..>", rank = 5)]
pub(crate) async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("htdocs/").join(file)).await.ok()
}
