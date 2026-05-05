use std::sync::Arc;

use anyhow::anyhow;
use warp_core::errors::{register_error, AnyhowErrorExt, ErrorExt};

/// We use a special error code header `X-Warp-Error-Code` to allow the server to send
/// more specific error code information, so that the client can discern between different
/// errors with the same error code.
const WARP_ERROR_CODE_HEADER: &str = "X-Warp-Error-Code";

/// An error indicating the user is out of credits. The server sends 429s to communicate this
/// state, but if Cloud Run is overloaded, it can also send 429s that aren't credit-related.
const WARP_ERROR_CODE_OUT_OF_CREDITS: &str = "OUT_OF_CREDITS";

#[derive(thiserror::Error, Debug)]
pub enum DeserializationError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Transport(reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum AIApiError {
    #[error("Request failed due to lack of AI quota.")]
    QuotaLimit,

    #[error("Warp is currently overloaded. Please try again later.")]
    ServerOverloaded,

    #[error("Internal error occurred at transport layer.")]
    Transport(#[source] reqwest::Error),

    #[error("Failed to deserialize API response.")]
    Deserialization(#[source] DeserializationError),

    #[error("No context found on context search.")]
    NoContextFound,

    #[error("Failed with status code {0}: {1}")]
    ErrorStatus(http::StatusCode, String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("Got error when streaming {stream_type}: {source:#}")]
    Stream {
        stream_type: &'static str,
        #[source]
        source: anyhow::Error,
    },
}

impl From<http_client::ResponseError> for AIApiError {
    fn from(err: http_client::ResponseError) -> Self {
        Self::from_response_error(err.source, &err.headers)
    }
}

impl From<reqwest::Error> for AIApiError {
    fn from(err: reqwest::Error) -> Self {
        Self::from_transport_error(err)
    }
}

impl From<serde_json::Error> for AIApiError {
    fn from(err: serde_json::Error) -> Self {
        AIApiError::Deserialization(err.into())
    }
}

impl AIApiError {
    fn from_response_error(err: reqwest::Error, headers: &::http::HeaderMap) -> Self {
        if err.status() == Some(http::StatusCode::TOO_MANY_REQUESTS) {
            return Self::error_for_429(headers);
        }

        Self::from_transport_error(err)
    }

    fn from_transport_error(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return AIApiError::Transport(err);
        }
        if err.is_decode() {
            #[cfg(not(target_family = "wasm"))]
            {
                use std::error::Error as _;
                let mut source = err.source();
                while let Some(underlying) = source {
                    if underlying.is::<hyper::Error>() {
                        return AIApiError::Transport(err);
                    }

                    source = underlying.source();
                }
            }

            return AIApiError::Deserialization(DeserializationError::Transport(err));
        }

        AIApiError::Transport(err)
    }

    fn error_for_429(headers: &::http::HeaderMap) -> Self {
        if headers
            .get(WARP_ERROR_CODE_HEADER)
            .and_then(|v| v.to_str().ok())
            == Some(WARP_ERROR_CODE_OUT_OF_CREDITS)
        {
            AIApiError::QuotaLimit
        } else {
            AIApiError::ServerOverloaded
        }
    }

    pub fn stream_error(stream_type: &'static str, err: anyhow::Error) -> Self {
        AIApiError::Stream {
            stream_type,
            source: err,
        }
    }

    pub fn is_retryable(&self) -> bool {
        fn is_retryable_status(status: http::StatusCode) -> bool {
            !status.is_client_error()
                || status == http::StatusCode::REQUEST_TIMEOUT
                || status == http::StatusCode::TOO_MANY_REQUESTS
        }

        match self {
            AIApiError::ErrorStatus(status, _) => is_retryable_status(*status),
            AIApiError::Transport(e) => {
                if let Some(status) = e.status() {
                    return is_retryable_status(status);
                }
                true
            }
            _ => true,
        }
    }
}

impl ErrorExt for AIApiError {
    fn is_actionable(&self) -> bool {
        match self {
            AIApiError::Deserialization(_) => true,
            AIApiError::Transport(error) => error.is_actionable(),
            AIApiError::Other(error) => error.is_actionable(),
            AIApiError::Stream { source, .. } => source.is_actionable(),
            AIApiError::ErrorStatus(_, _) => self.is_retryable(),
            AIApiError::QuotaLimit | AIApiError::ServerOverloaded | AIApiError::NoContextFound => {
                false
            }
        }
    }
}
register_error!(AIApiError);

#[derive(thiserror::Error, Debug)]
pub enum TranscribeError {
    #[error("Request failed due to lack of Voice quota.")]
    QuotaLimit,

    #[error("Warp is currently overloaded. Please try again later.")]
    ServerOverloaded,

    #[error("Internal error occurred at transport layer.")]
    Transport,

    #[error("Failed to deserialize JSON.")]
    Deserialization,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl TranscribeError {
    pub fn unavailable() -> Self {
        Self::Other(anyhow!(
            "Voice transcription is unavailable in local-only Warper"
        ))
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        pub type AIOutputStream<T> = futures::stream::LocalBoxStream<'static, Result<T, Arc<AIApiError>>>;
    } else {
        pub type AIOutputStream<T> = futures::stream::BoxStream<'static, Result<T, Arc<AIApiError>>>;
    }
}
