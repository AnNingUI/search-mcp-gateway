use std::sync::Arc;

use crate::{
    domain::{
        models::{CrawlRequest, ExtractRequest, SearchDepth, SearchRequest},
        provider::Provider,
    },
    gateway::health::HealthStore,
    infra::config::GatewayConfig,
};

pub fn rank_search_candidates(
    config: &GatewayConfig,
    health: &HealthStore,
    request: &SearchRequest,
    providers: Vec<Arc<dyn Provider>>,
) -> Vec<Arc<dyn Provider>> {
    let mut providers = providers;
    providers.sort_by(|left, right| {
        let left_key = search_rank_key(config, health, request, left.as_ref());
        let right_key = search_rank_key(config, health, request, right.as_ref());
        left_key.cmp(&right_key)
    });
    providers
}

fn search_rank_key<'a>(
    config: &GatewayConfig,
    health: &HealthStore,
    request: &SearchRequest,
    provider: &'a dyn Provider,
) -> (u32, u8, usize, u32, &'a str) {
    let name = provider.name();
    let rich_search_penalty = if prefer_rich_search_provider(request, name) {
        0
    } else {
        1
    };
    let open_penalty = if health.is_available(name) { 0 } else { 1000 };
    let health_penalty = health.penalty(name);
    let static_order = static_search_rank(config, name);
    (
        open_penalty,
        rich_search_penalty,
        static_order,
        health_penalty,
        name,
    )
}

pub fn rank_extract_candidates(
    health: &HealthStore,
    providers: Vec<Arc<dyn Provider>>,
    _request: &ExtractRequest,
) -> Vec<Arc<dyn Provider>> {
    rank_capability_only(health, providers)
}

pub fn rank_crawl_candidates(
    health: &HealthStore,
    providers: Vec<Arc<dyn Provider>>,
    _request: &CrawlRequest,
) -> Vec<Arc<dyn Provider>> {
    rank_capability_only(health, providers)
}

fn rank_capability_only(
    health: &HealthStore,
    mut providers: Vec<Arc<dyn Provider>>,
) -> Vec<Arc<dyn Provider>> {
    providers.sort_by_key(|provider| {
        let name = provider.name();
        let open_penalty = if health.is_available(name) { 0 } else { 1000 };
        let health_penalty = health.penalty(name);
        (open_penalty, health_penalty, name)
    });
    providers
}

fn prefer_rich_search_provider(request: &SearchRequest, provider_name: &str) -> bool {
    provider_name == "tavily"
        && (request.include_answer.unwrap_or(true)
            || request.include_raw_content.unwrap_or(false)
            || request.include_images.unwrap_or(false)
            || request.search_depth == Some(SearchDepth::Advanced)
            || request.topic.is_some())
}

fn static_search_rank(config: &GatewayConfig, provider_name: &str) -> usize {
    config
        .search_provider_order
        .iter()
        .position(|name| name == provider_name)
        .unwrap_or(usize::MAX / 2)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        domain::{
            models::{SearchRequest, SearchTopic},
            provider::{Provider, ProviderCapabilities},
        },
        gateway::{health::HealthStore, ranking::rank_search_candidates},
        infra::config::GatewayConfig,
    };

    struct FakeProvider(&'static str);

    impl Provider for FakeProvider {
        fn name(&self) -> &'static str {
            self.0
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                search: true,
                extract: false,
                crawl: false,
            }
        }
    }

    fn request() -> SearchRequest {
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

    fn names(providers: Vec<Arc<dyn Provider>>) -> Vec<&'static str> {
        providers
            .into_iter()
            .map(|provider| provider.name())
            .collect()
    }

    #[test]
    fn default_search_prefers_tavily_for_rich_results() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(FakeProvider("ddg")),
            Arc::new(FakeProvider("tavily")),
        ];

        let ranked =
            rank_search_candidates(&GatewayConfig::default(), &health, &request(), providers);

        assert_eq!(names(ranked), vec!["tavily", "ddg"]);
    }

    #[test]
    fn config_order_still_applies_when_rich_search_preference_is_not_needed() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(FakeProvider("tavily")),
            Arc::new(FakeProvider("ddg")),
        ];

        let mut config = GatewayConfig::default();
        config.search_provider_order = vec!["ddg".to_string(), "tavily".to_string()];

        let mut request = request();
        request.include_answer = Some(false);

        let ranked = rank_search_candidates(&config, &health, &request, providers);

        assert_eq!(names(ranked), vec!["ddg", "tavily"]);
    }

    #[test]
    fn topic_queries_prefer_tavily_even_if_include_answer_is_disabled() {
        let health = HealthStore::new(3, std::time::Duration::from_secs(30));
        let providers: Vec<Arc<dyn Provider>> = vec![
            Arc::new(FakeProvider("ddg")),
            Arc::new(FakeProvider("tavily")),
        ];

        let mut request = request();
        request.include_answer = Some(false);
        request.topic = Some(SearchTopic::News);

        let ranked =
            rank_search_candidates(&GatewayConfig::default(), &health, &request, providers);

        assert_eq!(names(ranked), vec!["tavily", "ddg"]);
    }
}
