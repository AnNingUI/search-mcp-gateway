use std::{collections::HashMap, sync::Arc};

use crate::{
    domain::{
        errors::GatewayResult,
        provider::{Provider, ProviderCapabilities},
    },
    infra::config::AppConfig,
    providers::{ddg::DuckDuckGoProvider, tavily::TavilyProvider},
};

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn from_config(config: &AppConfig) -> GatewayResult<Self> {
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();

        if config.tavily.enabled {
            let provider = Arc::new(TavilyProvider::from_config(config)?) as Arc<dyn Provider>;
            providers.insert(provider.name().to_string(), provider);
        }

        if config.ddg.enabled {
            let provider = Arc::new(DuckDuckGoProvider::from_config(config)?) as Arc<dyn Provider>;
            providers.insert(provider.name().to_string(), provider);
        }

        Ok(Self { providers })
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }

    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    pub fn search_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers_by(|caps| caps.search)
    }

    pub fn extract_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers_by(|caps| caps.extract)
    }

    pub fn crawl_providers(&self) -> Vec<Arc<dyn Provider>> {
        self.providers_by(|caps| caps.crawl)
    }

    pub fn providers_with_capabilities(&self) -> Vec<(String, ProviderCapabilities)> {
        self.providers
            .values()
            .map(|provider| (provider.name().to_string(), provider.capabilities()))
            .collect()
    }

    fn providers_by(
        &self,
        predicate: impl Fn(ProviderCapabilities) -> bool,
    ) -> Vec<Arc<dyn Provider>> {
        self.providers
            .values()
            .filter(|provider| predicate(provider.capabilities()))
            .cloned()
            .collect()
    }
}
