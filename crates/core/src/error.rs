//! Error types for translation services.

use thiserror::Error;

/// Result alias used by translation services.
pub type ServiceResult<T> = Result<T, ServiceError>;

/// Errors a translation service can return.
#[derive(Debug, Error)]
pub enum ServiceError {
    /// Required credentials are missing in the active config.
    #[error("missing credentials: {0}")]
    MissingCredentials(String),

    /// Network / HTTP error (DNS, TCP, TLS, etc.).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Upstream service returned an error response.
    #[error("api error [{code}]: {message}")]
    Api {
        /// Provider-specific error code.
        code: String,
        /// Human-readable error message.
        message: String,
    },

    /// Failed to parse a valid response from the upstream service.
    #[error("invalid response: {0}")]
    Parse(String),

    /// Provider throttled the caller. `retry_after_ms` is best-effort.
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited {
        /// Suggested wait in milliseconds.
        retry_after_ms: u64,
    },

    /// The request exceeded the per-service timeout.
    #[error("timeout after {elapsed_ms}ms")]
    Timeout {
        /// Actual elapsed time in milliseconds.
        elapsed_ms: u64,
    },

    /// Caller cancelled the request.
    #[error("cancelled")]
    Cancelled,
}

impl ServiceError {
    /// Convert into a stable error code for the UI / logs.
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingCredentials(_) => "missing_credentials",
            Self::Network(_) => "network",
            Self::Api { .. } => "api",
            Self::Parse(_) => "parse",
            Self::RateLimited { .. } => "rate_limited",
            Self::Timeout { .. } => "timeout",
            Self::Cancelled => "cancelled",
        }
    }
}
