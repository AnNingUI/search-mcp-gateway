use std::collections::HashSet;

use reqwest::{
    blocking::Client,
    header::{ACCEPT, ACCEPT_LANGUAGE, ORIGIN, REFERER},
};
use scraper::{Html, Selector};

use crate::{
    domain::{
        errors::{GatewayError, GatewayResult},
        models::{SearchRequest, SearchResponse, SearchResultItem},
        provider::{Provider, ProviderCapabilities},
    },
    infra::{config::AppConfig, http::build_http_client},
    providers::ddg::mapper::{build_result, decode_ddg_target},
};

pub struct DuckDuckGoProvider {
    client: Client,
    base_url: String,
    lite_url: String,
    region: String,
    safe_search: String,
}

#[derive(Clone, Copy)]
enum DdgRequestMethod {
    Get,
    Post,
}

#[derive(Clone, Copy)]
enum DdgResponseLayout {
    Html,
    Lite,
}

struct DdgSearchStrategy<'a> {
    name: &'a str,
    url: &'a str,
    method: DdgRequestMethod,
    layout: DdgResponseLayout,
}

impl DuckDuckGoProvider {
    pub fn from_config(config: &AppConfig) -> GatewayResult<Self> {
        let client = build_http_client(config.gateway.default_timeout_ms, &config.ddg.user_agent)?;
        Ok(Self {
            client,
            base_url: config.ddg.base_url.clone(),
            lite_url: config.ddg.lite_url.clone(),
            region: config.ddg.region.clone(),
            safe_search: config.ddg.safe_search.clone(),
        })
    }

    fn search_strategies(&self) -> [DdgSearchStrategy<'_>; 4] {
        [
            DdgSearchStrategy {
                name: "html_post",
                url: &self.base_url,
                method: DdgRequestMethod::Post,
                layout: DdgResponseLayout::Html,
            },
            DdgSearchStrategy {
                name: "html_get",
                url: &self.base_url,
                method: DdgRequestMethod::Get,
                layout: DdgResponseLayout::Html,
            },
            DdgSearchStrategy {
                name: "lite_post",
                url: &self.lite_url,
                method: DdgRequestMethod::Post,
                layout: DdgResponseLayout::Lite,
            },
            DdgSearchStrategy {
                name: "lite_get",
                url: &self.lite_url,
                method: DdgRequestMethod::Get,
                layout: DdgResponseLayout::Lite,
            },
        ]
    }

    fn execute_strategy(
        &self,
        strategy: &DdgSearchStrategy<'_>,
        query: &str,
        kp: &str,
        max_results: usize,
    ) -> GatewayResult<Vec<SearchResultItem>> {
        let response = match strategy.method {
            DdgRequestMethod::Get => self
                .client
                .get(strategy.url)
                .header(
                    ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                .header(REFERER, "https://duckduckgo.com/")
                .query(&[("q", query), ("kl", self.region.as_str()), ("kp", kp)])
                .send(),
            DdgRequestMethod::Post => self
                .client
                .post(strategy.url)
                .header(
                    ACCEPT,
                    "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
                )
                .header(ACCEPT_LANGUAGE, "en-US,en;q=0.9")
                .header(REFERER, "https://duckduckgo.com/")
                .header(ORIGIN, "https://duckduckgo.com")
                .form(&[("q", query), ("kl", self.region.as_str()), ("kp", kp)])
                .send(),
        }
        .map_err(|error| {
            GatewayError::provider(
                self.name(),
                "search_transport_error",
                format!("ddg {} request failed: {error}", strategy.name),
                true,
            )
        })?;

        let status = response.status();
        let body = response.text().map_err(|error| {
            GatewayError::provider(
                self.name(),
                "search_read_error",
                format!("failed to read ddg {} response: {error}", strategy.name),
                true,
            )
        })?;

        if !status.is_success() {
            return Err(GatewayError::provider(
                self.name(),
                "search_http_error",
                format!("ddg {} returned {status}: {body}", strategy.name),
                status.is_server_error() || status.as_u16() == 429,
            ));
        }

        self.parse_results(&body, strategy.layout, max_results)
    }

    fn parse_results(
        &self,
        body: &str,
        layout: DdgResponseLayout,
        max_results: usize,
    ) -> GatewayResult<Vec<SearchResultItem>> {
        let mut results = match layout {
            DdgResponseLayout::Html => parse_html_results(body, max_results)?,
            DdgResponseLayout::Lite => Vec::new(),
        };

        if results.is_empty() {
            results = parse_generic_anchor_results(body, max_results)?;
        }

        Ok(results)
    }
}

impl Provider for DuckDuckGoProvider {
    fn name(&self) -> &'static str {
        "ddg"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            search: true,
            extract: false,
            crawl: false,
        }
    }

    fn search(&self, request: &SearchRequest) -> GatewayResult<SearchResponse> {
        let kp = match self.safe_search.as_str() {
            "off" => "-2",
            "strict" => "1",
            _ => "-1",
        };

        let max_results = request.max_results.unwrap_or(5) as usize;
        let mut strategy_failures = Vec::new();
        let mut last_error = None;
        let mut results = Vec::new();

        for strategy in self.search_strategies() {
            match self.execute_strategy(&strategy, request.query.as_str(), kp, max_results) {
                Ok(found) => {
                    results = found;
                    break;
                }
                Err(error) => {
                    strategy_failures.push(format!("{}: {}", strategy.name, error.message));
                    last_error = Some(error);
                }
            }
        }

        if results.is_empty() && last_error.is_some() {
            return Err(GatewayError::provider(
                self.name(),
                "search_transport_error",
                format!(
                    "ddg search failed across strategies: {}",
                    strategy_failures.join(" | ")
                ),
                true,
            ));
        }

        Ok(SearchResponse {
            provider_used: self.name().to_string(),
            fallback_chain: Vec::new(),
            latency_ms: 0,
            warnings: Vec::new(),
            request_id: None,
            answer: None,
            follow_up_questions: None,
            results,
            images: Vec::new(),
        })
    }
}

fn parse_html_results(body: &str, max_results: usize) -> GatewayResult<Vec<SearchResultItem>> {
    let document = Html::parse_document(body);
    let result_selector =
        Selector::parse(".result").map_err(|error| GatewayError::gateway(error.to_string()))?;
    let title_selector = Selector::parse(".result__title a")
        .map_err(|error| GatewayError::gateway(error.to_string()))?;
    let snippet_selector = Selector::parse(".result__snippet")
        .map_err(|error| GatewayError::gateway(error.to_string()))?;

    let mut results = Vec::new();

    for result in document.select(&result_selector).take(max_results) {
        let Some(anchor) = result.select(&title_selector).next() else {
            continue;
        };
        let title = anchor.text().collect::<String>().trim().to_string();
        let href = anchor.value().attr("href").unwrap_or_default().to_string();
        let url = decode_ddg_target(&href);
        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|node| node.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if title.is_empty() || url.is_empty() || !is_result_link(&href, &url) {
            continue;
        }

        results.push(build_result(title, url, snippet));
    }

    Ok(results)
}

fn parse_generic_anchor_results(
    body: &str,
    max_results: usize,
) -> GatewayResult<Vec<SearchResultItem>> {
    let document = Html::parse_document(body);
    let anchor_selector =
        Selector::parse("a").map_err(|error| GatewayError::gateway(error.to_string()))?;
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for anchor in document.select(&anchor_selector) {
        let href = anchor.value().attr("href").unwrap_or_default();
        let url = decode_ddg_target(href);
        let title = anchor.text().collect::<String>().trim().to_string();

        if title.is_empty() || url.is_empty() || !is_result_link(href, &url) {
            continue;
        }

        if !seen.insert(url.clone()) {
            continue;
        }

        results.push(build_result(title, url, String::new()));

        if results.len() >= max_results {
            break;
        }
    }

    Ok(results)
}

fn is_result_link(href: &str, url: &str) -> bool {
    let href = href.to_ascii_lowercase();
    let url = url.to_ascii_lowercase();

    if url.starts_with("javascript:") || url.starts_with("mailto:") {
        return false;
    }

    if url.contains("duckduckgo.com") && !href.contains("uddg=") {
        return false;
    }

    url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::{is_result_link, parse_generic_anchor_results, parse_html_results};

    #[test]
    fn parses_html_result_cards() {
        let html = r#"
        <div class="result">
          <div class="result__title">
            <a href="/l/?uddg=https%3A%2F%2Fexample.com%2Farticle">Example Article</a>
          </div>
          <a class="result__snippet">Example snippet</a>
        </div>
        "#;

        let results = parse_html_results(html, 5).expect("html parser should succeed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example Article");
        assert_eq!(results[0].url, "https://example.com/article");
        assert_eq!(results[0].snippet, "Example snippet");
    }

    #[test]
    fn generic_parser_extracts_direct_links() {
        let html = r#"
        <html><body>
          <a href="https://example.com/one">Example One</a>
          <a href="https://duckduckgo.com/settings">Settings</a>
          <a href="/l/?uddg=https%3A%2F%2Fexample.com%2Ftwo">Example Two</a>
        </body></html>
        "#;

        let results = parse_generic_anchor_results(html, 10).expect("generic parser should work");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].url, "https://example.com/one");
        assert_eq!(results[1].url, "https://example.com/two");
    }

    #[test]
    fn filters_non_result_links() {
        assert!(is_result_link(
            "/l/?uddg=https%3A%2F%2Fexample.com",
            "https://example.com"
        ));
        assert!(!is_result_link(
            "https://duckduckgo.com/settings",
            "https://duckduckgo.com/settings"
        ));
    }
}
