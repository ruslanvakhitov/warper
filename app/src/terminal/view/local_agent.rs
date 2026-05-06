use instant::Instant;
use warp_cli::agent::Harness;
use warpui::{Entity, EntityId, ModelContext};

use crate::ai::agent::conversation::AIConversationId;
use warp_server_client::ids::SyncId;

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
}

pub type AmbientAgentViewModel = LocalAgentViewModel;

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
        self.harness
    }

    pub fn set_harness(&mut self, harness: Harness, ctx: &mut ModelContext<Self>) {
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

    pub fn status(&self) -> Status {
        Status::NotLocalAgent
    }

    pub fn reset_status(&mut self, ctx: &mut ModelContext<Self>) {
        self.environment_id = None;
        self.conversation_id = None;
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
