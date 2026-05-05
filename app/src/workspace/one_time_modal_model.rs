use super::hoa_onboarding;
use crate::settings::CodeSettings;
use settings::Setting as _;
use warp_core::features::FeatureFlag;
use warpui::{Entity, ModelContext, SingletonEntity, WindowId};

/// A model for managing local one-time flows that should be shown only once.
///
/// The model holds the canonical state of whether a flow is currently being shown and
/// automatically triggers it when appropriate conditions are met.
pub struct OneTimeModalModel {
    /// Whether the HOA onboarding flow is currently being shown.
    is_hoa_onboarding_open: bool,
    /// The window ID where the currently open one-time modal should be displayed.
    /// This is captured when a modal is first opened and ensures the modal stays on that window.
    target_window_id: Option<WindowId>,
}

impl OneTimeModalModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let _ = ctx;

        Self {
            is_hoa_onboarding_open: false,
            target_window_id: None,
        }
    }

    /// Returns the window ID where the currently open one-time modal should be displayed.
    pub fn target_window_id(&self) -> Option<WindowId> {
        self.target_window_id
    }

    /// Returns whether the HOA onboarding flow is currently open.
    pub fn is_hoa_onboarding_open(&self) -> bool {
        self.is_hoa_onboarding_open && self.target_window_id.is_some()
    }

    pub fn mark_hoa_onboarding_dismissed(&mut self, ctx: &mut ModelContext<Self>) {
        self.set_hoa_onboarding_open(false, ctx);
    }

    /// Returns true if any one-time modal is currently open.
    pub fn is_any_modal_open(&self) -> bool {
        self.is_hoa_onboarding_open && self.target_window_id.is_some()
    }

    pub fn update_target_window_id(&mut self, window_id: WindowId, ctx: &mut ModelContext<Self>) {
        let was_any_modal_visible = self.is_any_modal_open();
        self.target_window_id = Some(window_id);
        if was_any_modal_visible != self.is_any_modal_open() {
            ctx.emit(OneTimeModalEvent::VisibilityChanged {
                is_open: self.is_any_modal_open(),
            });
        }
    }

    fn check_and_trigger_all_modals(&mut self, ctx: &mut ModelContext<Self>) {
        // Never show one-time modals on WASM.
        if cfg!(target_family = "wasm") {
            return;
        }

        // Existing users should never see the code toolbelt new feature popup.
        CodeSettings::handle(ctx).update(ctx, |settings, ctx| {
            if let Err(e) = settings
                .dismissed_code_toolbelt_new_feature_popup
                .set_value(true, ctx)
            {
                log::warn!("Failed to mark code toolbelt new feature popup as dismissed: {e}");
            }
        });

        self.check_and_trigger_hoa_onboarding(ctx);
    }

    fn set_hoa_onboarding_open(&mut self, is_open: bool, ctx: &mut ModelContext<Self>) -> bool {
        if self.is_hoa_onboarding_open != is_open {
            self.is_hoa_onboarding_open = is_open;
            ctx.emit(OneTimeModalEvent::VisibilityChanged { is_open });
            return true;
        }
        false
    }

    fn check_and_trigger_hoa_onboarding(&mut self, ctx: &mut ModelContext<Self>) -> bool {
        if !FeatureFlag::HOAOnboardingFlow.is_enabled() {
            return false;
        }

        if hoa_onboarding::has_completed_hoa_onboarding(ctx) {
            return false;
        }

        // All required dependent feature flags must be enabled.
        if !FeatureFlag::VerticalTabs.is_enabled()
            || !FeatureFlag::HOANotifications.is_enabled()
            || !FeatureFlag::TabConfigs.is_enabled()
        {
            return false;
        }

        self.set_hoa_onboarding_open(true, ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneTimeModalEvent {
    VisibilityChanged { is_open: bool },
}

impl Entity for OneTimeModalModel {
    type Event = OneTimeModalEvent;
}

impl SingletonEntity for OneTimeModalModel {}
