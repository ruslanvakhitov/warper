use std::sync::Arc;

use warpui::{Entity, ModelContext, SingletonEntity};

use super::auth_state::AuthState;
use super::auth_view_modal::AuthViewVariant;
use super::AuthStateProvider;
use crate::server::telemetry::AnonymousUserSignupEntrypoint;

#[derive(Debug)]
pub enum AuthManagerEvent {
    AuthComplete,
}

pub type LoginGatedFeature = &'static str;

/// Local-only compatibility shell for code that still depends on an auth
/// singleton type. It never opens hosted login/signup/SSO flows, creates
/// Firebase users, refreshes tokens, or emits reauth UI.
pub struct AuthManager {
    auth_state: Arc<AuthState>,
}

impl AuthManager {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self {
            auth_state: AuthStateProvider::as_ref(ctx).get().clone(),
        }
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn new_for_test(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(ctx)
    }

    pub fn refresh_user(&self, _ctx: &mut ModelContext<Self>) {}

    pub fn set_needs_reauth(&self, _needs_reauth: bool, _ctx: &mut ModelContext<Self>) {}

    pub fn attempt_login_gated_feature(
        &self,
        _feature: LoginGatedFeature,
        _auth_view_variant: AuthViewVariant,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn anonymous_user_hit_drive_object_limit(&self, _ctx: &mut ModelContext<Self>) {}

    pub fn initiate_anonymous_user_linking(
        &self,
        _entrypoint: AnonymousUserSignupEntrypoint,
        _ctx: &mut ModelContext<Self>,
    ) {
    }

    pub fn open_url_maybe_with_anonymous_token(
        &self,
        _ctx: &mut ModelContext<Self>,
        _construct_url: Box<dyn FnOnce(Option<&str>) -> String>,
    ) {
    }

    pub fn set_user_onboarded(&self, _ctx: &mut ModelContext<Self>) {
        self.auth_state.set_is_onboarded(true);
    }
}

#[derive(Clone, Debug)]
pub struct PersistedCurrentUserInformation {
    pub email: String,
}

impl Entity for AuthManager {
    type Event = AuthManagerEvent;
}

impl SingletonEntity for AuthManager {}
