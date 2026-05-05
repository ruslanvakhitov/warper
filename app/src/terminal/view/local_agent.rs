use std::sync::Arc;

use instant::Instant;
use warp_cli::agent::Harness;
use warpui::prelude::Empty;
use warpui::{
    AppContext, Element, Entity, EntityId, ModelContext, ModelHandle, TypedActionView, View,
    ViewContext,
};

use crate::ai::agent::conversation::{AIConversationId, AmbientAgentTaskId};
use crate::server::ids::SyncId;
use crate::terminal::input::MenuPositioningProvider;

#[derive(Debug, Clone)]
pub struct AgentProgress {
    pub spawned_at: Instant,
    pub claimed_at: Option<Instant>,
    pub harness_started_at: Option<Instant>,
    pub stopped_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Status {
    NotLocalAgent,
}

pub struct LocalAgentViewModel {
    terminal_view_id: EntityId,
    has_parent_terminal: bool,
    environment_id: Option<SyncId>,
    conversation_id: Option<AIConversationId>,
    harness: Harness,
    setup_commands_state: SetupCommandState,
    has_inserted_user_query_block: bool,
}

pub type AmbientAgentViewModel = LocalAgentViewModel;
pub type AmbientAgentViewModelEvent = LocalAgentViewModelEvent;

impl LocalAgentViewModel {
    pub fn new(
        terminal_view_id: EntityId,
        has_parent_terminal: bool,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            terminal_view_id,
            has_parent_terminal,
            environment_id: None,
            conversation_id: None,
            harness: Harness::Unknown,
            setup_commands_state: SetupCommandState::default(),
            has_inserted_user_query_block: false,
        }
    }

    pub fn setup_command_state(&self) -> &SetupCommandState {
        &self.setup_commands_state
    }

    pub fn setup_command_state_mut(&mut self) -> &mut SetupCommandState {
        &mut self.setup_commands_state
    }

    pub fn set_setup_command_visibility(&mut self, is_visible: bool, ctx: &mut ModelContext<Self>) {
        if is_visible != self.setup_commands_state.should_expand() {
            self.setup_commands_state.set_should_expand(is_visible);
            ctx.emit(LocalAgentViewModelEvent::UpdatedSetupCommandVisibility);
        }
    }

    pub fn agent_progress(&self) -> Option<&AgentProgress> {
        None
    }

    pub fn selected_environment_id(&self) -> Option<&SyncId> {
        self.environment_id.as_ref()
    }

    pub fn selected_harness(&self) -> Harness {
        if self.harness == Harness::Oz {
            Harness::Unknown
        } else {
            self.harness
        }
    }

    pub fn set_harness(&mut self, harness: Harness, ctx: &mut ModelContext<Self>) {
        let harness = if harness == Harness::Oz {
            Harness::Unknown
        } else {
            harness
        };
        if self.harness != harness {
            self.harness = harness;
            ctx.emit(LocalAgentViewModelEvent::HarnessSelected);
        }
    }

    pub fn is_third_party_harness(&self) -> bool {
        false
    }

    pub fn harness_command_started(&self) -> bool {
        false
    }

    pub fn mark_harness_command_started(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn set_environment_id(
        &mut self,
        environment_id: Option<SyncId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.environment_id = environment_id;
        ctx.emit(LocalAgentViewModelEvent::EnvironmentSelected);
    }

    pub fn is_ambient_agent(&self) -> bool {
        false
    }

    pub fn task_id(&self) -> Option<AmbientAgentTaskId> {
        None
    }

    pub fn has_inserted_cloud_mode_user_query_block(&self) -> bool {
        self.has_inserted_user_query_block
    }

    pub fn set_has_inserted_cloud_mode_user_query_block(&mut self, has_inserted: bool) {
        self.has_inserted_user_query_block = has_inserted;
    }

    pub fn has_parent_terminal(&self) -> bool {
        self.has_parent_terminal
    }

    pub fn set_has_parent_terminal(&mut self, has_parent: bool) {
        self.has_parent_terminal = has_parent;
    }

    pub fn is_in_setup(&self) -> bool {
        false
    }

    pub fn is_configuring_ambient_agent(&self) -> bool {
        false
    }

    pub fn is_waiting_for_session(&self) -> bool {
        false
    }

    pub fn is_failed(&self) -> bool {
        false
    }

    pub fn is_cancelled(&self) -> bool {
        false
    }

    pub fn is_needs_github_auth(&self) -> bool {
        false
    }

    pub fn is_agent_running(&self) -> bool {
        false
    }

    pub fn should_show_status_footer(&self) -> bool {
        false
    }

    pub fn error_message(&self) -> Option<&str> {
        None
    }

    pub fn github_auth_url(&self) -> Option<&str> {
        None
    }

    pub fn github_auth_error_message(&self) -> Option<&str> {
        None
    }

    pub fn enter_setup(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.notify();
    }

    pub fn enter_composing_from_setup(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.notify();
    }

    pub fn enter_viewing_existing_session(
        &mut self,
        _task_id: AmbientAgentTaskId,
        ctx: &mut ModelContext<Self>,
    ) {
        ctx.notify();
    }

    pub fn status(&self) -> Status {
        Status::NotLocalAgent
    }

    pub fn reset_status(&mut self, ctx: &mut ModelContext<Self>) {
        self.environment_id = None;
        self.conversation_id = None;
        self.has_inserted_user_query_block = false;
        ctx.notify();
    }

    pub fn set_conversation_id(&mut self, id: Option<AIConversationId>) {
        self.conversation_id = id;
    }

    pub fn cancel_task(&mut self, ctx: &mut ModelContext<Self>) {
        ctx.notify();
    }

    pub fn terminal_view_id(&self) -> EntityId {
        self.terminal_view_id
    }
}

#[derive(Debug, Clone)]
pub enum LocalAgentViewModelEvent {
    EnteredSetupState,
    EnteredComposingState,
    DispatchedAgent,
    ProgressUpdated,
    EnvironmentSelected,
    Failed { error_message: String },
    ShowAICreditModal,
    NeedsGithubAuth,
    Cancelled,
    HarnessSelected,
    HarnessCommandStarted,
    UpdatedSetupCommandVisibility,
}

impl Entity for LocalAgentViewModel {
    type Event = LocalAgentViewModelEvent;
}

pub fn is_cloud_agent_pre_first_exchange(
    _local_agent_view_model: &ModelHandle<LocalAgentViewModel>,
    _agent_view_controller: &ModelHandle<crate::ai::blocklist::agent_view::AgentViewController>,
    _app: &AppContext,
) -> bool {
    false
}

pub struct AmbientAgentEntryBlock;

impl View for AmbientAgentEntryBlock {
    fn ui_name() -> &'static str {
        "AmbientAgentEntryBlock"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

pub enum AmbientAgentEntryBlockEvent {}

impl Entity for AmbientAgentEntryBlock {
    type Event = AmbientAgentEntryBlockEvent;
}

#[derive(Debug)]
pub enum AmbientAgentEntryBlockAction {}

impl TypedActionView for AmbientAgentEntryBlock {
    type Action = AmbientAgentEntryBlockAction;
}

#[derive(Clone, Debug, PartialEq)]
pub enum HarnessSelectorAction {
    ToggleMenu,
}

pub enum HarnessSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct HarnessSelector;

impl HarnessSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _local_agent_model: ModelHandle<LocalAgentViewModel>,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }

    pub fn set_button_theme<T>(&mut self, _theme: T, _ctx: &mut ViewContext<Self>) {}
}

impl View for HarnessSelector {
    fn ui_name() -> &'static str {
        "HarnessSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for HarnessSelector {
    type Event = HarnessSelectorEvent;
}

impl TypedActionView for HarnessSelector {
    type Action = HarnessSelectorAction;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostSelectorAction {
    ToggleMenu,
}

pub enum HostSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct HostSelector;

impl HostSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }
}

impl View for HostSelector {
    fn ui_name() -> &'static str {
        "HostSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for HostSelector {
    type Event = HostSelectorEvent;
}

impl TypedActionView for HostSelector {
    type Action = HostSelectorAction;
}

pub struct NakedHeaderButtonTheme;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSelectorAction {
    ToggleMenu,
}

pub enum ModelSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct ModelSelector;

impl ModelSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _terminal_view_id: EntityId,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }

    pub fn is_menu_open(&self) -> bool {
        false
    }
}

impl View for ModelSelector {
    fn ui_name() -> &'static str {
        "ModelSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for ModelSelector {
    type Event = ModelSelectorEvent;
}

impl TypedActionView for ModelSelector {
    type Action = ModelSelectorAction;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetupCommandState {
    did_execute_a_setup_command: bool,
    should_expand_setup_commands: bool,
}

impl SetupCommandState {
    pub fn did_execute_a_setup_command(&self) -> bool {
        self.did_execute_a_setup_command
    }

    pub fn set_did_execute_a_setup_command(&mut self, value: bool) {
        self.did_execute_a_setup_command = value;
    }

    pub fn should_expand(&self) -> bool {
        self.should_expand_setup_commands
    }

    pub fn set_should_expand(&mut self, value: bool) {
        self.should_expand_setup_commands = value;
    }
}

pub fn render_loading_footer(
    _appearance: &warp_core::ui::appearance::Appearance,
) -> Box<dyn Element> {
    Empty::new().finish()
}

pub fn render_error_footer(
    _error_message: &str,
    _appearance: &warp_core::ui::appearance::Appearance,
) -> Box<dyn Element> {
    Empty::new().finish()
}

pub struct ProgressProps;
pub struct ProgressStep;
pub struct ProgressStepState;

pub fn render_progress(_props: ProgressProps, _app: &AppContext) -> Box<dyn Element> {
    Empty::new().finish()
}
