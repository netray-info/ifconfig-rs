use netray_common::telemetry::TelemetryConfig;
use serde::{Deserialize, Serialize};

pub const HARD_CAP_RATE_LIMIT_PER_MINUTE: u32 = 600;
pub const HARD_CAP_RATE_LIMIT_BURST: u32 = 100;
pub const HARD_CAP_BATCH_SIZE: usize = 100;

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
    pub cins_army_ips: Option<String>,
    pub vpn_ranges: Option<String>,
    pub datacenter_ranges: Option<String>,
    pub bot_ranges: Option<String>,
    pub spamhaus_drop: Option<String>,
    pub asn_patterns: Option<String>,
    pub asn_info: Option<String>,
    /// Regex patterns matched against header names. Matching headers are
    /// excluded from `/headers` responses. Case-insensitive match.
    #[serde(default)]
    pub filtered_headers: Vec<String>,
    /// When true, accept `?ip=` queries for private and reserved IP ranges
    /// (RFC 1918, loopback, link-local, ULA). GeoIP returns no results for
    /// these IPs; `network.type` will be `"internal"`.
    /// Off by default — enabling is appropriate only for intranet deployments.
    #[serde(default)]
    pub internal_mode: bool,
    /// When true, watch data file directories for changes and auto-reload
    /// enrichment data (like SIGHUP but filesystem-triggered). Useful for
    /// Kubernetes/Docker deployments with geoipupdate.
    #[serde(default)]
    pub watch_data_files: bool,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub batch: BatchConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default, skip_serializing)]
    pub telemetry: TelemetryConfig,
    /// Base URL of the DNS inspection tool (e.g. `https://dns.netray.info`).
    /// Exposed in `/meta` for cross-tool deep links.
    #[serde(default)]
    pub dns_base_url: Option<String>,
    /// Base URL of the TLS inspection tool (e.g. `https://tls.netray.info`).
    /// Exposed in `/meta` for cross-tool deep links.
    #[serde(default)]
    pub tls_base_url: Option<String>,
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
    #[serde(default = "ServerConfig::default_admin_bind_option")]
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
            admin_bind: Some(Self::default_admin_bind()),
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
    fn default_admin_bind() -> String {
        "127.0.0.1:9090".to_string()
    }
    fn default_admin_bind_option() -> Option<String> {
        Some(Self::default_admin_bind())
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
    #[serde(default = "RateLimitConfig::default_per_target_per_minute")]
    pub per_target_per_minute: u32,
    #[serde(default = "RateLimitConfig::default_per_target_burst")]
    pub per_target_burst: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_ip_per_minute: Self::default_per_ip_per_minute(),
            per_ip_burst: Self::default_per_ip_burst(),
            per_target_per_minute: Self::default_per_target_per_minute(),
            per_target_burst: Self::default_per_target_burst(),
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
    fn default_per_target_per_minute() -> u32 {
        120
    }
    fn default_per_target_burst() -> u32 {
        20
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default = "CacheConfig::default_enabled")]
    pub enabled: bool,
    #[serde(default = "CacheConfig::default_ttl_secs")]
    pub ttl_secs: u64,
    #[serde(default = "CacheConfig::default_max_entries")]
    pub max_entries: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: CacheConfig::default_enabled(),
            ttl_secs: CacheConfig::default_ttl_secs(),
            max_entries: CacheConfig::default_max_entries(),
        }
    }
}

impl CacheConfig {
    fn default_enabled() -> bool {
        true
    }
    fn default_ttl_secs() -> u64 {
        300
    }
    fn default_max_entries() -> u64 {
        1024
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
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true),
        );

        let cfg: Self = builder.build()?.try_deserialize()?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), config::ConfigError> {
        if self.rate_limit.per_ip_per_minute == 0 {
            return Err(config::ConfigError::Message(
                "rate_limit.per_ip_per_minute must be > 0".to_string(),
            ));
        }
        if self.rate_limit.per_ip_per_minute > HARD_CAP_RATE_LIMIT_PER_MINUTE {
            return Err(config::ConfigError::Message(format!(
                "rate_limit.per_ip_per_minute ({}) exceeds hard cap ({})",
                self.rate_limit.per_ip_per_minute, HARD_CAP_RATE_LIMIT_PER_MINUTE
            )));
        }
        if self.rate_limit.per_ip_burst == 0 {
            return Err(config::ConfigError::Message(
                "rate_limit.per_ip_burst must be > 0".to_string(),
            ));
        }
        if self.rate_limit.per_ip_burst > HARD_CAP_RATE_LIMIT_BURST {
            return Err(config::ConfigError::Message(format!(
                "rate_limit.per_ip_burst ({}) exceeds hard cap ({})",
                self.rate_limit.per_ip_burst, HARD_CAP_RATE_LIMIT_BURST
            )));
        }
        if self.rate_limit.per_target_per_minute > HARD_CAP_RATE_LIMIT_PER_MINUTE {
            return Err(config::ConfigError::Message(format!(
                "rate_limit.per_target_per_minute ({}) exceeds hard cap ({})",
                self.rate_limit.per_target_per_minute, HARD_CAP_RATE_LIMIT_PER_MINUTE
            )));
        }
        if self.rate_limit.per_target_burst > HARD_CAP_RATE_LIMIT_BURST {
            return Err(config::ConfigError::Message(format!(
                "rate_limit.per_target_burst ({}) exceeds hard cap ({})",
                self.rate_limit.per_target_burst, HARD_CAP_RATE_LIMIT_BURST
            )));
        }
        if self.batch.max_size > HARD_CAP_BATCH_SIZE {
            return Err(config::ConfigError::Message(format!(
                "batch.max_size ({}) exceeds hard cap ({})",
                self.batch.max_size, HARD_CAP_BATCH_SIZE
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_without_file_uses_defaults() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
        assert_eq!(server.admin_bind.as_deref(), Some("127.0.0.1:9090"));
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let config = Config::load(None).unwrap();
        let toml_str = toml::to_string(&config).unwrap();
        let _parsed: Config = toml::from_str(&toml_str).unwrap();
    }

    // Env-var tests share a mutex to prevent concurrent tests from clobbering
    // each other's IFCONFIG_* env vars (set_var is process-global).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn env_var_overrides_top_level_field() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: single-threaded test context, guarded by ENV_LOCK mutex
        unsafe { std::env::set_var("IFCONFIG_BASE_URL", "env-test.example.com") };
        let result = Config::load(None);
        // SAFETY: single-threaded test context, guarded by ENV_LOCK mutex
        unsafe { std::env::remove_var("IFCONFIG_BASE_URL") };
        let config = result.unwrap();
        assert_eq!(config.base_url, "env-test.example.com");
    }

    #[test]
    fn env_var_overrides_nested_field_with_double_underscore() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: single-threaded test context, guarded by ENV_LOCK mutex
        unsafe { std::env::set_var("IFCONFIG_SERVER__BIND", "0.0.0.0:9191") };
        let result = Config::load(None);
        // SAFETY: single-threaded test context, guarded by ENV_LOCK mutex
        unsafe { std::env::remove_var("IFCONFIG_SERVER__BIND") };
        let config = result.unwrap();
        assert_eq!(config.server.bind, "0.0.0.0:9191");
    }

    #[test]
    fn validate_rejects_zero_per_ip_per_minute() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut config = Config::load(None).unwrap();
        config.rate_limit.per_ip_per_minute = 0;
        assert!(
            config.validate().is_err(),
            "validate() should error when per_ip_per_minute is 0"
        );
    }

    #[test]
    fn validate_rejects_zero_per_ip_burst() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut config = Config::load(None).unwrap();
        config.rate_limit.per_ip_burst = 0;
        assert!(
            config.validate().is_err(),
            "validate() should error when per_ip_burst is 0"
        );
    }
}
