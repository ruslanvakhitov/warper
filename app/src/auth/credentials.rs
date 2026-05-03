//! Passive representation of legacy Warp credential shapes.
//!
//! Warper does not launch hosted Warp auth flows, exchange Firebase tokens, or
//! restore persisted credentials. These enums remain only so retained dead
//! server-shaped modules and tests can compile while the runtime graph stays
//! local-only.
use warp_graphql::object_permissions::OwnerType;

use super::user::FirebaseAuthTokens;

/// Legacy credential variants retained as passive data.
#[derive(Clone, Debug)]
pub enum Credentials {
    /// Legacy Firebase authentication with ID token and refresh token.
    Firebase(FirebaseAuthTokens),
    /// Legacy Warp API key authentication.
    ApiKey {
        key: String,
        /// The owner type for this API key.
        owner_type: Option<OwnerType>,
    },
    /// Legacy browser session-cookie authentication.
    SessionCookie,
    /// Test credentials used in unit tests, integration tests, and skip_login builds.
    #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
    Test,
}

impl Credentials {
    /// Returns the Firebase auth tokens if this is a Firebase credential.
    pub fn as_firebase(&self) -> Option<&FirebaseAuthTokens> {
        match self {
            Credentials::Firebase(tokens) => Some(tokens),
            Credentials::ApiKey { .. } => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    /// Returns the API key string if this is an API key credential.
    pub fn as_api_key(&self) -> Option<&str> {
        match self {
            Credentials::ApiKey { key, .. } => Some(key),
            Credentials::Firebase(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    /// Returns the owner type if this is an API key credential.
    pub fn api_key_owner_type(&self) -> Option<OwnerType> {
        match self {
            Credentials::ApiKey { owner_type, .. } => *owner_type,
            Credentials::Firebase(_) => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    /// Returns the Firebase refresh token if this is a Firebase credential.
    pub fn refresh_token(&self) -> Option<&str> {
        match self {
            Credentials::Firebase(tokens) => Some(&tokens.refresh_token),
            Credentials::ApiKey { .. } => None,
            Credentials::SessionCookie => None,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }

    /// Returns the short-lived token to use in HTTP requests to the server.
    pub fn bearer_token(&self) -> AuthToken {
        match self {
            Credentials::Firebase(tokens) => AuthToken::Firebase(tokens.id_token.clone()),
            Credentials::ApiKey { key, .. } => AuthToken::ApiKey(key.clone()),
            Credentials::SessionCookie => AuthToken::NoAuth,
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => AuthToken::NoAuth,
        }
    }

    /// Get the long-lived login token for these credentials. Returns `None` if there is no such token.
    pub fn login_token(&self) -> Option<LoginToken> {
        match self {
            Credentials::Firebase(tokens) => Some(LoginToken::Firebase(FirebaseToken::Refresh(
                RefreshToken::new(&tokens.refresh_token),
            ))),
            Credentials::ApiKey { key, .. } => Some(LoginToken::ApiKey(key.clone())),
            Credentials::SessionCookie => Some(LoginToken::SessionCookie),
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            Credentials::Test => None,
        }
    }
}

/// Represents different types of authentication tokens.
#[derive(Debug, Clone)]
pub enum AuthToken {
    /// Firebase short-lived access token.
    Firebase(String),
    /// API key for legacy non-Warper authentication paths.
    ApiKey(String),
    /// No authentication token available (e.g. session cookie auth or test credentials).
    #[cfg_attr(
        not(any(test, feature = "integration_tests", feature = "skip_login")),
        allow(dead_code)
    )]
    NoAuth,
}

impl AuthToken {
    /// Returns the token string to use in an Authorization header, or `None` if auth is not
    /// header-based (e.g. session cookie) or there is no auth.
    pub fn as_bearer_token(&self) -> Option<&str> {
        match self {
            AuthToken::Firebase(token) => Some(token),
            AuthToken::ApiKey(key) => Some(key),
            AuthToken::NoAuth => None,
        }
    }

    /// Returns the bearer token as an owned string, or `None` if auth is not header-based.
    pub fn bearer_token(&self) -> Option<String> {
        match self {
            AuthToken::Firebase(token) => Some(token.clone()),
            AuthToken::ApiKey(key) => Some(key.clone()),
            AuthToken::NoAuth => None,
        }
    }
}

/// Legacy long-lived credential shapes. Warper does not exchange them.
#[derive(Debug)]
pub enum LoginToken {
    Firebase(FirebaseToken),
    ApiKey(String),
    SessionCookie,
}

/// Legacy Firebase token shape retained as passive data. Warper does not
/// exchange these tokens or construct Firebase/proxy requests.
#[derive(Debug)]
pub enum FirebaseToken {
    Refresh(RefreshToken),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct RefreshToken(String);

impl RefreshToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn get(&self) -> &str {
        self.0.as_str()
    }
}
