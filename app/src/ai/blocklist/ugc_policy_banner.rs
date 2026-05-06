use warpui::{
    elements::{Empty, MouseStateHandle},
    AppContext, Element, Entity, View, ViewContext,
};

#[derive(Default, Debug, Clone)]
pub struct UgcPolicyBanner {
    pub is_onboarded: bool,
    pub learn_more_mouse_state: MouseStateHandle,
    pub privacy_settings_mouse_state: MouseStateHandle,
    pub close_button_mouse_state: MouseStateHandle,
}

impl UgcPolicyBanner {
    pub fn new(is_onboarded: bool, _ctx: &mut ViewContext<Self>) -> Self {
        Self {
            is_onboarded,
            learn_more_mouse_state: Default::default(),
            privacy_settings_mouse_state: Default::default(),
            close_button_mouse_state: Default::default(),
        }
    }
}

impl View for UgcPolicyBanner {
    fn ui_name() -> &'static str {
        "UgcPolicyBanner"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let _ = app;
        Empty::new().finish()
    }
}

impl Entity for UgcPolicyBanner {
    type Event = ();
}

/// Returns `true` if local AI features may retain user-generated content in diagnostic metadata.
pub fn should_collect_ai_ugc(app: &AppContext, is_telemetry_enabled: bool) -> bool {
    let _ = (app, is_telemetry_enabled);
    false
}
