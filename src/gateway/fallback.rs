use std::sync::Arc;

use crate::{
    domain::{
        errors::{GatewayError, GatewayResult, ProviderFailure},
        provider::Provider,
    },
    gateway::health::HealthStore,
};

pub fn with_provider_fallback<T>(
    health: &HealthStore,
    candidates: Vec<Arc<dyn Provider>>,
    mut operation: impl FnMut(&Arc<dyn Provider>) -> GatewayResult<T>,
) -> GatewayResult<(T, Vec<String>)> {
    let mut tried = Vec::new();
    let mut failures: Vec<ProviderFailure> = Vec::new();
    let mut last_error: Option<GatewayError> = None;

    for provider in candidates {
        let provider_name = provider.name().to_string();

        if !health.is_available(&provider_name) {
            continue;
        }

        tried.push(provider_name.clone());

        match operation(&provider) {
            Ok(value) => return Ok((value, tried)),
            Err(error) => {
                let retryable = error.retryable;
                let message = error.message.clone();
                health.record_failure(&provider_name, message);
                failures.push(error.as_provider_failure());
                last_error = Some(error);
                if !retryable {
                    break;
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| GatewayError::gateway("no providers could satisfy the request"))
        .with_fallback_context(tried, failures))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        domain::{
            errors::{GatewayError, GatewayResult},
            models::{SearchRequest, SearchResponse},
            provider::{Provider, ProviderCapabilities},
        },
        gateway::health::HealthStore,
    };

    use super::with_provider_fallback;

    struct SuccessProvider {
        name: &'static str,
    }

    impl Provider for SuccessProvider {
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

    struct FailProvider {
        name: &'static str,
        retryable: bool,
        code: &'static str,
        message: &'static str,
    }

    impl Provider for FailProvider {
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
            Err(GatewayError::provider(
                self.name,
                self.code,
                self.message,
                self.retryable,
            ))
        }
    }

    fn search_request() -> SearchRequest {
        SearchRequest {
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
        }
    }

    #[test]
    fn fallback_uses_second_provider_after_retryable_error() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let first: Arc<dyn Provider> = Arc::new(FailProvider {
            name: "first",
            retryable: true,
            code: "timeout",
            message: "temporary upstream timeout",
        });
        let second: Arc<dyn Provider> = Arc::new(SuccessProvider { name: "second" });

        let request = search_request();

        let (response, chain) = with_provider_fallback(&health, vec![first, second], |provider| {
            provider.search(&request)
        })
        .expect("fallback should succeed");

        assert_eq!(response.provider_used, "second");
        assert_eq!(chain, vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn fallback_error_includes_all_attempted_provider_failures() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let first: Arc<dyn Provider> = Arc::new(FailProvider {
            name: "tavily",
            retryable: true,
            code: "search_transport_error",
            message: "search request failed: connection reset",
        });
        let second: Arc<dyn Provider> = Arc::new(FailProvider {
            name: "ddg",
            retryable: true,
            code: "search_transport_error",
            message: "ddg request failed: timeout",
        });

        let error = with_provider_fallback(&health, vec![first, second], |provider| {
            provider.search(&search_request())
        })
        .expect_err("all providers should fail");

        assert!(error.fallback_attempted);
        assert_eq!(
            error.attempted_providers,
            Some(vec!["tavily".to_string(), "ddg".to_string()])
        );

        let failures = error
            .fallback_failures
            .expect("fallback failures should be attached");
        assert_eq!(failures.len(), 2);
        assert_eq!(failures[0].provider, "tavily");
        assert_eq!(failures[0].code, "search_transport_error");
        assert_eq!(failures[1].provider, "ddg");
        assert_eq!(failures[1].message, "ddg request failed: timeout");
    }
}
