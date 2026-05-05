pub mod ai;
pub mod block;
pub mod integrations;

use crate::ai::get_relevant_files::api::{GetRelevantFiles, GetRelevantFilesResponse};
use crate::ai::predict::generate_ai_input_suggestions;
use crate::ai::predict::generate_ai_input_suggestions::GenerateAIInputSuggestionsRequest;
use crate::ai::predict::generate_am_query_suggestions;
use crate::ai::predict::generate_am_query_suggestions::GenerateAMQuerySuggestionsRequest;
use crate::ai::predict::predict_am_queries::{PredictAMQueriesRequest, PredictAMQueriesResponse};
use crate::ai::voice::transcribe::{TranscribeRequest, TranscribeResponse};
use crate::auth::auth_state::AuthState;
use ai::AIClient;
use warp_core::errors::{register_error, AnyhowErrorExt, ErrorExt};
use warpui::{r#async::BoxFuture, ModelContext};

use anyhow::{anyhow, Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use warpui::Entity;
use warpui::SingletonEntity;

pub const FETCH_CHANNEL_VERSIONS_TIMEOUT: std::time::Duration = Duration::from_secs(60);
/// We use a special error code header `X-Warp-Error-Code` to allow the server to send
/// more specific error code information, so that the client can discern between different
/// errors with the same error code.
/// See errors/http_error_codes.go on the server for possible values.
const WARP_ERROR_CODE_HEADER: &str = "X-Warp-Error-Code";

/// An error indicating the user is out of credits. The server sends 429s to communicate this
/// state, but if Cloud Run is overloaded, it can also send 429s that aren't credit-related.
/// So we use this to distinguish between the two cases.
const WARP_ERROR_CODE_OUT_OF_CREDITS: &str = "OUT_OF_CREDITS";

fn hosted_server_disabled() -> anyhow::Error {
    anyhow!("Warp-hosted server APIs are unavailable in Warper")
}

#[cfg(feature = "agent_mode_evals")]
pub const EVAL_USER_ID_HEADER: &str = "X-Eval-User-ID";

/// IDs in the staging database that were created specifically for evals.
/// These users have a clean state where they haven't been referred nor have referred anyone (which causes a popup in the client).
/// DO NOT REMOVE OR CHANGE THESE USERS!
///
/// Keep this list in sync with `script/populate_agent_mode_eval_user.sql`
/// in warp-server. Those rows need to exist in the DB so the authz user loader
/// can resolve these IDs during task creation; otherwise the server will 500
/// on every eval request with a nil-deref in `UserIDFromUser`.
#[cfg(feature = "agent_mode_evals")]
const EVAL_USER_IDS: [i32; 11] = [
    2162, 2164, 2165, 2166, 2167, 2168, 2169, 2172, 2173, 2174, 2175,
];

/// ResponseType received by Client
#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
#[error("{error}")]
pub struct ClientError {
    pub error: String,
    // We unconditionally check for GitHub auth errors in any public API response. It'd be much better
    // to have the server return error codes that we can parse, but this isn't yet supported.
    // See REMOTE-666
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_url: Option<String>,
}

/// Wrapper for deserialization errors. This covers both:
/// * Using `serde` directly
/// * Using `reqwest` decoding utilities
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
    /// Converts a reqwest error to an AIApiError, using response headers to distinguish
    /// between different types of 429 errors.
    fn from_response_error(err: reqwest::Error, headers: &::http::HeaderMap) -> Self {
        // For HTTP 429 errors, check the X-Warp-Error-Code header to distinguish
        // between out-of-credits and server-overload.
        if err.status() == Some(http::StatusCode::TOO_MANY_REQUESTS) {
            return Self::error_for_429(headers);
        }

        Self::from_transport_error(err)
    }

    /// Converts a transport-level reqwest error (no HTTP response) to an AIApiError.
    fn from_transport_error(err: reqwest::Error) -> Self {
        // Unfortunately, `reqwest` reports some non-decoding errors as decoding errors (e.g.
        // unexpected disconnects or timeouts while deserializing a response body). Since we
        // render deserialization and transport errors differently, we try to detect those cases
        // here.
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

    /// Returns the appropriate error for a 429 response by checking the X-Warp-Error-Code header.
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

    /// Format a stream error into a human-readable error message. This will read the response
    /// body if there is one.
    async fn from_stream_error(stream_type: &'static str, err: reqwest_eventsource::Error) -> Self {
        match err {
            reqwest_eventsource::Error::InvalidStatusCode(
                http::StatusCode::TOO_MANY_REQUESTS,
                ref res,
            ) => Self::error_for_429(res.headers()),
            reqwest_eventsource::Error::InvalidStatusCode(status, res) => Self::ErrorStatus(
                status,
                res.text()
                    .await
                    .unwrap_or_else(|e| format!("(no response body: {e:#})")),
            ),
            reqwest_eventsource::Error::Transport(err) => Self::from_transport_error(err),
            err => AIApiError::Stream {
                stream_type,
                // On WASM, `reqwest_eventsource::Error` doesn't implement `Into<anyhow::Error>` or
                // `Send` because it may contain a `wasm_bindgen` JS value.
                #[cfg(target_family = "wasm")]
                source: anyhow!("{err:#?}"),
                #[cfg(not(target_family = "wasm"))]
                source: anyhow!(err),
            },
        }
    }

    /// Returns whether or not the error can be retried.
    pub fn is_retryable(&self) -> bool {
        // Don't retry client errors, except for timeouts and quota limits.
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
            // By default, retry on error.
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

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        // The WASM version of this type has no bound on `Send`, which is not implemented on
        // `wasm_bindgen::JsValue`, which is ultimately used in reqwest_eventsource::Error. Furthermore,
        // `Send` is an unnecessary bound when targeting wasm because the browser is single-threaded (and
        // we don't leverage WebWorkers for async execution in WoW).
        pub type AIOutputStream<T> = futures::stream::LocalBoxStream<'static, Result<T, Arc<AIApiError>>>;
    } else {
        pub type AIOutputStream<T> = futures::stream::BoxStream<'static, Result<T, Arc<AIApiError>>>;
    }
}

/// An event related to the server API itself (and not a particular API call).
/// Most errors should be handled in callbacks to individual APIs, rather than sent over the
/// server API channel.
#[derive(Debug, Clone)]
pub enum ServerApiEvent {
    /// We made a staging API call that was blocked, which may indicate a firewall misconfiguration.
    StagingAccessBlocked,
    /// The user's access token was invalid, so they need to reauth before they can make
    /// requests to warp-server.
    NeedsReauth,
    /// The user's account has been disabled.
    UserAccountDisabled,
}

/// An API wrapper struct with methods to requests to warp-server.
///
/// Prefer NOT adding new methods directly on this struct; instead, add to one of the existing
/// client trait objects, or create your own. This helps keep `ServerApi` from being overloaded
/// with disparate types of calls, and allows you to mock methods in tests.
pub struct ServerApi {
    client: Arc<http_client::Client>,
    auth_state: Arc<AuthState>,
    event_sender: async_channel::Sender<ServerApiEvent>,
    #[cfg(feature = "agent_mode_evals")]
    eval_user_id: Option<i32>,
}

impl ServerApi {
    fn new(
        auth_state: Arc<AuthState>,
        event_sender: async_channel::Sender<ServerApiEvent>,
    ) -> Self {
        // We generate a random user ID for evals so we can run evals in parallel.
        #[cfg(feature = "agent_mode_evals")]
        let eval_user_id = {
            use rand::Rng;
            Some(EVAL_USER_IDS[rand::thread_rng().gen_range(0..EVAL_USER_IDS.len())])
        };

        Self {
            client: Arc::new(http_client::Client::new()),
            auth_state,
            event_sender,
            #[cfg(feature = "agent_mode_evals")]
            eval_user_id,
        }
    }

    /// Constructs a local-only compatibility API object for retained UI paths that
    /// still require a `ServerApi` handle. It is not registered globally and does
    /// not create a hosted provider.
    pub fn local_only(auth_state: Arc<AuthState>) -> Self {
        let (event_sender, _) = async_channel::bounded(1);
        Self::new(auth_state, event_sender)
    }

    #[cfg(test)]
    fn new_for_test() -> Self {
        let (tx, _) = async_channel::unbounded();

        Self {
            client: Arc::new(http_client::Client::new_for_test()),
            auth_state: Arc::new(AuthState::new_for_test()),
            event_sender: tx,
            #[cfg(feature = "agent_mode_evals")]
            eval_user_id: None,
        }
    }

    pub fn send_graphql_request<'a, QF, O: warp_graphql::client::Operation<QF> + Send + 'a>(
        &'a self,
        operation: O,
        timeout: Option<Duration>,
    ) -> BoxFuture<'a, Result<QF>> {
        let _ = timeout;
        Box::pin(async move {
            let _ = operation;
            Err(hosted_server_disabled())
        })
    }

    /// Sends a GET request to a public API endpoint.
    ///
    /// # Arguments
    /// * `path` - Endpoint path relative to `/api/v1` (e.g., "agent/tasks/{task_id}")
    async fn get_public_api<R>(&self, path: &str) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let response = self.get_public_api_response(path).await?;
        let url = response.url().clone();
        response
            .json::<R>()
            .await
            .with_context(|| format!("Failed to deserialize response from {url}"))
    }

    /// Sends a GET request to a public API endpoint and returns the raw response on success.
    ///
    /// Unlike [`get_public_api`], this does not attempt JSON deserialization on the
    /// response body, allowing the caller to decode it however they need.
    async fn get_public_api_response(&self, path: &str) -> Result<http_client::Response> {
        let _ = path;
        Err(hosted_server_disabled())
    }

    /// Opens an SSE stream to the agent event-push endpoint.
    ///
    /// The returned `EventSourceStream` yields `reqwest_eventsource::Event`
    /// items until the connection closes or an error occurs. The caller is
    /// responsible for reading the stream and handling reconnection.
    ///
    /// Hosted agent event streams are unavailable in Warper.
    pub async fn stream_agent_events(
        &self,
        run_ids: &[String],
        since_sequence: i64,
    ) -> Result<http_client::EventSourceStream> {
        let _ = (run_ids, since_sequence);
        Err(hosted_server_disabled())
    }

    /// Sends a POST request to a public API endpoint and returns the raw response on success.
    async fn post_public_api_response<B>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<http_client::Response>
    where
        B: Serialize,
    {
        let _ = (path, body);
        Err(hosted_server_disabled())
    }

    /// Converts a non-success public API response into the most specific client error available.
    async fn error_from_response(response: http_client::Response) -> anyhow::Error {
        let status = response.status();
        let is_out_of_credits = response
            .headers()
            .get(WARP_ERROR_CODE_HEADER)
            .and_then(|v| v.to_str().ok())
            == Some(WARP_ERROR_CODE_OUT_OF_CREDITS);

        // Get the response text first since we may need to try multiple deserializations.
        let response_text = response.text().await.unwrap_or_default();

        if status == StatusCode::TOO_MANY_REQUESTS && is_out_of_credits {
            return AIApiError::QuotaLimit.into();
        }

        // Try to deserialize error response as { "error": "message" }
        match serde_json::from_str::<ClientError>(&response_text) {
            Ok(error_response) => error_response.into(),
            Err(_) => anyhow!("API request failed with status {status}"),
        }
    }

    /// Sends a POST request to a public API endpoint.
    ///
    /// # Arguments
    /// * `path` - Endpoint path relative to `/api/v1` (e.g., "agent/run")
    /// * `body` - Request body to serialize as JSON
    async fn post_public_api<B, R>(&self, path: &str, body: &B) -> Result<R>
    where
        B: Serialize,
        R: serde::de::DeserializeOwned,
    {
        let response = self.post_public_api_response(path, body).await?;
        let url = response.url().clone();
        response
            .json::<R>()
            .await
            .with_context(|| format!("Failed to deserialize response from {url}"))
    }

    /// Sends a POST request to a public API endpoint that returns no response body.
    async fn post_public_api_unit<B>(&self, path: &str, body: &B) -> Result<()>
    where
        B: Serialize,
    {
        self.post_public_api_response(path, body).await?;
        Ok(())
    }

    /// Sends a PATCH request to a public API endpoint that returns no response body.
    async fn patch_public_api_unit<B>(&self, path: &str, body: &B) -> Result<()>
    where
        B: Serialize,
    {
        let _ = (path, body);
        Err(hosted_server_disabled())
    }

    /// Sends an authenticated empty POST request to /client/login, which signals to the server
    /// that the user is logged in.
    pub async fn notify_login(&self) {
        log::debug!("Skipping hosted login notification in Warper.");
    }

    /// Hits the /ai/generate_input_suggestions endpoint to get the predicted next action, based on past context.
    pub async fn generate_ai_input_suggestions(
        &self,
        request: &GenerateAIInputSuggestionsRequest,
    ) -> Result<generate_ai_input_suggestions::GenerateAIInputSuggestionsResponseV2, AIApiError>
    {
        let _ = request;
        Err(AIApiError::Other(hosted_server_disabled()))
    }

    pub async fn get_relevant_files(
        &self,
        request: &GetRelevantFiles,
    ) -> Result<GetRelevantFilesResponse, AIApiError> {
        let _ = request;
        Err(AIApiError::Other(hosted_server_disabled()))
    }

    /// Hits the /ai/generate_am_query_suggestions endpoint to get the predicted next query.
    pub async fn generate_am_query_suggestions(
        &self,
        request: &GenerateAMQuerySuggestionsRequest,
    ) -> Result<generate_am_query_suggestions::GenerateAMQuerySuggestionsResponse, AIApiError> {
        let _ = request;
        Err(AIApiError::Other(hosted_server_disabled()))
    }

    pub async fn predict_am_queries(
        &self,
        request: &PredictAMQueriesRequest,
    ) -> Result<PredictAMQueriesResponse, AIApiError> {
        let _ = request;
        Err(AIApiError::Other(hosted_server_disabled()))
    }

    /// Hits the /ai/transcribe endpoint to get the transcription for the given audio.
    pub async fn transcribe(
        &self,
        request: &TranscribeRequest,
    ) -> Result<TranscribeResponse, TranscribeError> {
        let _ = request;
        Err(TranscribeError::Other(hosted_server_disabled()))
    }

    pub async fn generate_multi_agent_output(
        &self,
        request: &warp_multi_agent_api::Request,
    ) -> std::result::Result<AIOutputStream<warp_multi_agent_api::ResponseEvent>, Arc<AIApiError>>
    {
        let _ = request;
        Err(Arc::new(AIApiError::Other(hosted_server_disabled())))
    }

    /// Returns the inner `http_client::Client` used by the `ServerApi`. Callers can use this long-lived
    /// client to make requests without having to create a new client.
    pub fn http_client(&self) -> &http_client::Client {
        &self.client
    }
}

/// A singleton entity that provides access to the global [`ServerApi`] instance,
/// or any of its implemented trait objects.
pub struct ServerApiProvider {
    server_api: Arc<ServerApi>,
}

impl ServerApiProvider {
    /// Constructs a new ServerApiProvider.
    pub fn new(
        auth_state: Arc<AuthState>,
        _agent_source: Option<ai::AgentSource>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            server_api: Arc::new(ServerApi::local_only(auth_state)),
        }
    }

    /// Constructs a new SeverApiProvider for tests.
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            server_api: Arc::new(ServerApi::new_for_test()),
        }
    }

    /// Returns a handle to the underlying [`ServerApi`] object.
    /// Prefer retrieving a specific trait object related to the methods you're calling.
    pub fn get(&self) -> Arc<ServerApi> {
        self.server_api.clone()
    }

    pub fn get_ai_client(&self) -> Arc<dyn AIClient> {
        self.server_api.clone()
    }

    /// Returns the shared HTTP client. This client is wired into network logging
    /// and includes standard Warp request headers.
    pub fn get_http_client(&self) -> Arc<http_client::Client> {
        self.server_api.client.clone()
    }
}

impl Entity for ServerApiProvider {
    type Event = ServerApiEvent;
}

impl SingletonEntity for ServerApiProvider {}
