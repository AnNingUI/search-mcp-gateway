use url::Url;

use crate::domain::models::SearchResultItem;

pub fn decode_ddg_target(raw: &str) -> String {
    if let Ok(parsed) = Url::parse(&format!("https://duckduckgo.com{raw}")) {
        if let Some(target) = parsed
            .query_pairs()
            .find(|(key, _)| key == "uddg")
            .map(|(_, value)| value.to_string())
        {
            return target;
        }
    }
    raw.to_string()
}

pub fn build_result(title: String, url: String, snippet: String) -> SearchResultItem {
    SearchResultItem {
        title,
        url,
        snippet,
        content: None,
        score: None,
        published_at: None,
        source_provider: "ddg".to_string(),
    }
}
