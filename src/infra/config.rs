use std::{env, fs, path::PathBuf, time::Duration};

use serde::Deserialize;

use crate::domain::errors::{GatewayError, GatewayResult};

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub tavily: TavilyConfig,
    #[serde(default)]
    pub ddg: DuckDuckGoConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig::default(),
            tavily: TavilyConfig::default(),
            ddg: DuckDuckGoConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load(config_path: Option<PathBuf>) -> GatewayResult<Self> {
        let path = config_path
            .or_else(|| env::var_os("SEARCH_MCP_GATEWAY_CONFIG").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("search-mcp-gateway.toml"));

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).map_err(|error| {
            GatewayError::config(format!(
                "failed to read config at {}: {error}",
                path.display()
            ))
        })?;

        toml::from_str::<Self>(&content).map_err(|error| {
            GatewayError::config(format!(
                "failed to parse config at {}: {error}",
                path.display()
            ))
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_timeout_ms")]
    pub default_timeout_ms: u64,
    #[serde(default = "default_cache_enabled")]
    pub cache_enabled: bool,
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,
    #[serde(default = "default_failure_threshold")]
    pub circuit_failure_threshold: u32,
    #[serde(default = "default_circuit_open_seconds")]
    pub circuit_open_seconds: u64,
    #[serde(default = "default_provider_order")]
    pub search_provider_order: Vec<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: default_timeout_ms(),
            cache_enabled: default_cache_enabled(),
            cache_ttl_seconds: default_cache_ttl_seconds(),
            circuit_failure_threshold: default_failure_threshold(),
            circuit_open_seconds: default_circuit_open_seconds(),
            search_provider_order: default_provider_order(),
        }
    }
}

impl GatewayConfig {
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_seconds)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TavilyConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_tavily_base_url")]
    pub base_url: String,
    #[serde(default = "default_tavily_api_key_env")]
    pub api_key_env: String,
    pub api_key: Option<String>,
    #[serde(default = "default_tavily_search_path")]
    pub search_path: String,
    #[serde(default = "default_tavily_extract_path")]
    pub extract_path: String,
    #[serde(default = "default_tavily_crawl_path")]
    pub crawl_path: String,
}

impl Default for TavilyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: default_tavily_base_url(),
            api_key_env: default_tavily_api_key_env(),
            api_key: None,
            search_path: default_tavily_search_path(),
            extract_path: default_tavily_extract_path(),
            crawl_path: default_tavily_crawl_path(),
        }
    }
}

impl TavilyConfig {
    pub fn api_key(&self) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| env::var(&self.api_key_env).ok())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DuckDuckGoConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ddg_base_url")]
    pub base_url: String,
    #[serde(default = "default_ddg_lite_url")]
    pub lite_url: String,
    #[serde(default = "default_ddg_region")]
    pub region: String,
    #[serde(default = "default_ddg_safe_search")]
    pub safe_search: String,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
}

impl Default for DuckDuckGoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: default_ddg_base_url(),
            lite_url: default_ddg_lite_url(),
            region: default_ddg_region(),
            safe_search: default_ddg_safe_search(),
            user_agent: default_user_agent(),
        }
    }
}

fn default_timeout_ms() -> u64 {
    20_000
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl_seconds() -> u64 {
    120
}

fn default_failure_threshold() -> u32 {
    3
}

fn default_circuit_open_seconds() -> u64 {
    30
}

fn default_provider_order() -> Vec<String> {
    vec!["tavily".to_string(), "ddg".to_string()]
}

fn default_true() -> bool {
    true
}

fn default_tavily_base_url() -> String {
    "https://tavily.ivanli.cc".to_string()
}

fn default_tavily_api_key_env() -> String {
    "TAVILY_HIKARI_TOKEN".to_string()
}

fn default_tavily_search_path() -> String {
    "/api/tavily/search".to_string()
}

fn default_tavily_extract_path() -> String {
    "/api/tavily/extract".to_string()
}

fn default_tavily_crawl_path() -> String {
    "/api/tavily/crawl".to_string()
}

fn default_ddg_base_url() -> String {
    "https://html.duckduckgo.com/html/".to_string()
}

fn default_ddg_lite_url() -> String {
    "https://lite.duckduckgo.com/lite/".to_string()
}

fn default_ddg_region() -> String {
    "wt-wt".to_string()
}

fn default_ddg_safe_search() -> String {
    "moderate".to_string()
}

fn default_user_agent() -> String {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36".to_string()
}
