pub mod backend;
pub mod fairings;
pub mod format;
pub mod guards;
pub mod handlers;
pub mod rate_limiter;
pub mod routes;

use fairings::*;
use rate_limiter::RateLimiter;
use routes::*;

use rocket::{catchers, routes, Build, Rocket};
use std::time::Duration;
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub enum Runtime {
    #[serde(rename = "xforwarded")]
    XForwarded,
    #[default]
    #[serde(rename = "local")]
    Local,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Config {
    #[serde(default = "ProjectInfo::default_name")]
    project_name: String,
    #[serde(default = "ProjectInfo::default_version")]
    project_version: String,
    #[serde(default)]
    runtime: Runtime,
    base_url: String,
    geoip_city_db: Option<String>,
    geoip_asn_db: Option<String>,
    user_agent_regexes: Option<String>,
    tor_exit_nodes: Option<String>,
    #[serde(default = "Config::default_rate_limit_requests")]
    rate_limit_requests: u32,
    #[serde(default = "Config::default_rate_limit_window")]
    rate_limit_window: u64,
}

impl Config {
    fn default_rate_limit_requests() -> u32 {
        60
    }
    fn default_rate_limit_window() -> u64 {
        60
    }
}

#[derive(Serialize)]
pub struct ProjectInfo {
    name: String,
    version: String,
    base_url: String,
}

impl ProjectInfo {
    fn default_name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    fn default_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

impl From<&Config> for ProjectInfo {
    fn from(config: &Config) -> Self {
        ProjectInfo {
            name: config.project_name.clone(),
            version: config.project_version.clone(),
            base_url: config.base_url.clone(),
        }
    }
}

pub fn rocket() -> Rocket<Build> {
    let mut rocket = rocket::build()
        .register("/", catchers![not_found, too_many_requests])
        .mount(
            "/",
            routes![
                root_plain_cli,
                root_json,
                root_yaml,
                root_toml,
                root_csv,
                root_plain,
                root_html,
                root_json_json,
                root_yaml_suffix,
                root_toml_suffix,
                root_csv_suffix,
                ip::plain_cli,
                ip::json,
                ip::yaml,
                ip::toml_accept,
                ip::csv,
                ip::plain,
                ip::json_json,
                ip::format_suffix,
                tcp::plain_cli,
                tcp::json,
                tcp::yaml,
                tcp::toml_accept,
                tcp::csv,
                tcp::plain,
                tcp::json_json,
                tcp::format_suffix,
                host::plain_cli,
                host::json,
                host::yaml,
                host::toml_accept,
                host::csv,
                host::plain,
                host::json_json,
                host::format_suffix,
                location::plain_cli,
                location::json,
                location::yaml,
                location::toml_accept,
                location::csv,
                location::plain,
                location::json_json,
                location::format_suffix,
                isp::plain_cli,
                isp::json,
                isp::yaml,
                isp::toml_accept,
                isp::csv,
                isp::plain,
                isp::json_json,
                isp::format_suffix,
                user_agent::plain_cli,
                user_agent::json,
                user_agent::yaml,
                user_agent::toml_accept,
                user_agent::csv,
                user_agent::plain,
                user_agent::json_json,
                user_agent::format_suffix,
                ipv4::plain_cli,
                ipv4::json,
                ipv4::yaml,
                ipv4::toml_accept,
                ipv4::csv,
                ipv4::plain,
                ipv4::json_json,
                ipv4::format_suffix,
                ipv6::plain_cli,
                ipv6::json,
                ipv6::yaml,
                ipv6::toml_accept,
                ipv6::csv,
                ipv6::plain,
                ipv6::json_json,
                ipv6::format_suffix,
                all::plain_cli,
                all::json,
                all::yaml,
                all::toml_accept,
                all::csv,
                all::plain,
                all::json_json,
                all::format_suffix,
                headers::plain_cli,
                headers::json,
                headers::yaml,
                headers::toml_accept,
                headers::csv,
                headers::plain,
                headers::json_json,
                headers::format_suffix,
                health::check,
                files
            ],
        )
        .attach(Template::fairing())
        .attach(SecurityHeaders);

    let config: Config = rocket.figment().extract().expect("config");

    let rate_limiter = RateLimiter::new(
        config.rate_limit_requests,
        Duration::from_secs(config.rate_limit_window),
    );
    rocket = rocket.manage(rate_limiter);

    rocket = match config.runtime {
        Runtime::XForwarded => rocket.attach(XForwardedFor),
        Runtime::Local => rocket,
    };

    let project_info = ProjectInfo::from(&config);
    rocket = rocket.manage(project_info);

    rocket = match &config.user_agent_regexes {
        Some(db) => rocket.manage(init_user_agent_parser(db)),
        _ => rocket,
    };

    rocket = match &config.geoip_city_db {
        Some(db) => rocket.manage(init_geoip_city_db(db)),
        _ => rocket,
    };

    rocket = match &config.geoip_asn_db {
        Some(db) => rocket.manage(init_geoip_asn_db(db)),
        _ => rocket,
    };

    let tor_exit_nodes = config
        .tor_exit_nodes
        .as_deref()
        .map(backend::TorExitNodes::from_file)
        .unwrap_or_else(backend::TorExitNodes::empty);
    rocket = rocket.manage(tor_exit_nodes);

    rocket
}

fn init_user_agent_parser(regexes: &str) -> backend::user_agent::UserAgentParser {
    backend::UserAgentParser::from_yaml(regexes).expect("Failed to load User Agent regexes")
}

fn init_geoip_city_db(db: &str) -> backend::GeoIpCityDb {
    backend::GeoIpCityDb::new(db).expect("Failed to load GeoIP City DB")
}

fn init_geoip_asn_db(db: &str) -> backend::GeoIpAsnDb {
    backend::GeoIpAsnDb::new(db).expect("Failed to load GeoIP ASN DB")
}
