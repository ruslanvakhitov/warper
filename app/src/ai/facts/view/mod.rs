use crate::pane_group::focus_state::PaneFocusHandle;
use crate::pane_group::{pane::view, BackingView, PaneConfiguration, PaneEvent};
use warp_core::ui::appearance::Appearance;
use warpui::{
    elements::{
        Align, ClippedScrollStateHandle, ClippedScrollable, ConstrainedBox, Container, Flex,
        MainAxisSize, ParentElement, ScrollbarWidth,
    },
    AppContext, Element, Entity, FocusContext, ModelHandle, SingletonEntity, TypedActionView, View,
    ViewContext,
};

use warpui::elements::ChildView;
use warpui::ViewHandle;

pub mod rule;
mod style;
use rule::*;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum AIFactPage {
    #[default]
    Rules,
}

impl std::fmt::Display for AIFactPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AIFactPage::Rules => write!(f, "Rules"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AIFactViewEvent {
    Pane(PaneEvent),
}

#[derive(Debug, Clone)]
pub enum AIFactViewAction {
    UpdatePage(AIFactPage),
}

pub struct AIFactView {
    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,
    current_page: AIFactPage,
    rule_view: ViewHandle<RuleView>,
    clipped_scroll_state: ClippedScrollStateHandle,
}

impl AIFactView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let pane_configuration = ctx.add_model(|_ctx| PaneConfiguration::new(HEADER_TEXT));

        let rule_view = ctx.add_typed_action_view(RuleView::new);
        ctx.subscribe_to_view(&rule_view, |me, _, event, ctx| {
            me.handle_rule_view_event(event, ctx);
        });

        Self {
            pane_configuration,
            focus_handle: None,
            rule_view,
            current_page: AIFactPage::default(),
            clipped_scroll_state: Default::default(),
        }
    }

    pub fn pane_configuration(&self) -> ModelHandle<PaneConfiguration> {
        self.pane_configuration.clone()
    }

    pub fn current_page(&self) -> AIFactPage {
        self.current_page
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        match self.current_page {
            AIFactPage::Rules => ctx.focus(&self.rule_view),
        }
    }

    fn handle_rule_view_event(&mut self, _event: &(), _ctx: &mut ViewContext<Self>) {}

    pub fn update_page(&mut self, page: AIFactPage, ctx: &mut ViewContext<Self>) {
        self.current_page = page;
        self.focus(ctx);
        ctx.notify();
    }
}

impl Entity for AIFactView {
    type Event = AIFactViewEvent;
}

impl View for AIFactView {
    fn ui_name() -> &'static str {
        "AIFactView"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            match self.current_page {
                AIFactPage::Rules => ctx.focus(&self.rule_view),
            }
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let mut col = Flex::column().with_main_axis_size(MainAxisSize::Min);
        match self.current_page {
            AIFactPage::Rules => col.add_child(ChildView::new(&self.rule_view).finish()),
        }

        ClippedScrollable::vertical(
            self.clipped_scroll_state.clone(),
            Align::new(
                Container::new(
                    ConstrainedBox::new(col.finish())
                        .with_max_width(style::PANE_WIDTH)
                        .finish(),
                )
                .with_uniform_padding(style::PANE_PADDING)
                .finish(),
            )
            .top_center()
            .finish(),
            ScrollbarWidth::Auto,
            appearance.theme().nonactive_ui_detail().into(),
            appearance.theme().active_ui_detail().into(),
            warpui::elements::Fill::None,
        )
        .finish()
    }
}

impl TypedActionView for AIFactView {
    type Action = AIFactViewAction;

    fn handle_action(&mut self, action: &AIFactViewAction, ctx: &mut ViewContext<Self>) {
        match action {
            AIFactViewAction::UpdatePage(page) => self.update_page(*page, ctx),
        }
    }
}

impl BackingView for AIFactView {
    type PaneHeaderOverflowMenuAction = AIFactViewAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        _action: &Self::PaneHeaderOverflowMenuAction,
        _ctx: &mut warpui::ViewContext<Self>,
    ) {
        self.handle_action(_action, _ctx)
    }

    fn close(&mut self, ctx: &mut warpui::ViewContext<Self>) {
        ctx.emit(AIFactViewEvent::Pane(PaneEvent::Close));
    }

    fn focus_contents(&mut self, ctx: &mut warpui::ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &view::HeaderRenderContext<'_>,
        _app: &AppContext,
    ) -> view::HeaderContent {
        view::HeaderContent::simple(HEADER_TEXT)
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
    }
}
