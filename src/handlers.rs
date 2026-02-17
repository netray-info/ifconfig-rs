use crate::backend::user_agent::UserAgentParser;
use crate::backend::*;
use crate::guards::*;
use crate::ProjectInfo;
use rocket::State;
use rocket_dyn_templates::Template;
use serde::Serialize;

pub(crate) static UNKNOWN_STR: &str = "unknown";

pub fn root_html(
    project_info: &State<ProjectInfo>,
    req_info: RequesterInfo,
    user_agent_parser: &State<UserAgentParser>,
    geoip_city_db: &State<GeoIpCityDb>,
    geoip_asn_db: &State<GeoIpAsnDb>,
) -> Template {
    let ifconfig_param = IfconfigParam {
        remote: &req_info.remote,
        user_agent_header: &req_info.user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
    };
    let ifconfig = get_ifconfig(&ifconfig_param);

    #[derive(Serialize)]
    struct Context<'a> {
        ifconfig: Ifconfig<'a>,
        project: &'a ProjectInfo,
        uri: &'a str,
    }

    let context = Context {
        ifconfig,
        project: project_info,
        uri: req_info.uri.as_ref(),
    };
    Template::render("index", context)
}

macro_rules! handler {
    ($name:ident, $ifconfig:ident, $json:block, $ty:ty, $plain:block) => {
        pub mod $name {
            use crate::backend::*;
            use crate::guards::*;
            #[allow(unused_imports)]
            use crate::handlers::UNKNOWN_STR;
            use rocket::serde::json::Json;
            use rocket::State;
            use serde_json::Value as JsonValue;

            fn to_json($ifconfig: Ifconfig) -> $ty {
                $json
            }

            pub fn json(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<Json<JsonValue>> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser: &user_agent_parser,
                    geoip_city_db: &geoip_city_db,
                    geoip_asn_db: &geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);

                let value = to_json(ifconfig);

                // 'Json(ifconfig)' requires a lifetime in the return value, which we cannot supply. Therefore, we serialize manually
                match serde_json::to_value(value) {
                    Ok(json) => Some(Json(json)),
                    Err(_) => None,
                }
            }

            fn to_plain($ifconfig: Ifconfig) -> String {
                $plain
            }

            pub fn plain(
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
            ) -> Option<String> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser: &user_agent_parser,
                    geoip_city_db: &geoip_city_db,
                    geoip_asn_db: &geoip_asn_db,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);

                let value = to_plain(ifconfig);

                Some(value)
            }
        }
    };
}

handler!(root, ifconfig, { ifconfig }, Ifconfig, {
    format!("{}\n", ifconfig.ip.addr)
});

handler!(ip, ifconfig, { ifconfig.ip }, Ip, { format!("{}\n", ifconfig.ip.addr) });

handler!(tcp, ifconfig, { ifconfig.tcp }, Tcp, {
    format!("{}\n", ifconfig.tcp.port)
});

handler!(host, ifconfig, { ifconfig.host }, Option<Host>, {
    format!(
        "{}\n",
        ifconfig.host.map(|h| h.name).unwrap_or_else(|| UNKNOWN_STR.to_string())
    )
});

handler!(isp, ifconfig, { ifconfig.isp }, Option<Isp>, {
    format!("{}\n", ifconfig.isp.and_then(|isp| isp.name).unwrap_or(UNKNOWN_STR))
});

handler!(location, ifconfig, { ifconfig.location }, Option<Location>, {
    format!(
        "{}, {}\n",
        ifconfig.location.as_ref().and_then(|l| l.city).unwrap_or(UNKNOWN_STR),
        ifconfig
            .location
            .as_ref()
            .and_then(|l| l.country)
            .unwrap_or(UNKNOWN_STR)
    )
});

handler!(user_agent, ifconfig, { ifconfig.user_agent }, Option<UserAgent>, {
    format!(
        "{}\n",
        ifconfig
            .user_agent
            .map(|ua| format!(
                "{}, {}, {}, {}",
                ua.browser.family, ua.browser.version, ua.os.family, ua.os.version
            ))
            .unwrap_or_else(|| UNKNOWN_STR.to_string())
    )
});
