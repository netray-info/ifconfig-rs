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
    tor_exit_nodes: &State<TorExitNodes>,
) -> Template {
    let ifconfig_param = IfconfigParam {
        remote: &req_info.remote,
        user_agent_header: &req_info.user_agent,
        user_agent_parser,
        geoip_city_db,
        geoip_asn_db,
        tor_exit_nodes,
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
            use crate::format::OutputFormat;
            use crate::guards::*;
            #[allow(unused_imports)]
            use crate::handlers::UNKNOWN_STR;
            use rocket::http::ContentType;
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
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<Json<JsonValue>> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser: &user_agent_parser,
                    geoip_city_db: &geoip_city_db,
                    geoip_asn_db: &geoip_asn_db,
                    tor_exit_nodes: &tor_exit_nodes,
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
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<String> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser: &user_agent_parser,
                    geoip_city_db: &geoip_city_db,
                    geoip_asn_db: &geoip_asn_db,
                    tor_exit_nodes: &tor_exit_nodes,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);

                let value = to_plain(ifconfig);

                Some(value)
            }

            pub fn formatted(
                format: OutputFormat,
                req_info: RequesterInfo,
                user_agent_parser: &State<UserAgentParser>,
                geoip_city_db: &State<GeoIpCityDb>,
                geoip_asn_db: &State<GeoIpAsnDb>,
                tor_exit_nodes: &State<TorExitNodes>,
            ) -> Option<(ContentType, String)> {
                let ifconfig_param = IfconfigParam {
                    remote: &req_info.remote,
                    user_agent_header: &req_info.user_agent,
                    user_agent_parser: &user_agent_parser,
                    geoip_city_db: &geoip_city_db,
                    geoip_asn_db: &geoip_asn_db,
                    tor_exit_nodes: &tor_exit_nodes,
                };
                let ifconfig = get_ifconfig(&ifconfig_param);
                let value = to_json(ifconfig);
                let json_val = serde_json::to_value(value).ok()?;
                format.serialize(&json_val)
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
    let name = ifconfig.isp.as_ref().and_then(|isp| isp.name).unwrap_or(UNKNOWN_STR);
    let asn = ifconfig.isp.as_ref().and_then(|isp| isp.asn);
    match asn {
        Some(n) => format!("{} (AS{})\n", name, n),
        None => format!("{}\n", name),
    }
});

handler!(location, ifconfig, { ifconfig.location }, Option<Location>, {
    let city = ifconfig.location.as_ref().and_then(|l| l.city).unwrap_or(UNKNOWN_STR);
    let country = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.country)
        .unwrap_or(UNKNOWN_STR);
    let iso = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.country_iso)
        .unwrap_or(UNKNOWN_STR);
    let continent = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.continent)
        .unwrap_or(UNKNOWN_STR);
    let timezone = ifconfig
        .location
        .as_ref()
        .and_then(|l| l.timezone)
        .unwrap_or(UNKNOWN_STR);
    format!("{}, {} ({}), {}, {}\n", city, country, iso, continent, timezone)
});

handler!(all, ifconfig, { ifconfig }, Ifconfig, {
    let mut lines = Vec::new();
    lines.push(format!("ip:         {}", ifconfig.ip.addr));
    lines.push(format!("version:    {}", ifconfig.ip.version));
    if let Some(ref host) = ifconfig.host {
        lines.push(format!("hostname:   {}", host.name));
    }
    if let Some(ref loc) = ifconfig.location {
        if let Some(city) = loc.city {
            lines.push(format!("city:       {}", city));
        }
        if let Some(country) = loc.country {
            lines.push(format!("country:    {}", country));
        }
        if let Some(iso) = loc.country_iso {
            lines.push(format!("country_iso: {}", iso));
        }
        if let Some(continent) = loc.continent {
            lines.push(format!("continent:  {}", continent));
        }
        if let Some(tz) = loc.timezone {
            lines.push(format!("timezone:   {}", tz));
        }
        if let Some(lat) = loc.latitude {
            lines.push(format!("latitude:   {}", lat));
        }
        if let Some(lon) = loc.longitude {
            lines.push(format!("longitude:  {}", lon));
        }
    }
    if let Some(ref isp) = ifconfig.isp {
        if let Some(name) = isp.name {
            lines.push(format!("isp:        {}", name));
        }
        if let Some(asn) = isp.asn {
            lines.push(format!("asn:        AS{}", asn));
        }
    }
    if let Some(is_tor) = ifconfig.is_tor {
        lines.push(format!("tor:        {}", is_tor));
    }
    lines.push(format!("port:       {}", ifconfig.tcp.port));
    if let Some(ref ua) = ifconfig.user_agent {
        lines.push(format!("browser:    {} {}", ua.browser.family, ua.browser.version));
        lines.push(format!("os:         {} {}", ua.os.family, ua.os.version));
    }
    lines.join("\n") + "\n"
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
