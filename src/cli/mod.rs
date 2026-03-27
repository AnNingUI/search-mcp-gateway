pub mod app;
pub mod commands;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::{
    cli::app::{handle_crawl, handle_extract, handle_mcp, handle_search, handle_status},
    domain::{
        errors::GatewayError,
        models::{
            CrawlRequest, ExtractRequest, OutputMode, SearchDepth, SearchRequest, SearchTopic,
            ToolEnvelope,
        },
    },
    gateway::service::GatewayService,
    infra::{config::AppConfig, telemetry},
};

#[derive(Debug, Parser)]
#[command(name = "search-mcp-gateway")]
#[command(about = "Provider-agnostic search gateway with MCP and CLI frontends")]
pub struct Cli {
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Mcp,
    Search {
        #[arg(long)]
        query: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        topic: Option<SearchTopic>,
        #[arg(long)]
        max_results: Option<u32>,
        #[arg(long)]
        search_depth: Option<SearchDepth>,
        #[arg(long)]
        include_answer: bool,
        #[arg(long)]
        include_raw_content: bool,
        #[arg(long)]
        include_images: bool,
        #[arg(long)]
        days: Option<u32>,
        #[arg(long)]
        site_filter: Vec<String>,
        #[arg(long)]
        exclude_domain: Vec<String>,
        #[arg(long)]
        country: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
    },
    Extract {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        include_images: bool,
        #[arg(long)]
        timeout_ms: Option<u64>,
        urls: Vec<String>,
    },
    Crawl {
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        max_depth: Option<u32>,
        #[arg(long)]
        instructions: Option<String>,
        #[arg(long)]
        timeout_ms: Option<u64>,
        url: String,
    },
    Status,
}

pub fn run() -> i32 {
    telemetry::init();

    let cli = Cli::parse();
    let output_mode = if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    };

    let config = match AppConfig::load(cli.config) {
        Ok(config) => config,
        Err(error) => {
            emit_error(&output_mode, &error);
            return 1;
        }
    };
    let gateway = match GatewayService::from_config(config) {
        Ok(gateway) => gateway,
        Err(error) => {
            emit_error(&output_mode, &error);
            return 1;
        }
    };

    let result = match cli.command {
        Commands::Mcp => handle_mcp(gateway),
        Commands::Search {
            query,
            provider,
            topic,
            max_results,
            search_depth,
            include_answer,
            include_raw_content,
            include_images,
            days,
            site_filter,
            exclude_domain,
            country,
            language,
            timeout_ms,
        } => handle_search(
            gateway,
            output_mode,
            SearchRequest {
                query,
                provider,
                topic,
                max_results,
                search_depth,
                include_answer: Some(include_answer),
                include_raw_content: Some(include_raw_content),
                include_images: Some(include_images),
                days,
                site_filter: (!site_filter.is_empty()).then_some(site_filter),
                exclude_domains: (!exclude_domain.is_empty()).then_some(exclude_domain),
                country,
                language,
                timeout_ms,
            },
        ),
        Commands::Extract {
            provider,
            include_images,
            timeout_ms,
            urls,
        } => handle_extract(
            gateway,
            output_mode,
            ExtractRequest {
                urls,
                provider,
                include_images: Some(include_images),
                timeout_ms,
            },
        ),
        Commands::Crawl {
            provider,
            limit,
            max_depth,
            instructions,
            timeout_ms,
            url,
        } => handle_crawl(
            gateway,
            output_mode,
            CrawlRequest {
                url,
                provider,
                limit,
                max_depth,
                instructions,
                timeout_ms,
            },
        ),
        Commands::Status => handle_status(gateway, output_mode),
    };

    match result {
        Ok(()) => 0,
        Err(error) => {
            emit_error(&output_mode, &error);
            1
        }
    }
}

fn emit_error(output_mode: &OutputMode, error: &GatewayError) {
    match output_mode {
        OutputMode::Json => {
            match serde_json::to_string_pretty(&ToolEnvelope::<()>::failure(error.clone())) {
                Ok(json) => eprintln!("{json}"),
                Err(_) => eprintln!("{error}"),
            }
        }
        OutputMode::Human => {
            eprintln!("{error}");

            if let Some(attempted) = &error.attempted_providers {
                if !attempted.is_empty() {
                    eprintln!("attempted providers: {}", attempted.join(" -> "));
                }
            }

            if let Some(failures) = &error.fallback_failures {
                for failure in failures {
                    eprintln!(
                        "{} [{}]: {}",
                        failure.provider, failure.code, failure.message
                    );
                }
            }
        }
    }
}
