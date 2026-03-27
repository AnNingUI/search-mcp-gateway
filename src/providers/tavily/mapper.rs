use serde_json::Value;

use crate::domain::models::{
    CrawlResponse, CrawledPage, ExtractResponse, ExtractedDocument, ImageResult, SearchResponse,
    SearchResultItem,
};

pub fn map_search_response(value: Value, provider_name: &str) -> SearchResponse {
    let answer = value
        .get("answer")
        .and_then(Value::as_str)
        .map(str::to_string);
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let follow_up_questions = value
        .get("follow_up_questions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
    let images = value
        .get("images")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|url| ImageResult {
                    url: url.to_string(),
                    description: None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let results = value
        .get("results")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| SearchResultItem {
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    url: item
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    snippet: item
                        .get("content")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    content: item
                        .get("raw_content")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    score: item.get("score").and_then(Value::as_f64),
                    published_at: item
                        .get("published_at")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    source_provider: provider_name.to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    SearchResponse {
        provider_used: provider_name.to_string(),
        fallback_chain: Vec::new(),
        latency_ms: 0,
        warnings: Vec::new(),
        request_id,
        answer,
        follow_up_questions,
        results,
        images,
    }
}

pub fn map_extract_response(value: Value, provider_name: &str) -> ExtractResponse {
    let documents = value
        .get("results")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| ExtractedDocument {
                    url: item
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    content: item
                        .get("raw_content")
                        .or_else(|| item.get("content"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    images: item
                        .get("images")
                        .and_then(Value::as_array)
                        .map(|images| {
                            images
                                .iter()
                                .filter_map(Value::as_str)
                                .map(|url| ImageResult {
                                    url: url.to_string(),
                                    description: None,
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                    source_provider: provider_name.to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ExtractResponse {
        provider_used: provider_name.to_string(),
        fallback_chain: Vec::new(),
        latency_ms: 0,
        warnings: Vec::new(),
        documents,
    }
}

pub fn map_crawl_response(value: Value, provider_name: &str) -> CrawlResponse {
    let pages = value
        .get("results")
        .or_else(|| value.get("pages"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| CrawledPage {
                    url: item
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    title: item
                        .get("title")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    summary: item
                        .get("summary")
                        .or_else(|| item.get("content"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    source_provider: provider_name.to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    CrawlResponse {
        provider_used: provider_name.to_string(),
        fallback_chain: Vec::new(),
        latency_ms: 0,
        warnings: Vec::new(),
        pages,
    }
}
