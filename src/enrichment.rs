use crate::backend::asn_heuristic::AsnPatterns;
use crate::backend::user_agent::UserAgentParser;
use crate::backend::{
    AsnInfo, BotDb, CloudProviderDb, DatacenterRanges, FeodoBotnetIps, GeoIpAsnDb, GeoIpCityDb, SpamhausDrop,
    TorExitNodes, VpnRanges,
};
use crate::config::Config;
use mhost::resolver::{ResolverGroup, ResolverGroupBuilder};
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{error, info, warn};

fn file_mtime_iso(path: &str) -> Option<String> {
    let mtime = std::fs::metadata(path).ok()?.modified().ok()?;
    let secs = mtime.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    // Decompose epoch seconds into date + time components without external deps.
    let time_of_day = secs % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    // civil_from_days (Howard Hinnant) for the date portion.
    let z = (secs / 86400) as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    Some(format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z"))
}

pub struct DataFileDates {
    pub geoip_city: Option<String>,
    pub geoip_asn: Option<String>,
    pub user_agent: Option<String>,
    pub tor: Option<String>,
    pub vpn: Option<String>,
    pub cloud: Option<String>,
    pub datacenter: Option<String>,
    pub bot: Option<String>,
    pub feodo: Option<String>,
    pub spamhaus: Option<String>,
    pub asn_info: Option<String>,
}

/// Record `data_file_age_seconds{source=<label>}` from the file's mtime.
/// Silently skips if mtime cannot be read (file may be synthetic or on a
/// filesystem that doesn't support mtime).
fn emit_file_age(path: &str, source_label: &'static str) {
    let age_secs = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|mtime| SystemTime::now().duration_since(mtime).ok())
        .map(|d| d.as_secs_f64());
    if let Some(age) = age_secs {
        metrics::gauge!("data_file_age_seconds", "source" => source_label).set(age);
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to create DNS resolver: {0}")]
pub struct LoadError(String);

/// Groups all reloadable backend resources. Stored behind `ArcSwap` in `AppState`
/// so SIGHUP can atomically swap in a freshly loaded context without dropping
/// in-flight requests.
pub struct EnrichmentContext {
    pub user_agent_parser: Option<Arc<UserAgentParser>>,
    pub geoip_city_db: Option<Arc<GeoIpCityDb>>,
    pub geoip_asn_db: Option<Arc<GeoIpAsnDb>>,
    pub tor_exit_nodes: Arc<TorExitNodes>,
    pub feodo_botnet_ips: Option<Arc<FeodoBotnetIps>>,
    pub cloud_provider_db: Option<Arc<CloudProviderDb>>,
    pub vpn_ranges: Option<Arc<VpnRanges>>,
    pub datacenter_ranges: Option<Arc<DatacenterRanges>>,
    pub bot_db: Option<Arc<BotDb>>,
    pub spamhaus_drop: Option<Arc<SpamhausDrop>>,
    pub asn_patterns: Arc<AsnPatterns>,
    pub asn_info: Option<Arc<AsnInfo>>,
    pub dns_resolver: Arc<ResolverGroup>,
    pub geoip_city_build_epoch: Option<u64>,
    /// Optional sources that were configured (path provided) but failed to load.
    /// Surfaced in `/ready` warnings and emitted as a single startup log line.
    pub missing_optional: Vec<&'static str>,
    pub data_file_dates: DataFileDates,
}

impl EnrichmentContext {
    /// Load all backends from the paths specified in `config`.
    /// DNS resolver is built from system config (async).
    pub async fn load(config: &Config) -> Result<Self, LoadError> {
        let geoip_city_db = if let Some(path) = config.geoip_city_db.as_deref() {
            match GeoIpCityDb::new(path).await {
                Some(db) => {
                    info!("Loaded GeoIP City database from {} ({} nodes)", path, db.node_count());
                    Some(Arc::new(db))
                }
                None => {
                    error!("Failed to load GeoIP City database from {}", path);
                    return Err(LoadError(format!("Failed to load GeoIP City database from {path}")));
                }
            }
        } else {
            warn!("geoip_city_db not configured; geolocation lookups disabled");
            None
        };

        let geoip_asn_db = if let Some(path) = config.geoip_asn_db.as_deref() {
            match GeoIpAsnDb::new(path).await {
                Some(db) => {
                    info!("Loaded GeoIP ASN database from {} ({} nodes)", path, db.node_count());
                    Some(Arc::new(db))
                }
                None => {
                    error!("Failed to load GeoIP ASN database from {}", path);
                    return Err(LoadError(format!("Failed to load GeoIP ASN database from {path}")));
                }
            }
        } else {
            warn!("geoip_asn_db not configured; ISP/ASN lookups disabled");
            None
        };

        let user_agent_parser = if let Some(path) = config.user_agent_regexes.as_deref() {
            match UserAgentParser::from_yaml(path).await {
                Ok(parser) => {
                    info!("Loaded User-Agent regexes from {}", path);
                    Some(Arc::new(parser))
                }
                Err(e) => {
                    error!("Failed to load User-Agent regexes from {}: {}", path, e);
                    return Err(LoadError(format!("Failed to load User-Agent regexes from {path}: {e}")));
                }
            }
        } else {
            warn!("user_agent_regexes not configured; User-Agent parsing disabled");
            None
        };

        let tor_exit_nodes = if let Some(path) = config.tor_exit_nodes.as_deref() {
            let nodes = TorExitNodes::from_file(path).await;
            if let Some(count) = nodes.len() {
                info!("Loaded Tor exit nodes from {} ({} IPs)", path, count);
            } else {
                warn!("Failed to load Tor exit nodes from {}", path);
            }
            nodes
        } else {
            warn!("tor_exit_nodes not configured; Tor exit node detection disabled");
            TorExitNodes::empty()
        };

        let feodo_botnet_ips = if let Some(path) = config.feodo_botnet_ips.as_deref() {
            let ips = FeodoBotnetIps::from_file(path).await;
            if let Some(count) = ips.len() {
                info!("Loaded Feodo botnet IPs from {} ({} IPs)", path, count);
                Some(Arc::new(ips))
            } else {
                warn!("Failed to load Feodo botnet IPs from {}", path);
                None
            }
        } else {
            warn!("feodo_botnet_ips not configured; botnet C2 detection disabled");
            None
        };

        let cloud_provider_db = if let Some(path) = config.cloud_provider_ranges.as_deref() {
            match CloudProviderDb::from_file(path).await {
                Some(db) => {
                    info!("Loaded cloud provider ranges from {} ({} ranges)", path, db.len());
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load cloud provider ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("cloud_provider_ranges not configured; cloud provider detection disabled");
            None
        };

        let vpn_ranges = if let Some(path) = config.vpn_ranges.as_deref() {
            match VpnRanges::from_file(path).await {
                Some(db) => {
                    info!("Loaded VPN ranges from {} ({} ranges)", path, db.len());
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load VPN ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("vpn_ranges not configured; CIDR-based VPN detection disabled");
            None
        };

        let datacenter_ranges = if let Some(path) = config.datacenter_ranges.as_deref() {
            match DatacenterRanges::from_file(path).await {
                Some(db) => {
                    info!("Loaded datacenter ranges from {} ({} ranges)", path, db.len());
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load datacenter ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("datacenter_ranges not configured; CIDR-based datacenter detection disabled");
            None
        };

        let bot_db = if let Some(path) = config.bot_ranges.as_deref() {
            match BotDb::from_file(path).await {
                Some(db) => {
                    info!("Loaded bot ranges from {} ({} ranges)", path, db.len());
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load bot ranges from {}", path);
                    None
                }
            }
        } else {
            warn!("bot_ranges not configured; bot IP detection disabled");
            None
        };

        let spamhaus_drop = if let Some(path) = config.spamhaus_drop.as_deref() {
            match SpamhausDrop::from_file(path).await {
                Some(db) => {
                    info!("Loaded Spamhaus DROP from {} ({} ranges)", path, db.len());
                    Some(Arc::new(db))
                }
                None => {
                    warn!("Failed to load Spamhaus DROP from {}", path);
                    None
                }
            }
        } else {
            warn!("spamhaus_drop not configured; threat/hijacked-netblock detection disabled");
            None
        };

        let asn_patterns = Arc::new(if let Some(path) = config.asn_patterns.as_deref() {
            match AsnPatterns::from_file(path).await {
                Ok(p) => {
                    info!(
                        "Loaded ASN patterns from {} ({} patterns)",
                        path,
                        p.hosting.len() + p.vpn.len()
                    );
                    p
                }
                Err(e) => {
                    warn!("Failed to load ASN patterns from {path}: {e}; using built-in defaults");
                    AsnPatterns::builtin()
                }
            }
        } else {
            AsnPatterns::builtin()
        });

        let asn_info = if let Some(path) = config.asn_info.as_deref() {
            match AsnInfo::from_file(path).await {
                Ok(db) => {
                    info!("Loaded ASN info from {} ({} ASNs)", path, db.len());
                    Some(Arc::new(db))
                }
                Err(e) => {
                    warn!("Failed to load ASN info from {path}: {e}; ASN category/role lookup disabled");
                    None
                }
            }
        } else {
            warn!("asn_info not configured; ASN category/role lookup disabled");
            None
        };

        let dns_resolver = ResolverGroupBuilder::new()
            .system()
            .build()
            .await
            .map_err(|e| LoadError(e.to_string()))?;
        info!("DNS resolver initialized from system config");

        let geoip_city_build_epoch = geoip_city_db.as_ref().map(|db| db.build_epoch());

        // Collect optional sources that were configured but failed to load.
        let missing_optional: Vec<&'static str> = [
            (
                config.user_agent_regexes.is_some() && user_agent_parser.is_none(),
                "user_agent_regexes",
            ),
            (
                config.tor_exit_nodes.is_some() && !tor_exit_nodes.is_loaded(),
                "tor_exit_nodes",
            ),
            (
                config.feodo_botnet_ips.is_some() && feodo_botnet_ips.is_none(),
                "feodo_botnet_ips",
            ),
            (
                config.cloud_provider_ranges.is_some() && cloud_provider_db.is_none(),
                "cloud_provider_ranges",
            ),
            (config.vpn_ranges.is_some() && vpn_ranges.is_none(), "vpn_ranges"),
            (
                config.datacenter_ranges.is_some() && datacenter_ranges.is_none(),
                "datacenter_ranges",
            ),
            (config.bot_ranges.is_some() && bot_db.is_none(), "bot_ranges"),
            (
                config.spamhaus_drop.is_some() && spamhaus_drop.is_none(),
                "spamhaus_drop",
            ),
            (config.asn_info.is_some() && asn_info.is_none(), "asn_info"),
        ]
        .into_iter()
        .filter_map(|(failed, name)| if failed { Some(name) } else { None })
        .collect();

        if !missing_optional.is_empty() {
            warn!("Optional data sources not loaded: {}", missing_optional.join(", "));
        }

        let data_file_dates = DataFileDates {
            geoip_city: config.geoip_city_db.as_deref().and_then(file_mtime_iso),
            geoip_asn: config.geoip_asn_db.as_deref().and_then(file_mtime_iso),
            user_agent: config.user_agent_regexes.as_deref().and_then(file_mtime_iso),
            tor: if tor_exit_nodes.is_loaded() {
                config.tor_exit_nodes.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            vpn: if vpn_ranges.is_some() {
                config.vpn_ranges.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            cloud: if cloud_provider_db.is_some() {
                config.cloud_provider_ranges.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            datacenter: if datacenter_ranges.is_some() {
                config.datacenter_ranges.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            bot: if bot_db.is_some() {
                config.bot_ranges.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            feodo: if feodo_botnet_ips.is_some() {
                config.feodo_botnet_ips.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            spamhaus: if spamhaus_drop.is_some() {
                config.spamhaus_drop.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
            asn_info: if asn_info.is_some() {
                config.asn_info.as_deref().and_then(file_mtime_iso)
            } else {
                None
            },
        };

        let ctx = EnrichmentContext {
            user_agent_parser,
            geoip_city_db,
            geoip_asn_db,
            tor_exit_nodes: Arc::new(tor_exit_nodes),
            feodo_botnet_ips,
            cloud_provider_db,
            vpn_ranges,
            datacenter_ranges,
            bot_db,
            spamhaus_drop,
            asn_patterns,
            asn_info,
            dns_resolver: Arc::new(dns_resolver),
            geoip_city_build_epoch,
            missing_optional,
            data_file_dates,
        };

        // Report which enrichment sources are loaded (updates on reload too).
        metrics::gauge!("enrichment_sources_loaded", "source" => "geoip_city")
            .set(f64::from(ctx.geoip_city_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "geoip_asn")
            .set(f64::from(ctx.geoip_asn_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "user_agent")
            .set(f64::from(ctx.user_agent_parser.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "tor_exit_nodes")
            .set(f64::from(ctx.tor_exit_nodes.is_loaded() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "cloud_provider")
            .set(f64::from(ctx.cloud_provider_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "vpn_ranges")
            .set(f64::from(ctx.vpn_ranges.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "datacenter_ranges")
            .set(f64::from(ctx.datacenter_ranges.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "bot_ranges")
            .set(f64::from(ctx.bot_db.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "feodo_botnet")
            .set(f64::from(ctx.feodo_botnet_ips.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "spamhaus_drop")
            .set(f64::from(ctx.spamhaus_drop.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "asn_patterns")
            .set(f64::from(config.asn_patterns.is_some() as u8));
        metrics::gauge!("enrichment_sources_loaded", "source" => "asn_info")
            .set(f64::from(ctx.asn_info.is_some() as u8));

        // Emit data_file_age_seconds for each successfully loaded optional file.
        if ctx.feodo_botnet_ips.is_some() {
            if let Some(path) = config.feodo_botnet_ips.as_deref() {
                emit_file_age(path, "feodo_botnet");
            }
        }
        if ctx.spamhaus_drop.is_some() {
            if let Some(path) = config.spamhaus_drop.as_deref() {
                emit_file_age(path, "spamhaus_drop");
            }
        }
        if ctx.vpn_ranges.is_some() {
            if let Some(path) = config.vpn_ranges.as_deref() {
                emit_file_age(path, "vpn_ranges");
            }
        }
        if ctx.cloud_provider_db.is_some() {
            if let Some(path) = config.cloud_provider_ranges.as_deref() {
                emit_file_age(path, "cloud_cidrs");
            }
        }
        if ctx.datacenter_ranges.is_some() {
            if let Some(path) = config.datacenter_ranges.as_deref() {
                emit_file_age(path, "datacenter_ranges");
            }
        }
        if ctx.bot_db.is_some() {
            if let Some(path) = config.bot_ranges.as_deref() {
                emit_file_age(path, "bot_ranges");
            }
        }
        if ctx.tor_exit_nodes.is_loaded() {
            if let Some(path) = config.tor_exit_nodes.as_deref() {
                emit_file_age(path, "tor_exit_nodes");
            }
        }
        if ctx.asn_info.is_some() {
            if let Some(path) = config.asn_info.as_deref() {
                emit_file_age(path, "asn_info");
            }
        }

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn load_with_no_paths_succeeds() {
        let config = Config::load(None).unwrap();
        let ctx = EnrichmentContext::load(&config).await;
        assert!(ctx.is_ok());
        let ctx = ctx.unwrap();
        assert!(ctx.geoip_city_db.is_none());
        assert!(ctx.geoip_asn_db.is_none());
        assert!(ctx.user_agent_parser.is_none());
        assert!(ctx.cloud_provider_db.is_none());
        assert!(ctx.vpn_ranges.is_none());
        assert!(ctx.geoip_city_build_epoch.is_none());
    }

    #[tokio::test]
    async fn nonexistent_geoip_city_path_returns_error() {
        let mut config = Config::load(None).unwrap();
        config.geoip_city_db = Some("/nonexistent/GeoLite2-City.mmdb".to_string());
        let result = EnrichmentContext::load(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nonexistent_geoip_asn_path_returns_error() {
        let mut config = Config::load(None).unwrap();
        config.geoip_asn_db = Some("/nonexistent/GeoLite2-ASN.mmdb".to_string());
        let result = EnrichmentContext::load(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn nonexistent_ua_regex_path_returns_error() {
        let mut config = Config::load(None).unwrap();
        config.user_agent_regexes = Some("/nonexistent/regexes.yaml".to_string());
        let result = EnrichmentContext::load(&config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn no_geoip_means_build_epoch_is_none() {
        let config = Config::load(None).unwrap();
        let ctx = EnrichmentContext::load(&config).await.unwrap();
        assert!(ctx.geoip_city_build_epoch.is_none());
    }
}
