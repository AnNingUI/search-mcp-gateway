use std::time::Duration;

use serde::Serialize;

use crate::{
    domain::{
        errors::GatewayResult,
        models::{
            CrawlRequest, CrawlResponse, ExtractRequest, ExtractResponse, SearchRequest,
            SearchResponse,
        },
    },
    gateway::{
        fallback::with_provider_fallback,
        health::{HealthSnapshot, HealthStore},
        strategy::{select_crawl_candidates, select_extract_candidates, select_search_candidates},
    },
    infra::{cache::TimedCache, config::AppConfig},
    providers::registry::ProviderRegistry,
};

#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    pub provider: String,
    pub search: bool,
    pub extract: bool,
    pub crawl: bool,
    pub enabled: bool,
    pub health: HealthSnapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub providers: Vec<ProviderStatus>,
}

pub struct GatewayService {
    config: AppConfig,
    registry: ProviderRegistry,
    health: HealthStore,
    search_cache: Option<TimedCache<SearchResponse>>,
}

impl GatewayService {
    pub fn from_config(config: AppConfig) -> GatewayResult<Self> {
        let registry = ProviderRegistry::from_config(&config)?;
        let health = HealthStore::new(
            config.gateway.circuit_failure_threshold,
            Duration::from_secs(config.gateway.circuit_open_seconds),
        );
        let search_cache = config
            .gateway
            .cache_enabled
            .then(|| TimedCache::new(config.gateway.cache_ttl()));

        Ok(Self {
            config,
            registry,
            health,
            search_cache,
        })
    }

    pub fn search(&self, request: SearchRequest) -> GatewayResult<SearchResponse> {
        request.validate()?;

        if let Some(cache) = &self.search_cache {
            if let Some(response) = cache.get(&request.cache_key()) {
                return Ok(response);
            }
        }

        let candidates =
            select_search_candidates(&self.config.gateway, &self.registry, &self.health, &request)?;

        let started = std::time::Instant::now();
        let (mut response, chain) = with_provider_fallback(&self.health, candidates, |provider| {
            provider.search(&request)
        })?;
        let elapsed = started.elapsed().as_millis();
        self.health.record_success(&response.provider_used, elapsed);
        response.latency_ms = elapsed;
        response.fallback_chain = chain;

        if let Some(cache) = &self.search_cache {
            cache.insert(request.cache_key(), response.clone());
        }

        Ok(response)
    }

    pub fn extract(&self, request: ExtractRequest) -> GatewayResult<ExtractResponse> {
        request.validate()?;
        let candidates = select_extract_candidates(&self.registry, &self.health, &request)?;
        let started = std::time::Instant::now();
        let (mut response, chain) = with_provider_fallback(&self.health, candidates, |provider| {
            provider.extract(&request)
        })?;
        let elapsed = started.elapsed().as_millis();
        self.health.record_success(&response.provider_used, elapsed);
        response.latency_ms = elapsed;
        response.fallback_chain = chain;
        Ok(response)
    }

    pub fn crawl(&self, request: CrawlRequest) -> GatewayResult<CrawlResponse> {
        request.validate()?;
        let candidates = select_crawl_candidates(&self.registry, &self.health, &request)?;
        let started = std::time::Instant::now();
        let (mut response, chain) = with_provider_fallback(&self.health, candidates, |provider| {
            provider.crawl(&request)
        })?;
        let elapsed = started.elapsed().as_millis();
        self.health.record_success(&response.provider_used, elapsed);
        response.latency_ms = elapsed;
        response.fallback_chain = chain;
        Ok(response)
    }

    pub fn status(&self) -> StatusResponse {
        let names = self.registry.names();
        let snapshots = self
            .health
            .snapshots(&names)
            .into_iter()
            .map(|snapshot| (snapshot.provider.clone(), snapshot))
            .collect::<std::collections::HashMap<_, _>>();

        let providers = self
            .registry
            .providers_with_capabilities()
            .into_iter()
            .map(|(provider, capabilities)| ProviderStatus {
                health: snapshots
                    .get(&provider)
                    .cloned()
                    .unwrap_or_else(|| HealthSnapshot {
                        provider: provider.clone(),
                        successes: 0,
                        failures: 0,
                        consecutive_failures: 0,
                        circuit_open: false,
                        circuit_remaining_ms: None,
                        last_error: None,
                        last_latency_ms: None,
                    }),
                provider,
                search: capabilities.search,
                extract: capabilities.extract,
                crawl: capabilities.crawl,
                enabled: true,
            })
            .collect();

        StatusResponse { providers }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        domain::{
            errors::{GatewayError, GatewayResult},
            models::{SearchRequest, SearchResponse},
            provider::{Provider, ProviderCapabilities},
        },
        gateway::{fallback::with_provider_fallback, health::HealthStore},
    };

    struct FakeProvider {
        name: &'static str,
        fail_once: Mutex<bool>,
    }

    impl Provider for FakeProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                search: true,
                extract: false,
                crawl: false,
            }
        }

        fn search(&self, _request: &SearchRequest) -> GatewayResult<SearchResponse> {
            let mut fail_once = self.fail_once.lock().expect("poisoned");
            if *fail_once {
                *fail_once = false;
                return Err(GatewayError::provider(
                    self.name,
                    "timeout",
                    "temporary upstream timeout",
                    true,
                ));
            }
            Ok(SearchResponse {
                provider_used: self.name.to_string(),
                fallback_chain: Vec::new(),
                latency_ms: 0,
                warnings: Vec::new(),
                request_id: None,
                answer: None,
                follow_up_questions: None,
                results: Vec::new(),
                images: Vec::new(),
            })
        }
    }

    #[test]
    fn fallback_uses_second_provider_after_retryable_error() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let first: Arc<dyn Provider> = Arc::new(FakeProvider {
            name: "first",
            fail_once: Mutex::new(true),
        });
        let second: Arc<dyn Provider> = Arc::new(FakeProvider {
            name: "second",
            fail_once: Mutex::new(false),
        });

        let request = SearchRequest {
            query: "test".to_string(),
            provider: None,
            topic: None,
            max_results: None,
            search_depth: None,
            include_answer: None,
            include_raw_content: None,
            include_images: None,
            days: None,
            site_filter: None,
            exclude_domains: None,
            country: None,
            language: None,
            timeout_ms: None,
        };

        let (response, chain) = with_provider_fallback(&health, vec![first, second], |provider| {
            provider.search(&request)
        })
        .expect("fallback should succeed");

        assert_eq!(response.provider_used, "second");
        assert_eq!(chain, vec!["first".to_string(), "second".to_string()]);
    }
}
