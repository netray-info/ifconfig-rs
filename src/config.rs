use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default = "Config::default_base_url")]
    pub base_url: String,
    /// Display name shown in the UI. Defaults to `base_url` if not set.
    pub site_name: Option<String>,
    #[serde(default = "Config::default_project_name")]
    pub project_name: String,
    #[serde(default = "Config::default_project_version")]
    pub project_version: String,
    pub geoip_city_db: Option<String>,
    pub geoip_asn_db: Option<String>,
    pub user_agent_regexes: Option<String>,
    pub tor_exit_nodes: Option<String>,
    pub cloud_provider_ranges: Option<String>,
    pub feodo_botnet_ips: Option<String>,
    pub vpn_ranges: Option<String>,
    pub datacenter_ranges: Option<String>,
    pub bot_ranges: Option<String>,
    pub spamhaus_drop: Option<String>,
    /// Regex patterns matched against header names. Matching headers are
    /// excluded from `/headers` responses. Case-insensitive match.
    #[serde(default)]
    pub filtered_headers: Vec<String>,
    /// When true, watch data file directories for changes and auto-reload
    /// enrichment data (like SIGHUP but filesystem-triggered). Useful for
    /// Kubernetes/Docker deployments with geoipupdate.
    #[serde(default)]
    pub watch_data_files: bool,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub batch: BatchConfig,
}

impl Config {
    fn default_base_url() -> String {
        "localhost".to_string()
    }
    fn default_project_name() -> String {
        env!("CARGO_PKG_NAME").to_string()
    }
    fn default_project_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_bind")]
    pub bind: String,
    pub admin_bind: Option<String>,
    /// Bearer token required for admin endpoints. If set, all requests to the
    /// admin port must include `Authorization: Bearer <token>`.
    pub admin_token: Option<String>,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
    #[serde(default = "ServerConfig::default_cors_allowed_origins")]
    pub cors_allowed_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: Self::default_bind(),
            admin_bind: None,
            admin_token: None,
            trusted_proxies: Vec::new(),
            cors_allowed_origins: Self::default_cors_allowed_origins(),
        }
    }
}

impl ServerConfig {
    fn default_bind() -> String {
        "127.0.0.1:8080".to_string()
    }
    fn default_cors_allowed_origins() -> Vec<String> {
        vec!["*".to_string()]
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    #[serde(default = "RateLimitConfig::default_per_ip_per_minute")]
    pub per_ip_per_minute: u32,
    #[serde(default = "RateLimitConfig::default_per_ip_burst")]
    pub per_ip_burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_ip_per_minute: Self::default_per_ip_per_minute(),
            per_ip_burst: Self::default_per_ip_burst(),
        }
    }
}

impl RateLimitConfig {
    fn default_per_ip_per_minute() -> u32 {
        60
    }
    fn default_per_ip_burst() -> u32 {
        10
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "BatchConfig::default_max_size")]
    pub max_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size: Self::default_max_size(),
        }
    }
}

impl BatchConfig {
    fn default_max_size() -> usize {
        100
    }
}

impl Config {
    pub fn load(path: Option<&str>) -> Result<Self, config::ConfigError> {
        let mut builder = config::Config::builder();

        if let Some(path) = path {
            builder = builder.add_source(config::File::with_name(path));
        }

        builder = builder.add_source(
            config::Environment::with_prefix("IFCONFIG")
                .separator("__")
                .try_parsing(true),
        );

        builder.build()?.try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_without_file_uses_defaults() {
        let config = Config::load(None).unwrap();
        assert_eq!(config.server.bind, "127.0.0.1:8080");
        assert_eq!(config.base_url, "localhost");
        assert_eq!(config.rate_limit.per_ip_per_minute, 60);
        assert_eq!(config.rate_limit.per_ip_burst, 10);
        assert!(!config.batch.enabled);
        assert_eq!(config.batch.max_size, 100);
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let result = Config::load(Some("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn server_config_default() {
        let server = ServerConfig::default();
        assert_eq!(server.bind, "127.0.0.1:8080");
        assert!(server.admin_bind.is_none());
        assert!(server.trusted_proxies.is_empty());
        assert_eq!(server.cors_allowed_origins, vec!["*"]);
    }

    #[test]
    fn rate_limit_config_default() {
        let rl = RateLimitConfig::default();
        assert_eq!(rl.per_ip_per_minute, 60);
        assert_eq!(rl.per_ip_burst, 10);
    }

    #[test]
    fn batch_config_default() {
        let batch = BatchConfig::default();
        assert!(!batch.enabled);
        assert_eq!(batch.max_size, 100);
    }

    #[test]
    fn config_round_trip_toml() {
        let config = Config::load(None).unwrap();
        let toml_str = toml::to_string(&config).unwrap();
        let _parsed: Config = toml::from_str(&toml_str).unwrap();
    }
}
