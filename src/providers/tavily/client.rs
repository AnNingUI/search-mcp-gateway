use std::{thread, time::Duration};

use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::{
    domain::{
        errors::{GatewayError, GatewayResult},
        models::{
            CrawlRequest, CrawlResponse, ExtractRequest, ExtractResponse, SearchDepth,
            SearchRequest, SearchResponse, SearchTopic,
        },
        provider::{Provider, ProviderCapabilities},
    },
    infra::{config::AppConfig, http::build_http_client},
    providers::tavily::mapper::{map_crawl_response, map_extract_response, map_search_response},
};

pub struct TavilyProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    search_path: String,
    extract_path: String,
    crawl_path: String,
}

impl TavilyProvider {
    const MAX_RETRY_ATTEMPTS: usize = 3;
    const INITIAL_BACKOFF_MS: u64 = 200;

    pub fn from_config(config: &AppConfig) -> GatewayResult<Self> {
        let client =
            build_http_client(config.gateway.default_timeout_ms, "search-mcp-gateway/0.1")?;
        Ok(Self {
            client,
            base_url: config.tavily.base_url.trim_end_matches('/').to_string(),
            api_key: config.tavily.api_key(),
            search_path: config.tavily.search_path.clone(),
            extract_path: config.tavily.extract_path.clone(),
            crawl_path: config.tavily.crawl_path.clone(),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn authed(
        &self,
        builder: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        if let Some(api_key) = &self.api_key {
            builder.bearer_auth(api_key)
        } else {
            builder
        }
    }

    fn send_json_once(&self, path: &str, payload: &Value, operation: &str) -> GatewayResult<Value> {
        let response = self
            .authed(self.client.post(self.endpoint(path)))
            .json(payload)
            .send()
            .map_err(|error| {
                GatewayError::provider(
                    self.name(),
                    format!("{operation}_transport_error"),
                    format!("{operation} request failed: {error}"),
                    true,
                )
            })?;

        let status = response.status();
        let body = response.text().map_err(|error| {
            GatewayError::provider(
                self.name(),
                format!("{operation}_read_error"),
                format!("failed to read {operation} response: {error}"),
                true,
            )
        })?;

        if !status.is_success() {
            return Err(GatewayError::provider(
                self.name(),
                format!("{operation}_http_error"),
                format!("{operation} request returned {status}: {body}"),
                status.is_server_error() || status.as_u16() == 429,
            ));
        }

        serde_json::from_str::<Value>(&body).map_err(|error| {
            GatewayError::provider(
                self.name(),
                format!("{operation}_decode_error"),
                format!("failed to decode {operation} response: {error}"),
                false,
            )
        })
    }

    fn send_json(&self, path: &str, payload: Value, operation: &str) -> GatewayResult<Value> {
        let mut last_error: Option<GatewayError> = None;

        for attempt in 1..=Self::MAX_RETRY_ATTEMPTS {
            match self.send_json_once(path, &payload, operation) {
                Ok(value) => return Ok(value),
                Err(error) if error.retryable && attempt < Self::MAX_RETRY_ATTEMPTS => {
                    last_error = Some(error);
                    thread::sleep(Duration::from_millis(
                        Self::INITIAL_BACKOFF_MS * attempt as u64,
                    ));
                }
                Err(mut error) => {
                    if attempt > 1 {
                        error.message = format!("{} after {attempt} attempts", error.message);
                    }
                    return Err(error);
                }
            }
        }

        let mut error = last_error.unwrap_or_else(|| {
            GatewayError::provider(
                self.name(),
                format!("{operation}_gateway_error"),
                format!("{operation} request exhausted retries without a concrete error"),
                true,
            )
        });
        error.message = format!(
            "{} after {} attempts",
            error.message,
            Self::MAX_RETRY_ATTEMPTS
        );
        Err(error)
    }
}

impl Provider for TavilyProvider {
    fn name(&self) -> &'static str {
        "tavily"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            search: true,
            extract: true,
            crawl: true,
        }
    }

    fn search(&self, request: &SearchRequest) -> GatewayResult<SearchResponse> {
        let payload = json!({
            "query": request.query,
            "topic": request.topic.unwrap_or(SearchTopic::General).as_str(),
            "search_depth": request.search_depth.unwrap_or(SearchDepth::Basic).as_str(),
            "max_results": request.max_results.unwrap_or(5),
            "include_answer": request.include_answer.unwrap_or(true),
            "include_raw_content": request.include_raw_content.unwrap_or(false),
            "include_images": request.include_images.unwrap_or(false),
            "days": request.days,
            "include_domains": request.site_filter,
            "exclude_domains": request.exclude_domains,
            "country": request.country,
            "language": request.language,
        });

        let value = self.send_json(&self.search_path, payload, "search")?;
        Ok(map_search_response(value, self.name()))
    }

    fn extract(&self, request: &ExtractRequest) -> GatewayResult<ExtractResponse> {
        let payload = json!({
            "urls": request.urls,
            "include_images": request.include_images.unwrap_or(false),
        });
        let value = self.send_json(&self.extract_path, payload, "extract")?;
        Ok(map_extract_response(value, self.name()))
    }

    fn crawl(&self, request: &CrawlRequest) -> GatewayResult<CrawlResponse> {
        let payload = json!({
            "url": request.url,
            "limit": request.limit.unwrap_or(10),
            "max_depth": request.max_depth.unwrap_or(2),
            "instructions": request.instructions,
        });
        let value = self.send_json(&self.crawl_path, payload, "crawl")?;
        Ok(map_crawl_response(value, self.name()))
    }
}
