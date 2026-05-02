use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use warp_core::ui::appearance::Appearance;
use warpui::{
    elements::{
        ConstrainedBox, Container, CrossAxisAlignment, Flex, FormattedTextElement,
        HighlightedHyperlink, HyperlinkLens, MainAxisAlignment, MainAxisSize, ParentElement,
    },
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext,
};

use crate::{
    ai::blocklist::error_color, network::NetworkStatus, server::ids::ServerId,
    ui_components::icons::Icon,
};

const NO_CONNECTION_PRIMARY_TEXT: &str = "No internet connection";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertAction {
    SignUpClickedForAnonymousUser,
    OpenSettingsClicked,
    OpenPrivacySettingsClicked,
    ManageBillingClicked { team_uid: ServerId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertEvent {
    SignupAnonymousUser,
    OpenBillingAndUsagePage,
    OpenPrivacyPage,
    OpenBillingPortal { team_uid: ServerId },
}

/// The alert state of the chip that appears to the right of certain parts of the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAlertState {
    /// The user is offline (no connection).
    NoConnection,
    /// Hosted Warp billing and quota states are retained as type-level compatibility
    /// for downstream UI code until those call sites are deleted.
    TelemetryDisabledOnFreeTier,
    AnonymousUserRequestLimitSoftGate,
    AnonymousUserRequestLimitHardGate,
    DelinquentDueToPaymentIssue,
    OveragesToggleableButNotEnabled,
    MonthlyOveragesSpendLimitReached,
    RequestLimitReached,
    /// No alert should be displayed.
    NoAlert,
}

pub struct PromptAlertView {
    state: PromptAlertState,
    action_hyperlink: HighlightedHyperlink,
}

impl PromptAlertView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let network_status = NetworkStatus::handle(ctx);

        ctx.subscribe_to_model(&network_status, |me, _, _, ctx| {
            me.state = Self::determine_state(ctx);
            ctx.notify();
        });

        Self {
            state: Self::determine_state(ctx),
            action_hyperlink: Default::default(),
        }
    }

    pub fn determine_state(app: &AppContext) -> PromptAlertState {
        if !NetworkStatus::as_ref(app).is_online() {
            return PromptAlertState::NoConnection;
        }

        PromptAlertState::NoAlert
    }

    pub fn is_no_alert(&self) -> bool {
        matches!(self.state, PromptAlertState::NoAlert)
    }

    pub fn state(&self) -> &PromptAlertState {
        &self.state
    }

    pub fn does_alert_block_ai_requests(app: &AppContext) -> bool {
        does_alert_block_ai_requests(&Self::determine_state(app))
    }

    fn primary_text(
        &self,
        state: &PromptAlertState,
        text_fragments: &mut Vec<FormattedTextFragment>,
    ) {
        text_fragments.push(FormattedTextFragment::plain_text("  "));
        match state {
            PromptAlertState::NoConnection => {
                text_fragments.push(FormattedTextFragment::plain_text(
                    NO_CONNECTION_PRIMARY_TEXT,
                ));
            }
            PromptAlertState::TelemetryDisabledOnFreeTier
            | PromptAlertState::AnonymousUserRequestLimitSoftGate
            | PromptAlertState::AnonymousUserRequestLimitHardGate
            | PromptAlertState::DelinquentDueToPaymentIssue
            | PromptAlertState::OveragesToggleableButNotEnabled
            | PromptAlertState::MonthlyOveragesSpendLimitReached
            | PromptAlertState::RequestLimitReached
            | PromptAlertState::NoAlert => {}
        }
    }
}

fn does_alert_block_ai_requests(state: &PromptAlertState) -> bool {
    match state {
        PromptAlertState::NoConnection => true,
        PromptAlertState::TelemetryDisabledOnFreeTier
        | PromptAlertState::AnonymousUserRequestLimitSoftGate
        | PromptAlertState::AnonymousUserRequestLimitHardGate
        | PromptAlertState::DelinquentDueToPaymentIssue
        | PromptAlertState::OveragesToggleableButNotEnabled
        | PromptAlertState::MonthlyOveragesSpendLimitReached
        | PromptAlertState::RequestLimitReached
        | PromptAlertState::NoAlert => false,
    }
}

impl Entity for PromptAlertView {
    type Event = PromptAlertEvent;
}

impl View for PromptAlertView {
    fn ui_name() -> &'static str {
        "PromptAlertView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let state = Self::determine_state(app);
        let mut text_fragments = vec![];

        self.primary_text(&state, &mut text_fragments);

        let formatted_text_element = FormattedTextElement::new(
            FormattedText::new([FormattedTextLine::Line(text_fragments)]),
            appearance.ui_font_size(),
            appearance.ui_font_family(),
            appearance.ui_font_family(),
            error_color(appearance.theme()),
            self.action_hyperlink.clone(),
        )
        .with_line_height_ratio(1.)
        .with_hyperlink_font_color(appearance.theme().ansi_fg_blue())
        .with_no_text_wrapping()
        .register_default_click_handlers_with_action_support(|hyperlink_lens, event, ctx| {
            match hyperlink_lens {
                HyperlinkLens::Url(url) => {
                    ctx.open_url(url);
                }
                HyperlinkLens::Action(action_ref) => {
                    if let Some(action) = action_ref.as_any().downcast_ref::<PromptAlertAction>() {
                        event.dispatch_typed_action(action.clone());
                    }
                }
            }
        })
        .finish();

        let icon_size = appearance.ui_font_size();

        let mut chip_row = Flex::row()
            .with_main_axis_alignment(MainAxisAlignment::End)
            .with_main_axis_size(MainAxisSize::Max)
            .with_cross_axis_alignment(CrossAxisAlignment::End);
        if does_alert_block_ai_requests(&self.state) {
            chip_row.add_child(
                ConstrainedBox::new(
                    Icon::AlertTriangle
                        .to_warpui_icon(error_color(appearance.theme()).into())
                        .finish(),
                )
                .with_width(icon_size)
                .with_height(icon_size)
                .finish(),
            )
        }

        chip_row.add_child(formatted_text_element);

        Container::new(chip_row.finish())
            .with_margin_right(16.)
            .finish()
    }
}

impl TypedActionView for PromptAlertView {
    type Action = PromptAlertAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            PromptAlertAction::SignUpClickedForAnonymousUser => {
                ctx.emit(PromptAlertEvent::SignupAnonymousUser);
            }
            PromptAlertAction::OpenSettingsClicked => {
                ctx.emit(PromptAlertEvent::OpenBillingAndUsagePage);
            }
            PromptAlertAction::OpenPrivacySettingsClicked => {
                ctx.emit(PromptAlertEvent::OpenPrivacyPage);
            }
            PromptAlertAction::ManageBillingClicked { team_uid } => {
                ctx.emit(PromptAlertEvent::OpenBillingPortal {
                    team_uid: *team_uid,
                });
            }
        }
    }
}
