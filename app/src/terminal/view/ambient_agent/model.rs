use instant::Instant;
use warp_cli::agent::Harness;
use warpui::{EntityId, ModelContext};

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::server::ids::SyncId;
use crate::server::server_api::ai::{AttachmentInput, SpawnAgentRequest};
use crate::terminal::view::ambient_agent::SetupCommandState;

#[derive(Debug, Clone)]
pub struct AgentProgress {
    pub spawned_at: Instant,
    pub claimed_at: Option<Instant>,
    pub harness_started_at: Option<Instant>,
    pub stopped_at: Option<Instant>,
}

#[derive(Debug, Clone)]
pub enum Status {
    NotAmbientAgent,
    Setup,
    Composing,
    WaitingForSession {
        progress: AgentProgress,
    },
    AgentRunning,
    Failed {
        progress: AgentProgress,
        error_message: String,
    },
    NeedsGithubAuth {
        progress: AgentProgress,
        error_message: String,
        auth_url: String,
    },
    Cancelled {
        progress: AgentProgress,
    },
}

pub struct AmbientAgentViewModel {
    status: Status,
    terminal_view_id: EntityId,
    has_parent_terminal: bool,
    environment_id: Option<SyncId>,
    conversation_id: Option<AIConversationId>,
    harness: Harness,
    setup_commands_state: SetupCommandState,
    has_inserted_cloud_mode_user_query_block: bool,
}

impl AmbientAgentViewModel {
    pub fn new(
        terminal_view_id: EntityId,
        has_parent_terminal: bool,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        Self {
            status: Status::NotAmbientAgent,
            terminal_view_id,
            has_parent_terminal,
            environment_id: None,
            conversation_id: None,
            harness: Harness::Unknown,
            setup_commands_state: SetupCommandState::default(),
            has_inserted_cloud_mode_user_query_block: false,
        }
    }

    pub fn request(&self) -> Option<&SpawnAgentRequest> {
        None
    }

    pub fn setup_command_state(&self) -> &SetupCommandState {
        &self.setup_commands_state
    }

    pub fn setup_command_state_mut(&mut self) -> &mut SetupCommandState {
        &mut self.setup_commands_state
    }

    pub(super) fn set_setup_command_visibility(
        &mut self,
        is_visible: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        if is_visible != self.setup_commands_state.should_expand() {
            self.setup_commands_state.set_should_expand(is_visible);
            ctx.emit(AmbientAgentViewModelEvent::UpdatedSetupCommandVisibility);
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
            ctx.emit(AmbientAgentViewModelEvent::HarnessSelected);
        }
    }

    pub(super) fn is_third_party_harness(&self) -> bool {
        false
    }

    pub(super) fn harness_command_started(&self) -> bool {
        false
    }

    pub(super) fn mark_harness_command_started(&mut self, _ctx: &mut ModelContext<Self>) {}

    pub fn set_environment_id(
        &mut self,
        environment_id: Option<SyncId>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.environment_id = environment_id;
        ctx.emit(AmbientAgentViewModelEvent::EnvironmentSelected);
    }

    pub fn is_ambient_agent(&self) -> bool {
        false
    }

    pub fn task_id(&self) -> Option<AmbientAgentTaskId> {
        None
    }

    pub fn has_inserted_cloud_mode_user_query_block(&self) -> bool {
        self.has_inserted_cloud_mode_user_query_block
    }

    pub fn set_has_inserted_cloud_mode_user_query_block(&mut self, has_inserted: bool) {
        self.has_inserted_cloud_mode_user_query_block = has_inserted;
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
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn enter_composing_from_setup(&mut self, ctx: &mut ModelContext<Self>) {
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn enter_viewing_existing_session(
        &mut self,
        _task_id: AmbientAgentTaskId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn reset_status(&mut self, ctx: &mut ModelContext<Self>) {
        self.status = Status::NotAmbientAgent;
        self.environment_id = None;
        self.conversation_id = None;
        self.has_inserted_cloud_mode_user_query_block = false;
        ctx.notify();
    }

    pub fn set_conversation_id(&mut self, id: Option<AIConversationId>) {
        self.conversation_id = id;
    }

    pub fn spawn_agent(
        &mut self,
        _prompt: String,
        _attachments: Vec<AttachmentInput>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn spawn_agent_with_request(
        &mut self,
        _request: SpawnAgentRequest,
        ctx: &mut ModelContext<Self>,
    ) {
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn cancel_task(&mut self, ctx: &mut ModelContext<Self>) {
        self.status = Status::NotAmbientAgent;
        ctx.notify();
    }

    pub fn terminal_view_id(&self) -> EntityId {
        self.terminal_view_id
    }
}

#[derive(Debug, Clone)]
pub enum AmbientAgentViewModelEvent {
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
