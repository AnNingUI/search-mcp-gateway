use crate::{
    domain::{
        errors::{GatewayError, GatewayResult},
        models::{CrawlRequest, ExtractRequest, OutputMode, SearchRequest, ToolEnvelope},
    },
    gateway::service::GatewayService,
    mcp::server::run_stdio_server,
};

pub fn handle_search(
    gateway: GatewayService,
    output_mode: OutputMode,
    request: SearchRequest,
) -> GatewayResult<()> {
    let response = gateway.search(request)?;
    print_output(output_mode, &response)
}

pub fn handle_extract(
    gateway: GatewayService,
    output_mode: OutputMode,
    request: ExtractRequest,
) -> GatewayResult<()> {
    let response = gateway.extract(request)?;
    print_output(output_mode, &response)
}

pub fn handle_crawl(
    gateway: GatewayService,
    output_mode: OutputMode,
    request: CrawlRequest,
) -> GatewayResult<()> {
    let response = gateway.crawl(request)?;
    print_output(output_mode, &response)
}

pub fn handle_status(gateway: GatewayService, output_mode: OutputMode) -> GatewayResult<()> {
    let response = gateway.status();
    print_output(output_mode, &response)
}

pub fn handle_mcp(gateway: GatewayService) -> GatewayResult<()> {
    run_stdio_server(gateway)
}

fn print_output<T: serde::Serialize>(output_mode: OutputMode, value: &T) -> GatewayResult<()> {
    match output_mode {
        OutputMode::Json => {
            let json = serde_json::to_string_pretty(&ToolEnvelope::success(value))
                .map_err(|error| GatewayError::serialization(error.to_string()))?;
            println!("{json}");
        }
        OutputMode::Human => {
            let json = serde_json::to_string_pretty(value)
                .map_err(|error| GatewayError::serialization(error.to_string()))?;
            println!("{json}");
        }
    }
    Ok(())
}
