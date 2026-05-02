use warp_core::ui::appearance::Appearance;
use warpui::{
    elements::Empty, AppContext, Element, Entity, TypedActionView, View, ViewContext, ViewHandle,
};

use crate::{
    settings_view::{
        settings_page::{MatchData, PageType, SettingsPageMeta, SettingsPageViewHandle},
        SettingsSection,
    },
    view_components::ToastFlavor,
};

pub fn create_discount_badge(_discount: u32, _appearance: &Appearance) -> Box<dyn Element> {
    Empty::new().finish()
}

pub struct BillingAndUsagePageView {
    page: PageType<Self>,
}

impl BillingAndUsagePageView {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self {
            page: PageType::new_uncategorized(Vec::new(), Some("Billing and usage")),
        }
    }

    pub fn get_modal_content(&self) -> Option<Box<dyn Element>> {
        None
    }
}

impl SettingsPageMeta for BillingAndUsagePageView {
    fn section() -> SettingsSection {
        SettingsSection::BillingAndUsage
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        false
    }

    fn update_filter(&mut self, _query: &str, _ctx: &mut ViewContext<Self>) -> MatchData {
        MatchData::Uncounted(false)
    }

    fn scroll_to_widget(&mut self, _widget_id: &'static str) {}

    fn clear_highlighted_widget(&mut self) {}
}

impl Entity for BillingAndUsagePageView {
    type Event = BillingAndUsagePageEvent;
}

impl View for BillingAndUsagePageView {
    fn ui_name() -> &'static str {
        "BillingAndUsagePageView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}

impl TypedActionView for BillingAndUsagePageView {
    type Action = BillingAndUsagePageAction;

    fn handle_action(&mut self, _action: &BillingAndUsagePageAction, _ctx: &mut ViewContext<Self>) {
    }
}

impl From<ViewHandle<BillingAndUsagePageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<BillingAndUsagePageView>) -> Self {
        SettingsPageViewHandle::BillingAndUsage(view_handle)
    }
}

#[derive(Debug, Clone)]
pub enum BillingAndUsagePageEvent {
    SignupAnonymousUser,
    ShowToast {
        message: String,
        flavor: ToastFlavor,
    },
    ShowModal,
    HideModal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BillingAndUsagePageAction {
    ToggleUsageEntryExpanded { conversation_id: String },
    NavigateToByokSettings,
}
