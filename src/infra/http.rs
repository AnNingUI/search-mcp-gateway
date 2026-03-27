use std::time::Duration;

use reqwest::blocking::Client;

use crate::domain::errors::{GatewayError, GatewayResult};

pub fn build_http_client(timeout_ms: u64, user_agent: &str) -> GatewayResult<Client> {
    Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent(user_agent)
        .build()
        .map_err(|error| GatewayError::config(format!("failed to build HTTP client: {error}")))
}
