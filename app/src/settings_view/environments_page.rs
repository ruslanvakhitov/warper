use super::{
    agent_assisted_environment_modal::{
        AgentAssistedEnvironmentModal, AgentAssistedEnvironmentModalEvent,
    },
    delete_environment_confirmation_dialog::{
        DeleteEnvironmentConfirmationDialog, DeleteEnvironmentConfirmationDialogEvent,
    },
    editor_text_colors,
    settings_page::{
        MatchData, PageType, SettingsPageEvent, SettingsPageMeta, SettingsWidget, CONTENT_FONT_SIZE,
    },
    update_environment_form::{
        EnvironmentFormInitArgs, EnvironmentFormValues, GithubAuthRedirectTarget,
        UpdateEnvironmentForm, UpdateEnvironmentFormEvent,
    },
    SettingsSection,
};
use crate::{
    appearance::Appearance,
    editor::{EditorView, PropagateAndNoOpNavigationKeys, SingleLineEditorOptions, TextOptions},
    root_view::CreateEnvironmentArg,
    server::ids::SyncId,
    terminal::view::init_environment::mode_selector::{
        EnvironmentSetupMode, EnvironmentSetupModeSelector, EnvironmentSetupModeSelectorEvent,
    },
    themes::theme::Fill as ThemeFill,
    ui_components::{blended_colors, buttons::icon_button_with_color, icons::Icon},
    view_components::{
        render_copyable_text_field, CopyButtonPlacement, CopyableTextFieldConfig, DismissibleToast,
        COPY_FEEDBACK_DURATION,
    },
    workspace::ToastStack,
};
use instant::Instant;
use pathfinder_geometry::vector::vec2f;
use std::collections::HashMap;
use warp_core::ui::color::blend::Blend;
use warp_core::ui::theme::color::internal_colors;
use warp_editor::editor::NavigationKey;
use warpui::{
    elements::{
        Align, Border, ChildAnchor, Clipped, ConstrainedBox, Container, CornerRadius,
        CrossAxisAlignment, Element, Expanded, Flex, Hoverable, MainAxisAlignment, MainAxisSize,
        MouseStateHandle, OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds,
        Radius, Shrinkable, SizeConstraintCondition, SizeConstraintSwitch, Stack, Text,
    },
    fonts::{Properties, Weight},
    prelude::ChildView,
    ui_components::{
        button::ButtonVariant,
        components::{UiComponent, UiComponentStyles},
    },
    windowing::{self, state::ApplicationStage, WindowManager},
    AppContext, Entity, FocusContext, ModelHandle, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle,
};

mod new_environment_button;
use new_environment_button::NewEnvironmentButtonView;

const PAGE_TITLE_TEXT: &str = "Environments";
const PAGE_DESCRIPTION_TEXT: &str =
    "Warper runs agents locally through OpenRouter. Hosted saved environments are not available.";
const CARD_BORDER_WIDTH: f32 = 1.;
const CARD_PADDING: f32 = 16.;
const CARD_SPACING: f32 = 12.;
const SECTION_SPACING: f32 = 16.;
const TITLE_DESCRIPTION_SPACING: f32 = 6.;
// Match the settings page MAX_PAGE_WIDTH (800px) for consistent alignment
const DROPDOWN_MAX_WIDTH: f32 = 800.;
const EMPTY_STATE_MAX_WIDTH_RATIO: f32 = 0.7;
const EMPTY_STATE_MIN_HEIGHT: f32 = 420.;
const EMPTY_STATE_ROW_VERTICAL_LAYOUT_THRESHOLD: f32 = 360.;
const TOOLBAR_SEARCH_MAX_WIDTH: f32 = 420.;

struct EmptyStateRowConfig {
    icon: Icon,
    title: &'static str,
    badge: Option<&'static str>,
    subtitle: &'static str,
    action_button: Box<dyn Element>,
    compact_action_button: Box<dyn Element>,
    icon_size: f32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EnvironmentsPage {
    #[default]
    List,
    Edit {
        env_id: SyncId,
    },
    Create,
}

/// A view-friendly representation of a saved environment.
#[derive(Clone, Debug)]
struct EnvironmentDisplayData {
    id: SyncId,
    name: String,
    description: Option<String>,
    docker_image: String,
    github_repos: Vec<(String, String)>, // (owner, repo)
    setup_commands: Vec<String>,
}

impl EnvironmentDisplayData {
    fn matches_search_query(&self, query: &str) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return true;
        }

        let needle = query.to_lowercase();

        let haystacks = [
            self.id.to_string(),
            self.name.clone(),
            self.description.clone().unwrap_or_default(),
            self.docker_image.clone(),
        ];

        if haystacks
            .into_iter()
            .any(|field| field.to_lowercase().contains(&needle))
        {
            return true;
        }

        self.github_repos.iter().any(|(owner, repo)| {
            let entry = format!("{owner}/{repo}");
            entry.to_lowercase().contains(&needle)
        })
    }

    /// Format the timestamp text showing last edited and last used times.
    fn format_timestamp_text(&self) -> String {
        "Last used: never".to_string()
    }
}

pub struct EnvironmentsPageView {
    page: PageType<Self>,
    current_page: EnvironmentsPage,
    copy_button_mouse_states: HashMap<SyncId, MouseStateHandle>,
    edit_button_mouse_states: HashMap<SyncId, MouseStateHandle>,
    card_hover_mouse_states: HashMap<SyncId, MouseStateHandle>,
    /// Tracks when each env ID was last copied, for showing checkmark feedback
    copy_feedback_times: HashMap<SyncId, Instant>,
    // List page search state
    search_query: String,
    search_editor: ViewHandle<EditorView>,
    empty_state_github_repos_button_mouse_state: MouseStateHandle,
    empty_state_local_repos_button_mouse_state: MouseStateHandle,
    // Delete confirmation dialog
    delete_confirmation_dialog: ViewHandle<DeleteEnvironmentConfirmationDialog>,
    // Agent-assisted environment creation modal
    agent_assisted_environment_modal: ViewHandle<AgentAssistedEnvironmentModal>,
    // New environment button (search -> tab focus target)
    new_env_button: ViewHandle<NewEnvironmentButtonView>,
    // Mode selector modal for new environment setup
    environment_setup_mode_selector: ViewHandle<EnvironmentSetupModeSelector>,
    is_environment_setup_mode_selector_open: bool,
    // Environment form
    environment_form: ViewHandle<UpdateEnvironmentForm>,
    // Pane configuration for BackingView support
    pane_configuration: ModelHandle<crate::pane_group::pane::PaneConfiguration>,
    // Focus handle for BackingView support
    focus_handle: Option<crate::pane_group::focus_state::PaneFocusHandle>,
}

impl EnvironmentsPageView {
    fn ensure_environment_mouse_states(&mut self, ctx: &mut ViewContext<Self>) {
        let _ = ctx;
    }
    pub fn update_page(&mut self, page: EnvironmentsPage, ctx: &mut ViewContext<Self>) {
        self.current_page = page.clone();

        // Update the environment form component based on the page
        match &page {
            EnvironmentsPage::Edit { env_id } => {
                self.environment_form.update(ctx, |form, ctx| {
                    form.set_mode(
                        EnvironmentFormInitArgs::Edit {
                            env_id: *env_id,
                            initial_values: Box::<EnvironmentFormValues>::default(),
                        },
                        ctx,
                    );
                });
            }
            EnvironmentsPage::Create => {
                // Update form mode to Create
                self.environment_form.update(ctx, |form, ctx| {
                    form.set_mode(EnvironmentFormInitArgs::Create, ctx);
                });
            }
            EnvironmentsPage::List => {
                self.ensure_environment_mouse_states(ctx);
            }
        }

        self.focus(ctx);
        ctx.notify();
    }

    fn create_single_line_editor(
        placeholder: &'static str,
        ctx: &mut ViewContext<Self>,
    ) -> ViewHandle<EditorView> {
        let editor = ctx.add_typed_action_view(|ctx| {
            let appearance = Appearance::as_ref(ctx);
            let options = SingleLineEditorOptions {
                text: TextOptions {
                    font_size_override: Some(appearance.ui_font_size()),
                    font_family_override: Some(appearance.ui_font_family()),
                    text_colors_override: Some(editor_text_colors(appearance)),
                    ..Default::default()
                },
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            let mut editor = EditorView::single_line(options, ctx);
            editor.set_placeholder_text(placeholder, ctx);
            editor
        });
        editor
    }

    fn update_search_editor_text_colors(&mut self, ctx: &mut ViewContext<Self>) {
        let appearance = Appearance::as_ref(ctx);
        let text_colors = editor_text_colors(appearance);
        self.search_editor.update(ctx, |editor, ctx| {
            editor.set_text_colors(text_colors, ctx);
        });
    }

    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&Appearance::handle(ctx), |view, _, _, ctx| {
            view.update_search_editor_text_colors(ctx);
        });
        // Create search editor for list page
        let search_editor = Self::create_single_line_editor("Search environments...", ctx);
        ctx.subscribe_to_view(&search_editor, |me, _, event, ctx| match event {
            crate::editor::Event::Edited(_) => {
                me.search_query = me.search_editor.as_ref(ctx).buffer_text(ctx);
                ctx.notify();
            }
            crate::editor::Event::Escape => {
                me.search_query.clear();
                me.search_editor.update(ctx, |editor, ctx| {
                    editor.clear_buffer_and_reset_undo_stack(ctx);
                });
                ctx.notify();
            }
            crate::editor::Event::Navigate(NavigationKey::Tab) => {
                ctx.focus(&me.new_env_button);
            }
            _ => {}
        });

        let new_env_button = ctx
            .add_typed_action_view(|ctx| NewEnvironmentButtonView::new(search_editor.clone(), ctx));

        let delete_confirmation_dialog =
            ctx.add_typed_action_view(DeleteEnvironmentConfirmationDialog::new);
        ctx.subscribe_to_view(&delete_confirmation_dialog, |me, _, event, ctx| {
            me.handle_delete_confirmation_event(event, ctx);
        });

        let agent_assisted_environment_modal =
            ctx.add_typed_action_view(AgentAssistedEnvironmentModal::new);
        ctx.subscribe_to_view(
            &agent_assisted_environment_modal,
            |me, _, event, ctx| match event {
                AgentAssistedEnvironmentModalEvent::Cancelled => {
                    me.agent_assisted_environment_modal
                        .update(ctx, |modal, ctx| {
                            modal.hide(ctx);
                        });
                    ctx.emit(SettingsPageEvent::AgentAssistedEnvironmentModalToggled {
                        is_open: false,
                    });
                    ctx.notify();
                }
                AgentAssistedEnvironmentModalEvent::Confirmed { repo_paths } => {
                    me.agent_assisted_environment_modal
                        .update(ctx, |modal, ctx| {
                            modal.hide(ctx);
                        });
                    ctx.emit(SettingsPageEvent::AgentAssistedEnvironmentModalToggled {
                        is_open: false,
                    });

                    let arg = CreateEnvironmentArg {
                        repos: repo_paths.clone(),
                    };

                    let window_id = ctx.window_id();
                    let primary_window_and_view = ctx
                        .root_view_id(window_id)
                        .map(|view_id| (window_id, view_id));

                    if let Some((primary_window_id, root_view_id)) = primary_window_and_view {
                        ctx.dispatch_action(
                            primary_window_id,
                            &[root_view_id],
                            "root_view:create_environment_in_existing_window_and_run",
                            &arg,
                            log::Level::Info,
                        );
                    } else {
                        ctx.dispatch_global_action("root_view:create_environment_and_run", arg);
                    }

                    ctx.notify();
                }
            },
        );

        let environment_setup_mode_selector =
            ctx.add_typed_action_view(EnvironmentSetupModeSelector::new);
        ctx.subscribe_to_view(&environment_setup_mode_selector, |me, _, event, ctx| {
            me.handle_environment_setup_mode_selector_event(event, ctx);
        });

        // Create the environment form (starts in Create mode)
        let environment_form = ctx.add_typed_action_view(|ctx| {
            UpdateEnvironmentForm::new(EnvironmentFormInitArgs::Create, ctx)
        });
        ctx.subscribe_to_view(&environment_form, |me, _, event, ctx| {
            me.handle_environment_form_event(event, ctx);
        });

        // Refetch GitHub repos when the app regains focus, in case the user
        // just completed the OAuth flow in the browser.
        ctx.subscribe_to_model(&WindowManager::handle(ctx), |me, _, evt, ctx| {
            let windowing::StateEvent::ValueChanged { current, previous } = evt;
            if previous.stage == ApplicationStage::Inactive
                && current.stage == ApplicationStage::Active
                && me
                    .environment_form
                    .as_ref(ctx)
                    .github_dropdown_state()
                    .auth_url
                    .is_some()
            {
                me.environment_form.update(ctx, |form, ctx| {
                    form.fetch_github_repos(ctx);
                });
            }
        });

        let copy_button_mouse_states = HashMap::new();
        let edit_button_mouse_states = HashMap::new();
        let card_hover_mouse_states = HashMap::new();

        // Create pane configuration for BackingView support
        let pane_configuration =
            ctx.add_model(|_| crate::pane_group::pane::PaneConfiguration::new("Environments"));

        let mut view = Self {
            page: PageType::new_monolith(
                EnvironmentsPageWidget,
                None, // Title rendered conditionally in widget
                true, /* is_dual_scrollable */
            ),
            current_page: EnvironmentsPage::default(),
            copy_button_mouse_states,
            edit_button_mouse_states,
            card_hover_mouse_states,
            copy_feedback_times: HashMap::new(),
            search_query: String::new(),
            search_editor,
            empty_state_github_repos_button_mouse_state: MouseStateHandle::default(),
            empty_state_local_repos_button_mouse_state: MouseStateHandle::default(),
            delete_confirmation_dialog,
            agent_assisted_environment_modal,
            new_env_button,
            environment_setup_mode_selector,
            is_environment_setup_mode_selector_open: false,
            environment_form,
            pane_configuration,
            focus_handle: None,
        };

        view.ensure_environment_mouse_states(ctx);
        view.update_search_editor_text_colors(ctx);

        view
    }

    /// Returns the current page/mode of the environments view.
    pub fn current_page(&self) -> &EnvironmentsPage {
        &self.current_page
    }

    /// Returns the environment setup mode selector view handle for tab-level rendering.
    pub fn environment_setup_mode_selector_handle(
        &self,
    ) -> Option<&ViewHandle<EnvironmentSetupModeSelector>> {
        self.is_environment_setup_mode_selector_open
            .then_some(&self.environment_setup_mode_selector)
    }

    /// Returns the agent-assisted environment modal view handle for tab-level rendering.
    pub fn agent_assisted_environment_modal_handle(
        &self,
        app: &AppContext,
    ) -> Option<&ViewHandle<AgentAssistedEnvironmentModal>> {
        self.agent_assisted_environment_modal
            .as_ref(app)
            .is_visible()
            .then_some(&self.agent_assisted_environment_modal)
    }

    /// Returns the pane configuration for BackingView support.
    pub fn pane_configuration(&self) -> ModelHandle<crate::pane_group::pane::PaneConfiguration> {
        self.pane_configuration.clone()
    }
    pub fn set_github_auth_redirect_target(
        &mut self,
        target: GithubAuthRedirectTarget,
        ctx: &mut ViewContext<Self>,
    ) {
        self.environment_form
            .update(ctx, |form, _| form.set_github_auth_redirect_target(target));
    }

    /// Focus the environments page view.
    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) {
        // Focus the search editor when on the list page, otherwise focus the form
        match &self.current_page {
            EnvironmentsPage::List => {
                ctx.focus(&self.search_editor);
            }
            EnvironmentsPage::Create | EnvironmentsPage::Edit { .. } => {
                ctx.focus(&self.environment_form);
            }
        }
    }

    fn show_error_toast(&self, message: String, ctx: &mut ViewContext<Self>) {
        let window_id = ctx.window_id();
        ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            toast_stack.add_ephemeral_toast(DismissibleToast::error(message), window_id, ctx);
        });
    }

    fn delete_environment(&mut self, env_id: SyncId, ctx: &mut ViewContext<Self>) {
        let _ = env_id;
        self.update_page(EnvironmentsPage::List, ctx);
    }

    fn handle_delete_confirmation_event(
        &mut self,
        event: &DeleteEnvironmentConfirmationDialogEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            DeleteEnvironmentConfirmationDialogEvent::Cancel => {
                self.delete_confirmation_dialog.update(ctx, |dialog, ctx| {
                    dialog.hide(ctx);
                });
                ctx.notify();
            }
            DeleteEnvironmentConfirmationDialogEvent::Confirm(env_id) => {
                let env_id = *env_id;
                self.delete_confirmation_dialog.update(ctx, |dialog, ctx| {
                    dialog.hide(ctx);
                });
                self.delete_environment(env_id, ctx);
            }
        }
    }

    fn handle_environment_form_event(
        &mut self,
        event: &UpdateEnvironmentFormEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            UpdateEnvironmentFormEvent::Created { environment } => {
                let _ = environment;
                self.show_error_toast(
                    "Saved hosted environments are not available in Warper.".to_string(),
                    ctx,
                );
                self.update_page(EnvironmentsPage::List, ctx);
            }
            UpdateEnvironmentFormEvent::Updated {
                env_id,
                environment,
            } => {
                let _ = (env_id, environment);
                self.show_error_toast(
                    "Saved hosted environments are not available in Warper.".to_string(),
                    ctx,
                );
                self.update_page(EnvironmentsPage::List, ctx);
            }
            UpdateEnvironmentFormEvent::DeleteRequested { env_id } => {
                self.delete_environment(*env_id, ctx);
            }
            UpdateEnvironmentFormEvent::Cancelled => {
                // Navigate back to list
                self.update_page(EnvironmentsPage::List, ctx);
            }
        }
    }

    fn open_agent_assisted_environment_modal(&mut self, ctx: &mut ViewContext<Self>) {
        self.agent_assisted_environment_modal
            .update(ctx, |modal, ctx| {
                modal.show(ctx);
            });
        ctx.emit(SettingsPageEvent::AgentAssistedEnvironmentModalToggled { is_open: true });
        ctx.notify();
    }

    fn open_environment_setup_mode_selector(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_environment_setup_mode_selector_open {
            return;
        }

        self.is_environment_setup_mode_selector_open = true;
        ctx.focus(&self.environment_setup_mode_selector);
        ctx.emit(SettingsPageEvent::EnvironmentSetupModeSelectorToggled { is_open: true });
        ctx.notify();
    }

    fn close_environment_setup_mode_selector(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.is_environment_setup_mode_selector_open {
            return;
        }

        self.is_environment_setup_mode_selector_open = false;
        ctx.emit(SettingsPageEvent::EnvironmentSetupModeSelectorToggled { is_open: false });
        ctx.notify();
    }

    fn handle_environment_setup_mode_selector_event(
        &mut self,
        event: &EnvironmentSetupModeSelectorEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            EnvironmentSetupModeSelectorEvent::Selected(mode) => {
                self.close_environment_setup_mode_selector(ctx);

                match mode {
                    EnvironmentSetupMode::RemoteGitHub => {
                        self.open_agent_assisted_environment_modal(ctx);
                    }
                    EnvironmentSetupMode::LocalRepositories => {
                        self.open_agent_assisted_environment_modal(ctx);
                    }
                }
            }
            EnvironmentSetupModeSelectorEvent::Dismissed => {
                self.close_environment_setup_mode_selector(ctx);
                self.focus(ctx);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum EnvironmentsPageAction {
    OpenEditPage(SyncId),
    RetryFetchGithubRepos,
    OpenUrl(String),
    StartGithubAuth,
    CopyEnvId(SyncId, String),
    OpenCreatePage,
    OpenAgentAssistedCreateModal,
    OpenEnvironmentSetupModeSelector,
}
impl Entity for EnvironmentsPageView {
    type Event = SettingsPageEvent;
}

impl TypedActionView for EnvironmentsPageView {
    type Action = EnvironmentsPageAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            EnvironmentsPageAction::OpenEditPage(env_id) => {
                self.update_page(EnvironmentsPage::Edit { env_id: *env_id }, ctx);
            }
            EnvironmentsPageAction::RetryFetchGithubRepos => {
                self.environment_form.update(ctx, |form, ctx| {
                    form.fetch_github_repos(ctx);
                });
            }
            EnvironmentsPageAction::OpenUrl(url) => {
                ctx.open_url(url);
            }
            EnvironmentsPageAction::StartGithubAuth => {
                self.environment_form.update(ctx, |form, ctx| {
                    form.start_github_auth(ctx);
                });
            }
            EnvironmentsPageAction::CopyEnvId(sync_id, env_id_string) => {
                ctx.clipboard()
                    .write(warpui::clipboard::ClipboardContent::plain_text(
                        env_id_string.clone(),
                    ));
                // Track when this was copied for feedback
                self.copy_feedback_times.insert(*sync_id, Instant::now());
                // Schedule a re-render after the feedback duration to clear the checkmark
                let duration = COPY_FEEDBACK_DURATION;
                ctx.spawn(
                    async move {
                        warpui::r#async::Timer::after(duration).await;
                    },
                    |me, _, ctx| {
                        ctx.notify();
                        // Clean up old entries
                        me.copy_feedback_times
                            .retain(|_, time| time.elapsed() < COPY_FEEDBACK_DURATION);
                    },
                );
                ctx.notify();
            }
            EnvironmentsPageAction::OpenCreatePage => {
                self.update_page(EnvironmentsPage::Create, ctx);
            }
            EnvironmentsPageAction::OpenAgentAssistedCreateModal => {
                self.open_agent_assisted_environment_modal(ctx);
            }
            EnvironmentsPageAction::OpenEnvironmentSetupModeSelector => {
                self.open_environment_setup_mode_selector(ctx);
            }
        }
    }

    fn action_accessibility_contents(
        &mut self,
        _action: &Self::Action,
        _ctx: &mut ViewContext<Self>,
    ) -> warpui::accessibility::ActionAccessibilityContent {
        warpui::accessibility::ActionAccessibilityContent::default()
    }
}

impl View for EnvironmentsPageView {
    fn ui_name() -> &'static str {
        "EnvironmentsPage"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            self.focus(ctx);
        }
    }
}

struct EnvironmentsPageWidget;

struct EnvironmentCardRenderState<'a> {
    copy_button_mouse_states: &'a HashMap<SyncId, MouseStateHandle>,
    edit_button_mouse_states: &'a HashMap<SyncId, MouseStateHandle>,
    card_hover_mouse_states: &'a HashMap<SyncId, MouseStateHandle>,
    copy_feedback_times: &'a HashMap<SyncId, Instant>,
}

impl SettingsWidget for EnvironmentsPageWidget {
    type View = EnvironmentsPageView;

    fn search_terms(&self) -> &str {
        "environments environment ambient agents github warp assisted manual configuration"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        match &view.current_page {
            EnvironmentsPage::List => Self::render_list_page(view, appearance, app),
            EnvironmentsPage::Edit { .. } => Self::render_edit_page(view),
            EnvironmentsPage::Create => Self::render_create_page(view),
        }
    }
}

impl EnvironmentsPageWidget {
    fn render_list_page(
        view: &EnvironmentsPageView,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let mut page = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_main_axis_size(MainAxisSize::Min)
            .with_spacing(SECTION_SPACING);

        // Page title + description
        let title = Text::new(
            PAGE_TITLE_TEXT,
            appearance.ui_font_family(),
            appearance.ui_font_size() * 1.5,
        )
        .with_style(Properties::default().weight(Weight::Bold))
        .with_color(theme.active_ui_text_color().into())
        .finish();

        let description = appearance
            .ui_builder()
            .paragraph(PAGE_DESCRIPTION_TEXT)
            .with_style(UiComponentStyles {
                font_color: Some(appearance.theme().nonactive_ui_text_color().into()),
                font_size: Some(CONTENT_FONT_SIZE),
                ..Default::default()
            })
            .build()
            .finish();

        let header = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Start)
            .with_spacing(TITLE_DESCRIPTION_SPACING)
            .with_child(title)
            .with_child(description)
            .finish();

        page.add_child(header);

        let mut environments = Vec::<EnvironmentDisplayData>::new();

        let has_any_environments = !environments.is_empty();

        if !view.search_query.trim().is_empty() {
            environments.retain(|env| env.matches_search_query(&view.search_query));
        }

        if !has_any_environments {
            page.add_child(Self::render_empty_state(view, appearance, app));
        } else {
            // Toolbar row with search (left) and New environment button (right)
            let toolbar_row = Self::render_toolbar_row(view, appearance);
            page.add_child(toolbar_row);

            if environments.is_empty() {
                page.add_child(Self::render_no_matches_state(appearance));
            } else {
                let card_render_state = EnvironmentCardRenderState {
                    copy_button_mouse_states: &view.copy_button_mouse_states,
                    edit_button_mouse_states: &view.edit_button_mouse_states,
                    card_hover_mouse_states: &view.card_hover_mouse_states,
                    copy_feedback_times: &view.copy_feedback_times,
                };

                if environments.is_empty() {
                    page.add_child(Self::render_no_matches_state(appearance));
                } else {
                    page.add_child(Self::render_personal_section(
                        &environments,
                        &card_render_state,
                        appearance,
                        app,
                    ));
                }
            }
        }

        page.finish()
    }

    fn render_toolbar_row(
        view: &EnvironmentsPageView,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        // Main toolbar row: search (left), button (right)
        //
        // Make the search bar flexible so it can shrink when the window gets narrow.
        // Without this, the search bar is laid out with an unbounded width constraint in a row,
        // so it happily takes its max width and can overflow/overlap on small screens.
        Flex::row()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(CARD_SPACING)
            .with_child(Shrinkable::new(1., Self::render_search_bar(view, appearance)).finish())
            .with_child(ChildView::new(&view.new_env_button).finish())
            .finish()
    }

    fn render_search_bar(view: &EnvironmentsPageView, appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();

        // Match the "New environment" button height, which is roughly:
        // ui_font_size + (2 * vertical_padding)
        let toolbar_height = appearance.ui_font_size() + 12.;
        let icon_size = appearance.ui_font_size();

        let search_icon = ConstrainedBox::new(
            Icon::Search
                .to_warpui_icon(blended_colors::text_sub(theme, theme.surface_2()).into())
                .finish(),
        )
        .with_width(icon_size)
        .with_height(icon_size)
        .finish();

        let editor =
            Container::new(Clipped::new(ChildView::new(&view.search_editor).finish()).finish())
                .finish();

        let input_contents = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(8.)
            .with_child(search_icon)
            .with_child(Expanded::new(1., editor).finish())
            .finish();

        // Use a fixed height + vertical centering so the icon and text stay aligned.
        let centered_contents = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_child(input_contents)
            .finish();

        ConstrainedBox::new(
            Container::new(
                ConstrainedBox::new(centered_contents)
                    .with_height(toolbar_height)
                    .finish(),
            )
            .with_horizontal_padding(12.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_border(Border::all(CARD_BORDER_WIDTH).with_border_fill(theme.outline()))
            .with_background(theme.surface_2())
            .finish(),
        )
        .with_max_width(TOOLBAR_SEARCH_MAX_WIDTH)
        .finish()
    }

    fn render_no_matches_state(appearance: &Appearance) -> Box<dyn Element> {
        let theme = appearance.theme();
        Container::new(
            Text::new(
                "No environments match your search.",
                appearance.ui_font_family(),
                appearance.ui_font_size(),
            )
            .with_color(theme.nonactive_ui_text_color().into())
            .finish(),
        )
        .with_uniform_padding(12.)
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
        .with_border(Border::all(CARD_BORDER_WIDTH).with_border_fill(theme.outline()))
        .with_background(theme.surface_2())
        .finish()
    }

    fn render_personal_section(
        environments: &[EnvironmentDisplayData],
        card_render_state: &EnvironmentCardRenderState<'_>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        // Keep header-to-card spacing smaller than the overall page/section spacing.
        const HEADER_TO_LIST_SPACING: f32 = 8.;

        Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(HEADER_TO_LIST_SPACING)
            .with_child(Self::render_overline_header("Personal", appearance))
            .with_child(Self::render_environments_list(
                environments,
                card_render_state,
                appearance,
                app,
            ))
            .finish()
    }

    fn render_overline_header(text: &str, appearance: &Appearance) -> Box<dyn Element> {
        Text::new(
            text.to_uppercase(),
            appearance.overline_font_family(),
            appearance.overline_font_size(),
        )
        .with_color(blended_colors::text_sub(
            appearance.theme(),
            appearance.theme().surface_2(),
        ))
        .finish()
    }

    fn render_edit_page(view: &EnvironmentsPageView) -> Box<dyn Element> {
        // Wrap the form in a Stack to overlay the confirmation dialog
        let mut stack = Stack::new();
        stack.add_child(ChildView::new(&view.environment_form).finish());
        stack.add_positioned_overlay_child(
            ChildView::new(&view.delete_confirmation_dialog).finish(),
            OffsetPositioning::offset_from_parent(
                vec2f(0., 0.),
                ParentOffsetBounds::WindowByPosition,
                ParentAnchor::Center,
                ChildAnchor::Center,
            ),
        );
        Clipped::new(stack.finish()).finish()
    }

    fn render_create_page(view: &EnvironmentsPageView) -> Box<dyn Element> {
        Clipped::new(ChildView::new(&view.environment_form).finish()).finish()
    }

    fn render_empty_state(
        view: &EnvironmentsPageView,
        appearance: &Appearance,
        _app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let icon_size = appearance.ui_font_size() * 1.3;

        let local_repos_button = Self::render_empty_state_button(
            appearance,
            "Launch agent",
            ButtonVariant::Accent,
            view.empty_state_local_repos_button_mouse_state.clone(),
            true,
            Some(EnvironmentsPageAction::OpenAgentAssistedCreateModal),
        );
        let local_repos_button_compact = Self::render_empty_state_button(
            appearance,
            "Launch agent",
            ButtonVariant::Accent,
            view.empty_state_local_repos_button_mouse_state.clone(),
            true,
            Some(EnvironmentsPageAction::OpenAgentAssistedCreateModal),
        );

        let local_repos_row = Self::render_empty_state_row(
            appearance,
            EmptyStateRowConfig {
                icon: Icon::Terminal,
                title: "Local agent",
                badge: None,
                subtitle: "Choose a local project and run the agent directly from this machine",
                action_button: local_repos_button,
                compact_action_button: local_repos_button_compact,
                icon_size,
            },
        );

        let rows = ConstrainedBox::new(
            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(8.)
                .with_child(local_repos_row)
                .finish(),
        )
        .with_max_width(DROPDOWN_MAX_WIDTH * EMPTY_STATE_MAX_WIDTH_RATIO)
        .finish();

        let header = Text::new(
            "Hosted environments are not available in Warper.",
            appearance.ui_font_family(),
            appearance.ui_font_size() * 1.1,
        )
        .with_style(Properties::default().weight(Weight::Semibold))
        .with_color(theme.active_ui_text_color().into())
        .finish();

        let subheader = Text::new(
            "Use the local agent flow to work with projects on this machine.",
            appearance.ui_font_family(),
            appearance.ui_font_size() * 0.95,
        )
        .with_color(theme.nonactive_ui_text_color().into())
        .soft_wrap(true)
        .finish();

        let constrained_subheader = ConstrainedBox::new(subheader)
            .with_max_width(DROPDOWN_MAX_WIDTH * EMPTY_STATE_MAX_WIDTH_RATIO)
            .finish();

        let content = Flex::column()
            .with_main_axis_size(MainAxisSize::Max)
            .with_main_axis_alignment(MainAxisAlignment::Center)
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_spacing(4.)
            .with_child(header)
            .with_child(constrained_subheader)
            .with_child(Container::new(rows).with_margin_top(8.).finish())
            .finish();

        ConstrainedBox::new(
            Container::new(content)
                .with_uniform_padding(24.)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
                .with_border(Border::all(CARD_BORDER_WIDTH).with_border_fill(theme.outline()))
                .with_background(theme.surface_2())
                .finish(),
        )
        .with_height(EMPTY_STATE_MIN_HEIGHT)
        .finish()
    }

    fn render_empty_state_row(
        appearance: &Appearance,
        config: EmptyStateRowConfig,
    ) -> Box<dyn Element> {
        let EmptyStateRowConfig {
            icon,
            title,
            badge,
            subtitle,
            action_button,
            compact_action_button,
            icon_size,
        } = config;
        let theme = appearance.theme();
        let build_icon = || {
            Container::new(
                ConstrainedBox::new(icon.to_warpui_icon(theme.active_ui_text_color()).finish())
                    .with_width(icon_size)
                    .with_height(icon_size)
                    .finish(),
            )
            .with_uniform_padding(8.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_background(theme.surface_3())
            .finish()
        };

        let build_text_column = || {
            let mut title_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_spacing(6.)
                .with_child(
                    Text::new(
                        title,
                        appearance.ui_font_family(),
                        appearance.ui_font_size(),
                    )
                    .with_style(Properties::default().weight(Weight::Semibold))
                    .with_color(theme.active_ui_text_color().into())
                    .finish(),
                );

            if let Some(badge) = badge {
                let badge = Container::new(
                    Text::new(
                        badge,
                        appearance.ui_font_family(),
                        appearance.ui_font_size() * 0.85,
                    )
                    .with_color(theme.nonactive_ui_text_color().into())
                    .finish(),
                )
                .with_horizontal_padding(6.)
                .with_vertical_padding(2.)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(10.)))
                .with_background(theme.surface_3())
                .finish();
                title_row.add_child(badge);
            }

            Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_spacing(2.)
                .with_child(title_row.finish())
                .with_child(
                    Text::new(
                        subtitle,
                        appearance.ui_font_family(),
                        appearance.ui_font_size() * 0.9,
                    )
                    .with_color(theme.nonactive_ui_text_color().into())
                    .soft_wrap(true)
                    .finish(),
                )
                .finish()
        };

        let row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_main_axis_alignment(MainAxisAlignment::Start)
            .with_main_axis_size(MainAxisSize::Max)
            .with_spacing(12.)
            .with_child(build_icon())
            .with_child(Expanded::new(1., build_text_column()).finish())
            .with_child(action_button)
            .finish();

        let horizontal = Container::new(row)
            .with_uniform_padding(12.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_border(Border::all(CARD_BORDER_WIDTH).with_border_fill(theme.outline()))
            .with_background(theme.surface_3())
            .finish();

        let compact_row = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(8.)
            .with_child(Align::new(build_icon()).finish())
            .with_child(build_text_column())
            .with_child(Align::new(compact_action_button).finish())
            .finish();

        let vertical = Container::new(compact_row)
            .with_uniform_padding(12.)
            .with_corner_radius(CornerRadius::with_all(Radius::Pixels(6.)))
            .with_border(Border::all(CARD_BORDER_WIDTH).with_border_fill(theme.outline()))
            .with_background(theme.surface_3())
            .finish();

        SizeConstraintSwitch::new(
            horizontal,
            vec![(
                SizeConstraintCondition::WidthLessThan(EMPTY_STATE_ROW_VERTICAL_LAYOUT_THRESHOLD),
                vertical,
            )],
        )
        .finish()
    }

    fn render_empty_state_button(
        appearance: &Appearance,
        label: &str,
        variant: ButtonVariant,
        mouse_state: MouseStateHandle,
        enabled: bool,
        action: Option<EnvironmentsPageAction>,
    ) -> Box<dyn Element> {
        let mut button = appearance
            .ui_builder()
            .button(variant, mouse_state)
            .with_centered_text_label(label.to_string())
            .build();

        if !enabled {
            button = button.disable();
            return button.finish();
        }

        if let Some(action) = action {
            return button
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(action.clone());
                })
                .finish();
        }

        button.finish()
    }

    fn render_environments_list(
        environments: &[EnvironmentDisplayData],
        card_render_state: &EnvironmentCardRenderState<'_>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut list = Flex::column()
            .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
            .with_spacing(CARD_SPACING);

        for environment in environments {
            list.add_child(Self::render_environment_card(
                environment,
                card_render_state,
                appearance,
                app,
            ));
        }

        list.finish()
    }

    fn render_environment_card(
        environment: &EnvironmentDisplayData,
        card_render_state: &EnvironmentCardRenderState<'_>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let env_id = environment.id;

        // Get card hover state
        let card_hover_mouse_state = card_render_state
            .card_hover_mouse_states
            .get(&env_id)
            .cloned()
            .unwrap_or_else(MouseStateHandle::default);

        // Clone data needed for the closure
        let env_name = environment.name.clone();
        let env_description = environment.description.clone();
        let env_docker_image = environment.docker_image.clone();
        let env_github_repos = environment.github_repos.clone();
        let env_setup_commands = environment.setup_commands.clone();
        let timestamp_text = environment.format_timestamp_text();
        let env_id_str = env_id.to_string();
        let copy_button_mouse_state = card_render_state
            .copy_button_mouse_states
            .get(&env_id)
            .cloned()
            .unwrap_or_else(MouseStateHandle::default);
        let edit_button_mouse_state = card_render_state
            .edit_button_mouse_states
            .get(&env_id)
            .cloned()
            .unwrap_or_else(MouseStateHandle::default);

        let last_copied_at = card_render_state.copy_feedback_times.get(&env_id).copied();

        Hoverable::new(card_hover_mouse_state, move |state| {
            // Render the Env ID row with copy button - must be inside the closure
            // since it returns a Box<dyn Element> that can only be consumed once
            let env_id_str_copy = env_id_str.clone();
            let env_id_with_copy = render_copyable_text_field(
                CopyableTextFieldConfig::new(format!("Env ID: {}", env_id_str.clone()))
                    .with_font_size(appearance.ui_font_size() * 0.9)
                    .with_text_color(blended_colors::text_sub(theme, theme.surface_1()))
                    .with_icon_size(12.)
                    .with_mouse_state(copy_button_mouse_state.clone())
                    .with_last_copied_at(last_copied_at.as_ref())
                    .with_copy_button_placement(CopyButtonPlacement::NextToText),
                move |ctx| {
                    ctx.dispatch_typed_action(EnvironmentsPageAction::CopyEnvId(
                        env_id,
                        env_id_str_copy.clone(),
                    ));
                },
                app,
            );
            // Content column with all the card information
            let mut content_column = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(8.);

            // Name (title) - selectable
            content_column.add_child(
                Text::new_inline(
                    env_name.clone(),
                    appearance.ui_font_family(),
                    appearance.ui_font_size(),
                )
                .with_style(Properties::default().weight(Weight::Semibold))
                .with_color(theme.active_ui_text_color().into())
                .with_selectable(true)
                .finish(),
            );

            // Description (if present) - lighter than other details
            if let Some(description) = &env_description {
                if !description.is_empty() {
                    content_column.add_child(
                        Text::new(
                            description.clone(),
                            appearance.ui_font_family(),
                            appearance.ui_font_size(),
                        )
                        .soft_wrap(true)
                        .with_color(
                            theme
                                .background()
                                .blend(&theme.foreground().with_opacity(80))
                                .into(),
                        )
                        .with_selectable(true)
                        .finish(),
                    );
                }
            }

            let mut details_parts = vec![format!("Image: {}", env_docker_image)];

            if !env_github_repos.is_empty() {
                let repos_text = env_github_repos
                    .iter()
                    .map(|(owner, repo)| format!("{}/{}", owner, repo))
                    .collect::<Vec<_>>()
                    .join(", ");
                details_parts.push(format!("Repos: {}", repos_text));
            }

            if !env_setup_commands.is_empty() {
                let commands_text = env_setup_commands.join(", ");
                details_parts.push(format!("Setup commands: {}", commands_text));
            }

            // Create details section with Env ID on first line and other details below
            let mut details_section = Flex::column()
                .with_cross_axis_alignment(CrossAxisAlignment::Stretch)
                .with_spacing(4.);

            // Add Env ID with copy button
            details_section.add_child(env_id_with_copy);

            // Add other details on a new line - selectable
            let details_text = details_parts.join(" · ");
            details_section.add_child(
                Text::new(
                    details_text,
                    appearance.ui_font_family(),
                    appearance.ui_font_size() * 0.9,
                )
                .soft_wrap(true)
                .with_color(blended_colors::text_sub(theme, theme.surface_1()))
                .with_selectable(true)
                .finish(),
            );

            let timestamp_color = blended_colors::text_sub(theme, theme.surface_1());

            let timestamp_row = Text::new_inline(
                timestamp_text,
                appearance.ui_font_family(),
                appearance.ui_font_size() * 0.9,
            )
            .with_color(timestamp_color)
            .with_selectable(true)
            .finish();

            details_section.add_child(timestamp_row);

            content_column.add_child(details_section.finish());

            // Main card row with content on left and edit button on right
            let mut card_row = Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Start)
                .with_spacing(12.);

            // Use Expanded to make content take available space, pushing button to the right
            card_row.add_child(Expanded::new(1., content_column.finish()).finish());

            // Action buttons - always rendered to maintain consistent layout,
            // but use transparent color when not hovering
            let is_card_hovered = state.is_hovered();
            let icon_color: ThemeFill = if is_card_hovered {
                theme.foreground()
            } else {
                ThemeFill::Solid(warpui::color::ColorU::transparent_black())
            };

            let edit_ui_builder = appearance.ui_builder().clone();
            let mut edit_button = icon_button_with_color(
                appearance,
                Icon::Pencil,
                false,
                edit_button_mouse_state.clone(),
                icon_color,
            );
            // Only show tooltip when card is hovered
            if is_card_hovered {
                edit_button = edit_button.with_tooltip(move || {
                    edit_ui_builder
                        .tool_tip("Edit".to_string())
                        .build()
                        .finish()
                });
            }
            let edit_button_element = edit_button
                .build()
                .on_click(move |ctx, _, _| {
                    ctx.dispatch_typed_action(EnvironmentsPageAction::OpenEditPage(env_id));
                })
                .finish();
            card_row.add_child(edit_button_element);

            // Use translucent foreground overlays so the background shows through at rest and on hover.
            let background = if state.is_hovered() {
                internal_colors::fg_overlay_2(theme)
            } else {
                internal_colors::fg_overlay_1(theme)
            };

            Container::new(card_row.finish())
                .with_uniform_padding(CARD_PADDING)
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(8.)))
                .with_background(background)
                .with_border(
                    Border::all(CARD_BORDER_WIDTH)
                        .with_border_fill(internal_colors::fg_overlay_3(theme)),
                )
                .finish()
        })
        .finish()
    }
}

impl SettingsPageMeta for EnvironmentsPageView {
    fn section() -> SettingsSection {
        SettingsSection::Account
    }
    fn on_page_selected(&mut self, _allow_steal_focus: bool, ctx: &mut ViewContext<Self>) {
        self.environment_form.update(ctx, |form, ctx| {
            form.fetch_github_repos(ctx);
        });
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

use crate::pane_group::{
    focus_state::PaneFocusHandle,
    pane::{
        view::{HeaderContent, HeaderRenderContext},
        BackingView,
    },
};

impl BackingView for EnvironmentsPageView {
    type PaneHeaderOverflowMenuAction = EnvironmentsPageAction;
    type CustomAction = ();
    type AssociatedData = ();

    fn handle_pane_header_overflow_menu_action(
        &mut self,
        action: &Self::PaneHeaderOverflowMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        self.handle_action(action, ctx);
    }

    fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(SettingsPageEvent::Pane(
            super::settings_page::PaneEventWrapper::Close,
        ));
    }

    fn focus_contents(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus(ctx);
    }

    fn render_header_content(
        &self,
        _ctx: &HeaderRenderContext<'_>,
        _app: &AppContext,
    ) -> HeaderContent {
        HeaderContent::simple("Environments")
    }

    fn set_focus_handle(&mut self, focus_handle: PaneFocusHandle, _ctx: &mut ViewContext<Self>) {
        self.focus_handle = Some(focus_handle);
        // Use a lower minimum width when used as a pane to allow narrow layouts.
        // This affects when the SettingsPage switches into horizontal-scroll mode.
        self.page.set_min_page_width(260.);
    }
}
