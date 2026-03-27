use std::sync::Arc;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::{runtime::Builder, task};

use crate::{
    domain::{
        errors::{GatewayError, GatewayResult},
        models::{CrawlRequest, ExtractRequest, SearchRequest, ToolEnvelope},
    },
    gateway::service::GatewayService,
};

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[serde(default)]
struct ProviderStatusRequest {}

#[derive(Clone)]
struct SearchGatewayMcpServer {
    gateway: Arc<GatewayService>,
    tool_router: ToolRouter<Self>,
}

impl SearchGatewayMcpServer {
    fn new(gateway: GatewayService) -> Self {
        Self {
            gateway: Arc::new(gateway),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl SearchGatewayMcpServer {
    #[tool(
        description = "Search the web through the gateway. Supports automatic provider selection, explicit provider override, and fallback reporting."
    )]
    async fn search_web(
        &self,
        Parameters(request): Parameters<SearchRequest>,
    ) -> Result<String, String> {
        let gateway = Arc::clone(&self.gateway);
        run_blocking(move || gateway.search(request)).await
    }

    #[tool(description = "Extract document content from one or more URLs through the gateway.")]
    async fn extract_web(
        &self,
        Parameters(request): Parameters<ExtractRequest>,
    ) -> Result<String, String> {
        let gateway = Arc::clone(&self.gateway);
        run_blocking(move || gateway.extract(request)).await
    }

    #[tool(
        description = "Crawl or map site pages through the gateway with provider fallback when available."
    )]
    async fn crawl_map(
        &self,
        Parameters(request): Parameters<CrawlRequest>,
    ) -> Result<String, String> {
        let gateway = Arc::clone(&self.gateway);
        run_blocking(move || gateway.crawl(request)).await
    }

    #[tool(description = "Inspect provider capabilities, health, and circuit-breaker status.")]
    async fn provider_status(
        &self,
        Parameters(_request): Parameters<ProviderStatusRequest>,
    ) -> Result<String, String> {
        let gateway = Arc::clone(&self.gateway);
        run_blocking(move || Ok(gateway.status())).await
    }
}

#[tool_handler]
impl ServerHandler for SearchGatewayMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Provider-agnostic search gateway exposing compact MCP tools for search, extract, crawl, and provider health.".to_string(),
        )
    }
}

pub fn run_stdio_server(gateway: GatewayService) -> GatewayResult<()> {
    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| {
            GatewayError::transport(format!("failed to build tokio runtime: {error}"))
        })?;

    runtime.block_on(async move {
        let server = SearchGatewayMcpServer::new(gateway);
        let service = server.serve(stdio()).await.map_err(|error| {
            GatewayError::transport(format!("failed to start MCP server: {error}"))
        })?;
        service.waiting().await.map_err(|error| {
            GatewayError::transport(format!("MCP server exited with error: {error}"))
        })?;
        Ok(())
    })
}

async fn run_blocking<T, F>(operation: F) -> Result<String, String>
where
    T: serde::Serialize + Send + 'static,
    F: FnOnce() -> GatewayResult<T> + Send + 'static,
{
    let result = task::spawn_blocking(operation)
        .await
        .map_err(|error| GatewayError::transport(format!("blocking task join error: {error}")));
    let envelope = match result {
        Ok(Ok(value)) => ToolEnvelope::success(value),
        Ok(Err(error)) => ToolEnvelope::<T>::failure(error),
        Err(error) => ToolEnvelope::<T>::failure(error),
    };
    serde_json::to_string_pretty(&envelope).map_err(|error| error.to_string())
}
