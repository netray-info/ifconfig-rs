use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
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
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_bind")]
    pub bind: String,
    pub admin_bind: Option<String>,
    #[serde(default)]
    pub trusted_proxies: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: Self::default_bind(),
            admin_bind: None,
            trusted_proxies: Vec::new(),
        }
    }
}

impl ServerConfig {
    fn default_bind() -> String {
        "127.0.0.1:8080".to_string()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RateLimitConfig {
    #[serde(default = "RateLimitConfig::default_per_ip_per_minute")]
    pub per_ip_per_minute: u64,
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
    fn default_per_ip_per_minute() -> u64 {
        60
    }
    fn default_per_ip_burst() -> u32 {
        10
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
