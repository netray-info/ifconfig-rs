use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use crate::format::OutputFormat;
use crate::guards::*;
use crate::handlers;
use crate::rate_limiter::RateLimited;
use crate::ProjectInfo;
use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::serde::json::Json;
use rocket::{Request, State};
use rocket_dyn_templates::Template;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};

#[rocket::get("/", rank = 1)]
pub(crate) fn root_plain_cli(
    _rate_limited: RateLimited,
    req_info: RequesterInfo<'_>,
    _cli_req: CliClientRequest<'_>,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<String> {
    handlers::root::plain(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
}

#[rocket::get("/", format = "text/plain", rank = 2)]
pub(crate) fn root_plain(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<String> {
    handlers::root::plain(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
}

#[rocket::get("/", format = "application/json", rank = 3)]
pub(crate) fn root_json(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<Json<JsonValue>> {
    handlers::root::json(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
}

#[rocket::get("/", format = "application/yaml", rank = 4)]
pub(crate) fn root_yaml(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Yaml;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::get("/", format = "application/toml", rank = 5)]
pub(crate) fn root_toml(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Toml;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::get("/", format = "text/csv", rank = 6)]
pub(crate) fn root_csv(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Csv;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::get("/", rank = 7)]
pub(crate) fn root_html(
    _rate_limited: RateLimited,
    project_info: &State<ProjectInfo>,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Template {
    let context = handlers::root_html(
        project_info,
        &req_info,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
    );
    Template::render("index", context)
}

#[rocket::get("/json")]
pub(crate) fn root_json_json(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<Json<JsonValue>> {
    handlers::root::json(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
}

#[rocket::get("/yaml")]
pub(crate) fn root_yaml_suffix(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Yaml;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::get("/toml")]
pub(crate) fn root_toml_suffix(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Toml;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::get("/csv")]
pub(crate) fn root_csv_suffix(
    _rate_limited: RateLimited,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
    tor_exit_nodes: &State<TorExitNodes>,
) -> Option<(ContentType, String)> {
    let fmt = OutputFormat::Csv;
    let body = handlers::root::formatted(&fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
    let (top, sub) = fmt.mime_type();
    Some((ContentType::new(top, sub), body))
}

#[rocket::catch(404)]
pub(crate) fn not_found(_: &Request) -> &'static str {
    "not implemented"
}

#[rocket::catch(429)]
pub(crate) fn too_many_requests(_: &Request) -> &'static str {
    "rate limit exceeded\n"
}

macro_rules! route {
    ($name:ident, $route:tt, $route_json:tt, $route_fmt:tt) => {
        pub mod $name {
            use crate::backend::user_agent::UserAgentParser;
            use crate::backend::*;
            use crate::format::OutputFormat;
            use crate::guards::*;
            use crate::handlers;
            use crate::rate_limiter::RateLimited;
            use rocket::http::ContentType;
            use rocket::serde::json::Json;
            use rocket::State;
            use serde_json::Value as JsonValue;

            #[rocket::get($route, rank = 1)]
            pub(crate) fn plain_cli(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                _cli_req: CliClientRequest,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<String> {
                handlers::$name::plain(
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )
            }

            #[rocket::get($route, format = "application/json", rank = 2)]
            pub(crate) fn json(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<Json<JsonValue>> {
                handlers::$name::json(
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                ).map(Json)
            }

            #[rocket::get($route, format = "application/yaml", rank = 3)]
            pub(crate) fn yaml(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Yaml;
                let body = handlers::$name::formatted(
                    &fmt,
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, format = "application/toml", rank = 4)]
            pub(crate) fn toml_accept(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Toml;
                let body = handlers::$name::formatted(
                    &fmt,
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, format = "text/csv", rank = 5)]
            pub(crate) fn csv(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Csv;
                let body = handlers::$name::formatted(
                    &fmt,
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, rank = 6)]
            pub(crate) fn plain(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<String> {
                handlers::$name::plain(
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )
            }

            #[rocket::get($route_json)]
            pub(crate) fn json_json(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<Json<JsonValue>> {
                handlers::$name::json(
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                ).map(Json)
            }

            #[rocket::get($route_fmt)]
            pub(crate) fn format_suffix(
                _rate_limited: RateLimited,
                fmt: OutputFormat,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let body = handlers::$name::formatted(
                    &fmt,
                    &req_info,
                    user_agent_parser,
                    geoip_city_db,
                    geoip_asn_db,
                    tor_exit_nodes,
                )?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }
        }
    };
}

route!(ip, "/ip", "/ip/json", "/ip/<fmt>");

route!(tcp, "/tcp", "/tcp/json", "/tcp/<fmt>");

route!(host, "/host", "/host/json", "/host/<fmt>");

route!(isp, "/isp", "/isp/json", "/isp/<fmt>");

route!(location, "/location", "/location/json", "/location/<fmt>");

route!(user_agent, "/user_agent", "/user_agent/json", "/user_agent/<fmt>");

pub mod all {
    use crate::backend::user_agent::UserAgentParser;
    use crate::backend::*;
    use crate::format::OutputFormat;
    use crate::guards::*;
    use crate::handlers;
    use crate::rate_limiter::RateLimited;
    use rocket::http::ContentType;
    use rocket::serde::json::Json;
    use rocket::State;
    use serde_json::Value as JsonValue;

    #[rocket::get("/all", rank = 1)]
    pub(crate) fn plain_cli(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        _cli_req: CliClientRequest,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<String> {
        handlers::all::plain(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
    }

    #[rocket::get("/all", format = "application/json", rank = 2)]
    pub(crate) fn json(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<Json<JsonValue>> {
        handlers::all::json(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
    }

    #[rocket::get("/all", format = "application/yaml", rank = 3)]
    pub(crate) fn yaml(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Yaml;
        let body = handlers::all::formatted(
            &fmt,
            &req_info,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/all", format = "application/toml", rank = 4)]
    pub(crate) fn toml_accept(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Toml;
        let body = handlers::all::formatted(
            &fmt,
            &req_info,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/all", format = "text/csv", rank = 5)]
    pub(crate) fn csv(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Csv;
        let body = handlers::all::formatted(
            &fmt,
            &req_info,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/all", rank = 6)]
    pub(crate) fn plain(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<String> {
        handlers::all::plain(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
    }

    #[rocket::get("/all/json")]
    pub(crate) fn json_json(
        _rate_limited: RateLimited,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<Json<JsonValue>> {
        handlers::all::json(&req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
    }

    #[rocket::get("/all/<fmt>")]
    pub(crate) fn format_suffix(
        _rate_limited: RateLimited,
        fmt: OutputFormat,
        req_info: RequesterInfo,
        user_agent_parser: &State<UserAgentParser>,
        geoip_city_db: &State<GeoIpCityDb>,
        geoip_asn_db: &State<GeoIpAsnDb>,
        tor_exit_nodes: &State<TorExitNodes>,
    ) -> Option<(ContentType, String)> {
        let body = handlers::all::formatted(
            &fmt,
            &req_info,
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes,
        )?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }
}

macro_rules! ip_version_route {
    ($name:ident, $version:tt, $route:tt, $route_json:tt, $route_fmt:tt) => {
        pub mod $name {
            use crate::backend::user_agent::UserAgentParser;
            use crate::backend::*;
            use crate::format::OutputFormat;
            use crate::guards::*;
            use crate::handlers;
            use crate::rate_limiter::RateLimited;
            use rocket::http::ContentType;
            use rocket::serde::json::Json;
            use rocket::State;
            use serde_json::Value as JsonValue;

            #[rocket::get($route, rank = 1)]
            pub(crate) fn plain_cli(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                _cli_req: CliClientRequest,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<String> {
                handlers::ip_version::plain($version, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
            }

            #[rocket::get($route, format = "application/json", rank = 2)]
            pub(crate) fn json(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<Json<JsonValue>> {
                handlers::ip_version::json($version, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
            }

            #[rocket::get($route, format = "application/yaml", rank = 3)]
            pub(crate) fn yaml(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Yaml;
                let body = handlers::ip_version::formatted($version, &fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, format = "application/toml", rank = 4)]
            pub(crate) fn toml_accept(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Toml;
                let body = handlers::ip_version::formatted($version, &fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, format = "text/csv", rank = 5)]
            pub(crate) fn csv(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let fmt = OutputFormat::Csv;
                let body = handlers::ip_version::formatted($version, &fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }

            #[rocket::get($route, rank = 6)]
            pub(crate) fn plain(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<String> {
                handlers::ip_version::plain($version, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)
            }

            #[rocket::get($route_json)]
            pub(crate) fn json_json(
                _rate_limited: RateLimited,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<Json<JsonValue>> {
                handlers::ip_version::json($version, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes).map(Json)
            }

            #[rocket::get($route_fmt)]
            pub(crate) fn format_suffix(
                _rate_limited: RateLimited,
                fmt: OutputFormat,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let body = handlers::ip_version::formatted($version, &fmt, &req_info, user_agent_parser, geoip_city_db, geoip_asn_db, tor_exit_nodes)?;
                let (top, sub) = fmt.mime_type();
                Some((ContentType::new(top, sub), body))
            }
        }
    };
}

ip_version_route!(ipv4, "4", "/ipv4", "/ipv4/json", "/ipv4/<fmt>");
ip_version_route!(ipv6, "6", "/ipv6", "/ipv6/json", "/ipv6/<fmt>");

pub mod headers {
    use crate::format::OutputFormat;
    use crate::guards::*;
    use crate::handlers;
    use crate::rate_limiter::RateLimited;
    use rocket::http::ContentType;
    use rocket::serde::json::Json;
    use serde_json::Value as JsonValue;

    #[rocket::get("/headers", rank = 1)]
    pub(crate) fn plain_cli(_rate_limited: RateLimited, _cli_req: CliClientRequest, req_headers: RequestHeaders) -> String {
        handlers::headers::to_plain(&req_headers)
    }

    #[rocket::get("/headers", format = "application/json", rank = 2)]
    pub(crate) fn json(_rate_limited: RateLimited, req_headers: RequestHeaders) -> Json<JsonValue> {
        Json(handlers::headers::to_json_value(&req_headers))
    }

    #[rocket::get("/headers", format = "application/yaml", rank = 3)]
    pub(crate) fn yaml(_rate_limited: RateLimited, req_headers: RequestHeaders) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Yaml;
        let body = handlers::headers::formatted(&fmt, &req_headers)?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/headers", format = "application/toml", rank = 4)]
    pub(crate) fn toml_accept(_rate_limited: RateLimited, req_headers: RequestHeaders) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Toml;
        let body = handlers::headers::formatted(&fmt, &req_headers)?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/headers", format = "text/csv", rank = 5)]
    pub(crate) fn csv(_rate_limited: RateLimited, req_headers: RequestHeaders) -> Option<(ContentType, String)> {
        let fmt = OutputFormat::Csv;
        let body = handlers::headers::formatted(&fmt, &req_headers)?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
    }

    #[rocket::get("/headers", rank = 6)]
    pub(crate) fn plain(_rate_limited: RateLimited, req_headers: RequestHeaders) -> String {
        handlers::headers::to_plain(&req_headers)
    }

    #[rocket::get("/headers/json")]
    pub(crate) fn json_json(_rate_limited: RateLimited, req_headers: RequestHeaders) -> Json<JsonValue> {
        Json(handlers::headers::to_json_value(&req_headers))
    }

    #[rocket::get("/headers/<fmt>")]
    pub(crate) fn format_suffix(_rate_limited: RateLimited, fmt: OutputFormat, req_headers: RequestHeaders) -> Option<(ContentType, String)> {
        let body = handlers::headers::formatted(&fmt, &req_headers)?;
        let (top, sub) = fmt.mime_type();
        Some((ContentType::new(top, sub), body))
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

#[rocket::get("/<file..>", rank = 10)]
pub(crate) async fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("htdocs/").join(file)).await.ok()
}
