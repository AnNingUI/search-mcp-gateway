use std::sync::Arc;

use crate::{
    domain::{
        errors::{GatewayError, GatewayResult},
        models::{CrawlRequest, ExtractRequest, SearchRequest},
        provider::Provider,
    },
    gateway::{
        health::HealthStore,
        ranking::{rank_crawl_candidates, rank_extract_candidates, rank_search_candidates},
    },
    infra::config::GatewayConfig,
    providers::registry::ProviderRegistry,
};

pub fn select_search_candidates(
    config: &GatewayConfig,
    registry: &ProviderRegistry,
    health: &HealthStore,
    request: &SearchRequest,
) -> GatewayResult<Vec<Arc<dyn Provider>>> {
    if let Some(provider_name) = &request.provider {
        return registry
            .get(provider_name)
            .map(|provider| vec![provider])
            .ok_or_else(|| GatewayError::validation(format!("unknown provider: {provider_name}")));
    }

    let candidates = rank_search_candidates(config, health, request, registry.search_providers());
    if candidates.is_empty() {
        return Err(GatewayError::gateway("no search providers are enabled"));
    }
    Ok(candidates)
}

pub fn select_extract_candidates(
    registry: &ProviderRegistry,
    health: &HealthStore,
    request: &ExtractRequest,
) -> GatewayResult<Vec<Arc<dyn Provider>>> {
    if let Some(provider_name) = &request.provider {
        return registry
            .get(provider_name)
            .map(|provider| vec![provider])
            .ok_or_else(|| GatewayError::validation(format!("unknown provider: {provider_name}")));
    }

    let candidates = rank_extract_candidates(health, registry.extract_providers(), request);
    if candidates.is_empty() {
        return Err(GatewayError::gateway("no extract providers are enabled"));
    }
    Ok(candidates)
}

pub fn select_crawl_candidates(
    registry: &ProviderRegistry,
    health: &HealthStore,
    request: &CrawlRequest,
) -> GatewayResult<Vec<Arc<dyn Provider>>> {
    if let Some(provider_name) = &request.provider {
        return registry
            .get(provider_name)
            .map(|provider| vec![provider])
            .ok_or_else(|| GatewayError::validation(format!("unknown provider: {provider_name}")));
    }

    let candidates = rank_crawl_candidates(health, registry.crawl_providers(), request);
    if candidates.is_empty() {
        return Err(GatewayError::gateway("no crawl providers are enabled"));
    }
    Ok(candidates)
}
