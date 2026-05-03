use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use uuid::Uuid;
use warp_core::channel::{Channel, ChannelState};
use warp_graphql::object_permissions::OwnerType;
use warpui::{AppContext, Entity, SingletonEntity};
use warpui_extras::secure_storage::AppContextExt;

use crate::{
    cloud_object::{GenericStringObjectFormat, JsonObjectType, ObjectType},
    report_error,
};

use super::{
    anonymous_id::get_or_create_anonymous_id,
    credentials::Credentials,
    user::{AnonymousUserType, FirebaseAuthTokens, PersonalObjectLimits, PrincipalType, User},
    UserUid,
};

const ANONYMOUS_USER_NOTIFICATION_BLOCK_TIMER: Duration = Duration::days(7);
const LEGACY_USER_STORAGE_KEY: &str = "User";

/// AuthState is local-only in Warper. It may expose passive user metadata to
/// retained local code, but startup never restores, refreshes, or creates
/// Warp-hosted credentials.
pub struct AuthState {
    /// The currently logged-in User. None if the user isn't logged in currently.
    user: RwLock<Option<User>>,

    /// Local UUID used where retained local code needs a stable process/user
    /// identifier without account state.
    anonymous_id: Uuid,

    /// Legacy compatibility bit. Warper never sets this from a hosted refresh
    /// failure or shows a reauth surface.
    needs_reauth: AtomicBool,

    /// The current authentication credentials.
    credentials: RwLock<Option<Credentials>>,
}

impl AuthState {
    fn new(ctx: &AppContext) -> Self {
        Self {
            user: RwLock::new(None),
            anonymous_id: get_or_create_anonymous_id(ctx),
            needs_reauth: AtomicBool::new(false),
            credentials: RwLock::new(None),
        }
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn new_for_test() -> Self {
        Self {
            user: RwLock::new(Some(User::test())),
            anonymous_id: Uuid::new_v4(),
            needs_reauth: AtomicBool::new(false),
            credentials: RwLock::new(Some(Credentials::Test)),
        }
    }

    /// Creates local-only auth state. Tests may still install a test user, but
    /// Warper startup does not restore or create Warp-hosted credentials.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    pub fn initialize(ctx: &AppContext) -> Self {
        let state = Self::new(ctx);

        if Self::should_use_test_user() {
            state.set_user(Some(User::test()));
            #[cfg(any(test, feature = "integration_tests", feature = "skip_login"))]
            state.set_credentials(Some(Credentials::Test));
            return state;
        }

        let _ = ctx
            .secure_storage()
            .remove_value(LEGACY_USER_STORAGE_KEY)
            .map_err(|err| {
                log::info!("Unable to clear persisted Warp auth state: {err:?}");
            });

        state
    }

    fn should_use_test_user() -> bool {
        cfg!(any(test, feature = "skip_login")) || ChannelState::channel() == Channel::Integration
    }

    /// Sets passive user metadata for tests or retained local compatibility.
    pub(super) fn set_user(&self, user: Option<User>) {
        *self.user.write() = user;
    }

    /// Returns the current credentials.
    pub fn credentials(&self) -> Option<Credentials> {
        self.credentials.read().clone()
    }

    /// Sets passive credentials for tests or retained local compatibility.
    pub(super) fn set_credentials(&self, credentials: Option<Credentials>) {
        *self.credentials.write() = credentials;
    }

    /// Updates the Firebase auth tokens within the current credentials.
    /// Reports an error if the current credentials are not Firebase.
    pub(crate) fn update_firebase_tokens(&self, new_auth_tokens: FirebaseAuthTokens) {
        let mut write_lock = self.credentials.write();
        if let Some(Credentials::Firebase(tokens)) = write_lock.as_mut() {
            *tokens = new_auth_tokens;
        } else {
            report_error!(anyhow!(
                "Tried to update Firebase tokens without Firebase credentials"
            ));
        }
    }

    /// Determines whether the user should be considered as logged in.
    pub fn is_logged_in(&self) -> bool {
        self.credentials.read().is_some()
    }

    /// Returns whether the user should be treated as not having a full account.
    /// True if the user is anonymous OR if there is no user at all (fully logged out).
    ///
    /// Note: uses `unwrap_or(true)` intentionally (not `unwrap_or_default()`) so that
    /// during the transient state where credentials exist but user data hasn't loaded
    /// yet, the user is conservatively treated as lacking a full account.
    pub fn is_anonymous_or_logged_out(&self) -> bool {
        !self.is_logged_in() || self.is_user_anonymous().unwrap_or(true)
    }

    /// Returns the cached access token, if any exists. This method *will not* check if the JWT is
    /// still valid! Usually, you want to use [`ServerApi::get_or_refresh_access_token`] instead!
    pub fn get_access_token_ignoring_validity(&self) -> Option<String> {
        let credentials = self.credentials.read();
        credentials.as_ref()?.bearer_token().bearer_token()
    }

    /// Returns the user's display name.
    pub fn username_for_display(&self) -> Option<String> {
        Some(self.user.read().as_ref()?.username_for_display().to_owned())
    }

    /// Returns the user's display name, does NOT fall back to email.
    pub fn display_name(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.display_name().to_owned())
    }

    /// Returns the user's email. Note the non-obvious semantics of this function:
    /// If the user is logged in and not anonymous, the email will always be populated.
    /// If the user is logged in and anonymous, their email will be an empty string.
    /// If the user is not logged in, their email will be `None`.
    pub fn user_email(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .map(|user| user.metadata.email.clone())
    }

    /// Returns whether the user considered onboarded to Warp.
    pub fn is_onboarded(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.is_onboarded)
    }

    /// Returns the user's email domain (anything after the @ sign of their email).
    pub fn user_email_domain(&self) -> Option<String> {
        self.user.read().as_ref().map(|user| {
            user.metadata
                .email
                .clone()
                .split('@')
                .nth(1)
                .unwrap_or("")
                .to_string()
        })
    }

    /// Returns whether retained passive metadata describes a legacy anonymous
    /// account. Returns `None` when no metadata is installed.
    pub fn is_user_anonymous(&self) -> Option<bool> {
        self.user
            .read()
            .as_ref()
            .map(|user| user.is_user_anonymous())
    }

    /// Returns whether retained passive metadata came from a legacy web-client
    /// anonymous account.
    pub fn is_user_web_anonymous_user(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| {
            user.anonymous_user_type() == Some(AnonymousUserType::WebClientAnonymousUser)
                && user.linked_at().is_none()
        })
    }

    /// Returns whether or not the user is a feature gated anonymous user.
    pub fn is_anonymous_user_feature_gated(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| {
            if !self.is_user_anonymous().unwrap_or_default() {
                return false;
            }

            matches!(
                user.anonymous_user_type(),
                Some(AnonymousUserType::NativeClientAnonymousUserFeatureGated)
            )
        })
    }

    /// Legacy object-limit helper for dead cloud-object code. Local Warper
    /// startup installs no anonymous account metadata, so this normally returns
    /// `None`.
    pub fn is_anonymous_user_past_object_limit(
        &self,
        object_type: ObjectType,
        num_objects: usize,
    ) -> Option<bool> {
        self.user.read().as_ref().map(|user| {
            if !self.is_anonymous_user_feature_gated().unwrap_or_default() {
                return false;
            }

            if let Some(limits) = user.personal_object_limits() {
                match object_type {
                    ObjectType::Notebook => num_objects > limits.notebook_limit,
                    ObjectType::Workflow => num_objects > limits.workflow_limit,
                    ObjectType::GenericStringObject(GenericStringObjectFormat::Json(
                        JsonObjectType::EnvVarCollection,
                    )) => num_objects > limits.env_var_limit,
                    _ => false,
                }
            } else {
                false
            }
        })
    }

    /// Returns a retained passive profile-photo URL, if present.
    pub fn user_photo_url(&self) -> Option<String> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.metadata.photo_url.clone())
    }

    /// Legacy passive SSO-link metadata. Warper does not surface SSO UI.
    pub fn needs_sso_link(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.needs_sso_link)
    }

    /// Returns the anonymous user type.
    /// Retained passive legacy account type.
    pub fn anonymous_user_type(&self) -> Option<AnonymousUserType> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.anonymous_user_type())
    }

    /// Returns the personal object limits the user has.
    /// Retained passive legacy object limits.
    pub fn personal_object_limits(&self) -> Option<PersonalObjectLimits> {
        self.user
            .read()
            .as_ref()
            .and_then(|user| user.personal_object_limits())
    }

    /// Set whether or not the user is onboarded.
    pub fn set_is_onboarded(&self, is_onboarded: bool) {
        if let Some(user) = self.user.write().as_mut() {
            user.is_onboarded = is_onboarded;
        }
    }

    /// Returns a retained passive user id. Local Warper startup returns `None`.
    pub fn user_id(&self) -> Option<UserUid> {
        self.user.read().as_ref().map(|user| user.local_id)
    }

    /// Returns the user's anonymous id.
    /// The anonymous id will be consistent across the app's lifetime. It is a random UUID.
    pub fn anonymous_id(&self) -> String {
        self.anonymous_id.to_string()
    }

    /// Returns the inert legacy reauth bit.
    pub fn needs_reauth(&self) -> bool {
        self.needs_reauth.load(Ordering::Relaxed)
    }

    /// Sets the inert legacy reauth bit.
    pub(super) fn set_needs_reauth(&self, new_needs_reauth: bool) -> bool {
        let prev_needs_reauth = self.needs_reauth.swap(new_needs_reauth, Ordering::Relaxed);
        !prev_needs_reauth && new_needs_reauth
    }

    /// Legacy helper for dead anonymous-account prompts.
    pub fn anonymous_user_renotification_block_expired(
        &self,
        last_time_opt: Option<String>,
    ) -> bool {
        self.is_anonymous_user_feature_gated().unwrap_or_default()
            && last_time_opt
                .and_then(|last_time_string| last_time_string.parse::<DateTime<Utc>>().ok())
                .is_none_or(|last_time| {
                    Utc::now() - ANONYMOUS_USER_NOTIFICATION_BLOCK_TIMER >= last_time
                })
    }

    /// Returns whether or not the user is on a work domain.
    /// This calculation is done on the server, using a list of
    pub fn is_on_work_domain(&self) -> Option<bool> {
        self.user.read().as_ref().map(|user| user.is_on_work_domain)
    }

    /// Returns whether the current user is authenticated via API key.
    pub fn is_api_key_authenticated(&self) -> bool {
        matches!(
            self.credentials.read().as_ref(),
            Some(Credentials::ApiKey { .. })
        )
    }

    /// Returns the API key if using API key authentication.
    pub fn api_key(&self) -> Option<String> {
        let credentials = self.credentials.read();
        credentials.as_ref()?.as_api_key().map(|s| s.to_owned())
    }

    /// Returns the type of principal (user or service account).
    pub fn principal_type(&self) -> Option<PrincipalType> {
        self.user.read().as_ref().map(|user| user.principal_type)
    }

    /// Returns whether the authenticated principal is a service account.
    pub fn is_service_account(&self) -> bool {
        matches!(self.principal_type(), Some(PrincipalType::ServiceAccount))
    }

    /// Returns the owner type of the currently-authenticated API key.
    pub fn api_key_owner_type(&self) -> Option<OwnerType> {
        self.credentials.read().as_ref()?.api_key_owner_type()
    }
}

// Adapter for the [`warp_managed_secrets`] crate, which needs to access the current user.
impl warp_managed_secrets::ActorProvider for AuthState {
    fn actor_uid(&self) -> Option<String> {
        self.user_id().map(|uid| uid.as_string())
    }
}

/// AuthStateProvider is a singleton model which provides a reference to the global AuthState.
pub struct AuthStateProvider {
    auth_state: Arc<AuthState>,
}

impl AuthStateProvider {
    pub fn new(auth_state: Arc<AuthState>) -> Self {
        Self { auth_state }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            auth_state: Arc::new(AuthState::new_for_test()),
        }
    }

    /// Constructs a provider backed by a fully logged-out `AuthState` (no user,
    /// no credentials). Used by unit tests that need to exercise code paths
    /// gated on `AuthState::user_id()` / `UserWorkspaces::personal_drive()`
    /// returning `None`.
    #[cfg(test)]
    pub fn new_logged_out_for_test() -> Self {
        Self {
            auth_state: Arc::new(AuthState {
                user: RwLock::new(None),
                anonymous_id: Uuid::new_v4(),
                needs_reauth: AtomicBool::new(false),
                credentials: RwLock::new(None),
            }),
        }
    }

    pub fn get(&self) -> &Arc<AuthState> {
        &self.auth_state
    }
}

impl Entity for AuthStateProvider {
    type Event = ();
}

impl SingletonEntity for AuthStateProvider {}
