use crate::domain::{
    errors::GatewayResult,
    models::{
        CrawlRequest, CrawlResponse, ExtractRequest, ExtractResponse, SearchRequest, SearchResponse,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct ProviderCapabilities {
    pub search: bool,
    pub extract: bool,
    pub crawl: bool,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> ProviderCapabilities;

    fn search(&self, _request: &SearchRequest) -> GatewayResult<SearchResponse> {
        Err(crate::domain::errors::GatewayError::provider(
            self.name(),
            "unsupported_operation",
            "provider does not support search",
            false,
        ))
    }

    fn extract(&self, _request: &ExtractRequest) -> GatewayResult<ExtractResponse> {
        Err(crate::domain::errors::GatewayError::provider(
            self.name(),
            "unsupported_operation",
            "provider does not support extract",
            false,
        ))
    }

    fn crawl(&self, _request: &CrawlRequest) -> GatewayResult<CrawlResponse> {
        Err(crate::domain::errors::GatewayError::provider(
            self.name(),
            "unsupported_operation",
            "provider does not support crawl",
            false,
        ))
    }
}
