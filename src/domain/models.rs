use clap::ValueEnum;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::domain::errors::GatewayError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchTopic {
    General,
    News,
}

impl SearchTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::News => "news",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchDepth {
    Basic,
    Advanced,
}

impl SearchDepth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Human,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchRequest {
    pub query: String,
    pub provider: Option<String>,
    pub topic: Option<SearchTopic>,
    pub max_results: Option<u32>,
    pub search_depth: Option<SearchDepth>,
    pub include_answer: Option<bool>,
    pub include_raw_content: Option<bool>,
    pub include_images: Option<bool>,
    pub days: Option<u32>,
    pub site_filter: Option<Vec<String>>,
    pub exclude_domains: Option<Vec<String>>,
    pub country: Option<String>,
    pub language: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl SearchRequest {
    pub fn validate(&self) -> Result<(), GatewayError> {
        if self.query.trim().is_empty() {
            return Err(GatewayError::validation("query must not be empty"));
        }
        if matches!(self.max_results, Some(0)) {
            return Err(GatewayError::validation(
                "max_results must be greater than 0",
            ));
        }
        Ok(())
    }

    pub fn cache_key(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.query.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtractRequest {
    pub urls: Vec<String>,
    pub provider: Option<String>,
    pub include_images: Option<bool>,
    pub timeout_ms: Option<u64>,
}

impl ExtractRequest {
    pub fn validate(&self) -> Result<(), GatewayError> {
        if self.urls.is_empty() {
            return Err(GatewayError::validation("urls must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrawlRequest {
    pub url: String,
    pub provider: Option<String>,
    pub limit: Option<u32>,
    pub max_depth: Option<u32>,
    pub instructions: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl CrawlRequest {
    pub fn validate(&self) -> Result<(), GatewayError> {
        if self.url.trim().is_empty() {
            return Err(GatewayError::validation("url must not be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: Option<String>,
    pub score: Option<f64>,
    pub published_at: Option<String>,
    pub source_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageResult {
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub provider_used: String,
    pub fallback_chain: Vec<String>,
    pub latency_ms: u128,
    pub warnings: Vec<String>,
    pub request_id: Option<String>,
    pub answer: Option<String>,
    pub follow_up_questions: Option<Vec<String>>,
    pub results: Vec<SearchResultItem>,
    pub images: Vec<ImageResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub images: Vec<ImageResult>,
    pub source_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResponse {
    pub provider_used: String,
    pub fallback_chain: Vec<String>,
    pub latency_ms: u128,
    pub warnings: Vec<String>,
    pub documents: Vec<ExtractedDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPage {
    pub url: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub source_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResponse {
    pub provider_used: String,
    pub fallback_chain: Vec<String>,
    pub latency_ms: u128,
    pub warnings: Vec<String>,
    pub pages: Vec<CrawledPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEnvelope<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<crate::domain::errors::GatewayError>,
}

impl<T> ToolEnvelope<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: crate::domain::errors::GatewayError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}
