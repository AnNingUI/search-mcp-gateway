use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStage {
    Validation,
    Config,
    Provider,
    Gateway,
    Transport,
    Serialization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderFailure {
    pub provider: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub stage: ErrorStage,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[error("{code}: {message}")]
pub struct GatewayError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub provider: Option<String>,
    pub stage: ErrorStage,
    pub fallback_attempted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempted_providers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_failures: Option<Vec<ProviderFailure>>,
}

impl GatewayError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "validation_error".to_string(),
            message: message.into(),
            retryable: false,
            provider: None,
            stage: ErrorStage::Validation,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self {
            code: "config_error".to_string(),
            message: message.into(),
            retryable: false,
            provider: None,
            stage: ErrorStage::Config,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn provider(
        provider: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
            provider: Some(provider.into()),
            stage: ErrorStage::Provider,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn gateway(message: impl Into<String>) -> Self {
        Self {
            code: "gateway_error".to_string(),
            message: message.into(),
            retryable: false,
            provider: None,
            stage: ErrorStage::Gateway,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn transport(message: impl Into<String>) -> Self {
        Self {
            code: "transport_error".to_string(),
            message: message.into(),
            retryable: false,
            provider: None,
            stage: ErrorStage::Transport,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        Self {
            code: "serialization_error".to_string(),
            message: message.into(),
            retryable: false,
            provider: None,
            stage: ErrorStage::Serialization,
            fallback_attempted: false,
            attempted_providers: None,
            fallback_failures: None,
        }
    }

    pub fn with_fallback_attempted(mut self, attempted: bool) -> Self {
        self.fallback_attempted = attempted;
        self
    }

    pub fn with_fallback_context(
        mut self,
        attempted_providers: Vec<String>,
        fallback_failures: Vec<ProviderFailure>,
    ) -> Self {
        self.fallback_attempted = attempted_providers.len() > 1;
        self.attempted_providers = (!attempted_providers.is_empty()).then_some(attempted_providers);
        self.fallback_failures = (!fallback_failures.is_empty()).then_some(fallback_failures);
        self
    }

    pub fn as_provider_failure(&self) -> ProviderFailure {
        ProviderFailure {
            provider: self
                .provider
                .clone()
                .unwrap_or_else(|| "gateway".to_string()),
            code: self.code.clone(),
            message: self.message.clone(),
            retryable: self.retryable,
            stage: self.stage.clone(),
        }
    }
}

pub type GatewayResult<T> = Result<T, GatewayError>;
