pub mod backend;
pub mod fairings;
pub mod guards;
pub mod handlers;
pub mod routes;

use fairings::*;
use routes::*;

use rocket::{catchers, routes, Build, Rocket};
use rocket_dyn_templates::Template;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
pub enum Runtime {
    #[serde(rename = "xforwarded")]
    XFORWARDED,
    #[default]
    #[serde(rename = "local")]
    LOCAL,
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
        .register("/", catchers![not_found])
        .mount(
            "/",
            routes![
                root_plain_cli,
                root_json,
                root_plain,
                root_html,
                root_json_json,
                ip::plain_cli,
                ip::json,
                ip::plain,
                ip::json_json,
                tcp::plain_cli,
                tcp::json,
                tcp::plain,
                tcp::json_json,
                host::plain_cli,
                host::json,
                host::plain,
                host::json_json,
                location::plain_cli,
                location::json,
                location::plain,
                location::json_json,
                isp::plain_cli,
                isp::json,
                isp::plain,
                isp::json_json,
                user_agent::plain_cli,
                user_agent::json,
                user_agent::plain,
                user_agent::json_json,
                headers::plain_cli,
                headers::json,
                headers::plain,
                headers::json_json,
                files
            ],
        )
        .attach(Template::fairing())
        .attach(SecurityHeaders);

    let config: Config = rocket.figment().extract().expect("config");

    rocket = match config.runtime {
        Runtime::XFORWARDED => rocket.attach(XForwardedFor),
        _ => rocket,
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
