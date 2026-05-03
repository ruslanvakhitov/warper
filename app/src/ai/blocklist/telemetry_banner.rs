use warpui::{
    elements::{Empty, MouseStateHandle},
    AppContext, Element, Entity, View, ViewContext,
};

#[derive(Default, Debug, Clone)]
pub struct TelemetryBanner {
    pub is_onboarded: bool,
    pub learn_more_mouse_state: MouseStateHandle,
    pub privacy_settings_mouse_state: MouseStateHandle,
    pub close_button_mouse_state: MouseStateHandle,
}

impl TelemetryBanner {
    pub fn new(is_onboarded: bool, _ctx: &mut ViewContext<Self>) -> Self {
        Self {
            is_onboarded,
            learn_more_mouse_state: Default::default(),
            privacy_settings_mouse_state: Default::default(),
            close_button_mouse_state: Default::default(),
        }
    }
}

impl View for TelemetryBanner {
    fn ui_name() -> &'static str {
        "TelemetryBanner"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let _ = app;
        Empty::new().finish()
    }
}

impl Entity for TelemetryBanner {
    type Event = ();
}

/// Returns `true` if we should collect UGC (user-generated content) telemetry for AI features.
///
/// This should apply to telemetry events that include user-generated content, like queries or
/// outputs, but need not be checked for regular metadata telemetry events.
///
/// For example, a metadata event that records if a user toggled Pair/Dispatch mode does not
/// require this check, but an event that logs the input buffer for natural language detection
/// _does_ need to check this.
pub fn should_collect_ai_ugc_telemetry(app: &AppContext, is_telemetry_enabled: bool) -> bool {
    let _ = (app, is_telemetry_enabled);
    false
}
