use std::{result::Result as StdResult, sync::Arc};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use instant::Duration;
#[cfg(test)]
use mockall::{automock, predicate::*};
use thiserror::Error;
use warp_core::errors::{AnyhowErrorExt, ErrorExt};
use warp_graphql::mutations::expire_api_key::ExpireApiKeyResult;
use warp_graphql::queries::get_conversation_usage::ConversationUsage;

use warp_graphql::mutations::{
    create_anonymous_user::{AnonymousUserType, CreateAnonymousUserResult},
    generate_api_key::GenerateApiKeyResult,
    mint_custom_token::MintCustomTokenResult,
};
use warp_graphql::object_permissions::OwnerType;
use warp_graphql::queries::api_keys::ApiKeyProperties;
use warp_graphql::queries::get_user::UserOutput as GqlUserOutput;
use warpui::r#async::BoxFuture;

use crate::auth::UserUid;
use crate::server::graphql::get_user_facing_error_message;
use crate::server::ids::ApiKeyUid;
use crate::server::server_api::register_error;
use crate::settings::PrivacySettingsSnapshot;
use crate::{
    auth::{
        credentials::{AuthToken, Credentials, FirebaseToken, LoginToken},
        user::FirebaseAuthTokens,
        user::User,
    },
    convert_to_server_experiment,
    server::experiments::ServerExperiment,
};

use super::ServerApi;

fn hosted_auth_disabled() -> anyhow::Error {
    anyhow!("Warp-hosted auth and account APIs are unavailable in Warper")
}

/// Header key for the ambient workload token attached to multi-agent requests.
pub const AMBIENT_WORKLOAD_TOKEN_HEADER: &str = "X-Warp-Ambient-Workload-Token";

/// Header key for the cloud agent task ID attached to requests from ambient agents.
pub const CLOUD_AGENT_ID_HEADER: &str = "X-Warp-Cloud-Agent-ID";

/// Duration for which the ambient workload token is valid (3 hours).
const AMBIENT_WORKLOAD_TOKEN_DURATION: Duration = Duration::from_secs(3 * 60 * 60);

/// User settings that are currently 'synced' (e.g. stored server-side) on a per-user basis.
#[derive(Copy, Clone, Debug, Default)]
pub struct SyncedUserSettings {
    pub is_cloud_conversation_storage_enabled: bool,
    pub is_crash_reporting_enabled: bool,
    pub is_telemetry_enabled: bool,
}

/// Results of an attempt to fetch the current user.
pub struct FetchUserResult {
    pub user: User,
    /// The credentials used to authenticate this user.
    pub credentials: Credentials,
    pub server_experiments: Vec<ServerExperiment>,
    /// Whether this attempt to fetch the user was for refreshing an existing logged-in user.
    pub from_refresh: bool,
    /// LLM model choices for this user.
    pub llms: crate::ai::llms::ModelsByFeature,
}

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait AuthClient: 'static + Send + Sync {
    /// Creates an anonymous user, who is allowed to use Warp but may lack the ability
    /// to interact with particular features.
    async fn create_anonymous_user(
        &self,
        referral_code: Option<String>,
        anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult>;

    /// Returns the cached access token, if it is still valid. If it has expired, fetches a new
    /// access token using the user's refresh token, caches it, and the returns it.
    /// Returns an auth mode that may not require an Authorization header (e.g. session cookies or
    /// test credentials).
    async fn get_or_refresh_access_token(&self) -> Result<AuthToken>;

    /// Fetches data required to construct the [`User`] object. This includes the user's metadata
    /// and authentication tokens.
    async fn fetch_user(
        &self,
        token: LoginToken,
        for_refresh: bool,
    ) -> StdResult<FetchUserResult, UserAuthenticationError>;

    /// Creates and fetches an new custom token for the current user from Firebase.
    /// This only works for anonymous users, and will surface an error if the user is not anonymous.
    async fn fetch_new_custom_token(&self) -> Result<MintCustomTokenResult>;

    /// Handles the response from [`Self::fetch_new_custom_token`], returning the newly-minted custom token.
    fn on_custom_token_fetched(
        &self,
        response: Result<MintCustomTokenResult>,
    ) -> Result<String, MintCustomTokenError>;

    /// Queries warp-server for a set of the currently logged-in user's fields.
    async fn fetch_user_properties<'a>(&self, auth_token: Option<&'a str>)
        -> Result<GqlUserOutput>;

    /// Upon success, returns an `Option` containing the user's settings retrieved from the server,
    /// if any. The user may not have server-side settings if they onboarded prior to the launch
    /// of telemetry opt-out, have not logged in since the launch, and have never changed defaults
    /// for any of the settings in [`SyncedUserSettings`]. If the fetched settings object exists
    /// but is missing required fields, or if the request itself failed, returns an error.
    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>>;

    /// Returns conversation usage history for the current user over the past n days.
    /// If last_updated_end_timestamp is provided, only conversations with
    /// lastUpdated earlier than this timestamp are returned.
    async fn get_conversation_usage_history(
        &self,
        days: Option<i32>,
        limit: Option<i32>,
        last_updated_end_timestamp: Option<warp_graphql::scalars::Time>,
    ) -> Result<Vec<ConversationUsage>>;

    async fn set_is_telemetry_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_crash_reporting_enabled(&self, value: bool) -> Result<()>;

    async fn set_is_cloud_conversation_storage_enabled(&self, value: bool) -> Result<()>;

    /// Sends a request to update the user's settings on the server with values contained in the
    /// given `settings_snapshot`.
    async fn update_user_settings(&self, settings_snapshot: PrivacySettingsSnapshot) -> Result<()>;

    async fn set_user_is_onboarded(&self) -> Result<bool>;

    /// Requests a device authorization code from the server. This is only used for headless CLI/SDK authentication.
    async fn request_device_code(
        &self,
    ) -> StdResult<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError>;

    /// Wait for the request to be approved or rejected and exchange it for a short-lived custom access token.
    async fn exchange_device_access_token(
        &self,
        details: &oauth2::StandardDeviceAuthorizationResponse,
        timeout: Duration,
    ) -> StdResult<FirebaseToken, UserAuthenticationError>;
    // API Keys
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>>;

    async fn create_api_key(
        &self,
        name: String,
        team_id: Option<cynic::Id>,
        expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult>;

    async fn expire_api_key(&self, key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult>;

    /// Returns a cached ambient workload token, or issues a new one if not present or expired.
    ///
    /// Returns `Ok(None)` if not running in an isolation platform (e.g., Namespace) or on WASM.
    async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>>;
}

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl AuthClient for ServerApi {
    async fn create_anonymous_user(
        &self,
        _referral_code: Option<String>,
        _anonymous_user_type: AnonymousUserType,
    ) -> Result<CreateAnonymousUserResult> {
        Err(hosted_auth_disabled())
    }

    async fn get_or_refresh_access_token(&self) -> Result<AuthToken> {
        Err(hosted_auth_disabled())
    }

    async fn fetch_user(
        &self,
        _token: LoginToken,
        _for_refresh: bool,
    ) -> StdResult<FetchUserResult, UserAuthenticationError> {
        Err(UserAuthenticationError::Unexpected(hosted_auth_disabled()))
    }

    async fn fetch_new_custom_token(&self) -> Result<MintCustomTokenResult> {
        Err(hosted_auth_disabled())
    }

    fn on_custom_token_fetched(
        &self,
        response: Result<MintCustomTokenResult>,
    ) -> Result<String, MintCustomTokenError> {
        match response {
            Ok(response_data) => match response_data {
                MintCustomTokenResult::MintCustomTokenOutput(output) => Ok(output.custom_token),
                MintCustomTokenResult::UserFacingError(user_facing_error) => {
                    Err(MintCustomTokenError::UserFacingError(
                        get_user_facing_error_message(user_facing_error),
                    ))
                }
                MintCustomTokenResult::Unknown => Err(MintCustomTokenError::Unknown),
            },
            Err(_) => Err(MintCustomTokenError::Unknown),
        }
    }

    async fn fetch_user_properties<'a>(
        &self,
        _auth_token: Option<&'a str>,
    ) -> Result<GqlUserOutput> {
        Err(hosted_auth_disabled())
    }

    async fn get_user_settings(&self) -> Result<Option<SyncedUserSettings>> {
        Err(hosted_auth_disabled())
    }

    // Returns a history of the current user's conversation usage over the past n days.
    async fn get_conversation_usage_history(
        &self,
        _days: Option<i32>,
        _limit: Option<i32>,
        _last_updated_end_timestamp: Option<warp_graphql::scalars::Time>,
    ) -> Result<Vec<ConversationUsage>> {
        Err(hosted_auth_disabled())
    }

    async fn set_is_telemetry_enabled(&self, _value: bool) -> Result<()> {
        Err(hosted_auth_disabled())
    }

    async fn set_is_crash_reporting_enabled(&self, _value: bool) -> Result<()> {
        Err(hosted_auth_disabled())
    }

    async fn set_is_cloud_conversation_storage_enabled(&self, _value: bool) -> Result<()> {
        Err(hosted_auth_disabled())
    }

    async fn update_user_settings(
        &self,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Err(hosted_auth_disabled())
    }

    async fn set_user_is_onboarded(&self) -> Result<bool> {
        Err(hosted_auth_disabled())
    }

    async fn request_device_code(
        &self,
    ) -> StdResult<oauth2::StandardDeviceAuthorizationResponse, UserAuthenticationError> {
        Err(UserAuthenticationError::Unexpected(hosted_auth_disabled()))
    }

    async fn exchange_device_access_token(
        &self,
        details: &oauth2::StandardDeviceAuthorizationResponse,
        timeout: Duration,
    ) -> StdResult<FirebaseToken, UserAuthenticationError> {
        let _ = (details, timeout);
        Err(UserAuthenticationError::Unexpected(hosted_auth_disabled()))
    }

    // API Keys
    async fn list_api_keys(&self) -> Result<Vec<ApiKeyProperties>> {
        Err(hosted_auth_disabled())
    }

    async fn create_api_key(
        &self,
        _name: String,
        _team_id: Option<cynic::Id>,
        _expires_at: Option<warp_graphql::scalars::Time>,
    ) -> Result<GenerateApiKeyResult> {
        Err(hosted_auth_disabled())
    }
    async fn expire_api_key(&self, _key_uid: &ApiKeyUid) -> Result<ExpireApiKeyResult> {
        Err(hosted_auth_disabled())
    }

    async fn get_or_create_ambient_workload_token(&self) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Exchange a long-lived token for fresh [`Credentials`].
async fn exchange_credentials(
    client: Arc<http_client::Client>,
    token: LoginToken,
) -> StdResult<Credentials, UserAuthenticationError> {
    let _ = (client, token);
    Err(UserAuthenticationError::Unexpected(hosted_auth_disabled()))
}

fn fetch_auth_tokens(
    _client: Arc<http_client::Client>,
    _token: FirebaseToken,
) -> BoxFuture<'static, StdResult<FirebaseAuthTokens, UserAuthenticationError>> {
    Box::pin(async move {
        Err(UserAuthenticationError::Unexpected(anyhow!(
            "Firebase token exchange is unavailable in Warper"
        )))
    })
}

/// The [`oauth2::Client`] type, specialized to the endpoints that we require.
pub type OAuth2Client = oauth2::basic::BasicClient<
    oauth2::EndpointNotSet, // HasAuthUrl
    oauth2::EndpointSet,    // HasDeviceAuthUrl
    oauth2::EndpointNotSet, // HasIntrospectionUrl
    oauth2::EndpointNotSet, // HasRevocationUrl
    oauth2::EndpointSet,    // HasTokenUrl
>;

/// Intermediate type produced by converting a [`GqlUserOutput`] from the server.
struct UserProperties {
    user: User,
    server_experiments: Vec<ServerExperiment>,
    llms: crate::ai::llms::ModelsByFeature,
    api_key_owner_type: Option<OwnerType>,
}

impl From<GqlUserOutput> for UserProperties {
    fn from(user_output: GqlUserOutput) -> Self {
        let principal_type = user_output
            .principal_type
            .map(|pt| pt.into())
            .unwrap_or_default();
        let user_properties = user_output.user;

        let is_on_work_domain = user_properties.is_on_work_domain;
        let is_onboarded = user_properties.is_onboarded;
        let api_key_owner_type = user_output.api_key_owner_type;

        let linked_at = user_properties
            .anonymous_user_info
            .as_ref()
            .and_then(|info| info.linked_at);

        let anonymous_user_type = user_properties
            .anonymous_user_info
            .as_ref()
            .map(|info| info.anonymous_user_type.clone());
        let personal_object_limits = user_properties
            .anonymous_user_info
            .and_then(|info| info.personal_object_limits.clone());
        let user_profile = user_properties.profile;
        let local_id = UserUid::new(user_profile.uid.as_str());
        let needs_sso_link = user_profile.needs_sso_link;

        let server_experiments: Vec<ServerExperiment> = user_properties
            .experiments
            .and_then(|experiments| convert_to_server_experiment!(experiments))
            .unwrap_or_default();

        // Convert LLM model choices from GraphQL response
        let llms = user_properties.llms.try_into().unwrap_or_default();

        let user = User {
            is_onboarded,
            local_id,
            metadata: user_profile.into(),
            needs_sso_link,
            anonymous_user_type: anonymous_user_type.and_then(|t| t.try_into().ok()),
            is_on_work_domain,
            linked_at,
            personal_object_limits: personal_object_limits.and_then(|t| t.try_into().ok()),
            principal_type,
        };

        UserProperties {
            user,
            server_experiments,
            llms,
            api_key_owner_type,
        }
    }
}

#[derive(Error, Debug)]
/// Error type when retrieving a hosted Warp user.
pub enum UserAuthenticationError {
    #[error("Invalid state parameter in auth redirect")]
    InvalidStateParameter,
    #[error("Missing state parameter in auth redirect")]
    MissingStateParameter,
    #[error("unexpected error occurred when fetching an ID token: {0:#}")]
    Unexpected(#[from] anyhow::Error),
}

impl ErrorExt for UserAuthenticationError {
    fn is_actionable(&self) -> bool {
        match self {
            UserAuthenticationError::Unexpected(err) => err.is_actionable(),
            UserAuthenticationError::InvalidStateParameter
            | UserAuthenticationError::MissingStateParameter => {
                // For now, we're marking these as actionable, since a surplus of these errors
                // could mean that something is wrong in our login flow (e.g. we're not properly
                // passing the `state` variable back to the desktop client).
                // But in general, someone attempting to trick another into logging into their
                // account with a spoofed `state` variable is not actionable.
                true
            }
        }
    }
}
register_error!(UserAuthenticationError);

#[derive(Error, Debug)]
/// Error type when creating anonymous users
pub enum AnonymousUserCreationError {
    #[error("The network request to create the anonymous user failed")]
    CreationFailed,

    #[error("Received a user facing error: {0}")]
    UserFacingError(String),

    /// Failure that occurs after the user is created, but the ID token could not be fetched.
    #[error("The user was created, but the ID token could not be fetched")]
    UserAuthenticationFailed(#[from] UserAuthenticationError),

    #[error("Failed to create anonymous user with unknown error")]
    Unknown,
}

#[derive(Error, Debug)]
/// Error type when minting a new custom token for an anonymous user
pub enum MintCustomTokenError {
    #[error("Received a user facing error: {0}")]
    UserFacingError(String),
    #[error("Failed to create new custom token with unknown error")]
    Unknown,
}

#[cfg(test)]
#[path = "auth_test.rs"]
mod tests;
