use std::collections::HashSet;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use session_sharing_protocol::common::ParticipantId;
use session_sharing_protocol::common::Role;
use session_sharing_protocol::common::SessionId as SharedSessionId;
use session_sharing_protocol::sharer::SessionEndedReason;
use strum_macros::EnumDiscriminants;
use strum_macros::EnumIter;
use warp_completer::completer::MatchType;
use warp_core::command::ExitCode;
use warp_core::telemetry::EnablementState;
use warp_core::telemetry::TelemetryEvent as TelemetryEventTrait;
use warp_core::telemetry::TelemetryEventDesc;
use warpui::keymap::Keystroke;
use warpui::notification::{NotificationSendError, RequestPermissionsOutcome};
use warpui::rendering::ThinStrokes;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum SharedSessionActionSource {
    BlocklistContextMenu,
    Tab,
    PaneHeader,
    CommandPalette,
    OnboardingBlock,
    Closed,
    InactivityModal,
    NonUser,
    SharingDialog,
    RightClickMenu,
    FooterChip,
}

use crate::ai::agent::api::ServerConversationToken;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::agent::conversation::AmbientAgentTaskId;
use crate::ai::agent::AIAgentActionId;
use crate::ai::agent::AIAgentExchangeId;
use crate::ai::agent::AIAgentInput as FullAIAgentInput;
use crate::ai::agent::AIIdentifiers;
use crate::ai::agent::EntrypointType;
use crate::ai::agent::PassiveSuggestionTrigger;
use crate::ai::agent::ServerOutputId;
use crate::ai::agent::SuggestedLoggingId;
use crate::ai::blocklist::agent_view::AgentViewEntryOrigin;
use crate::ai::blocklist::AIBlockResponseRating;
use crate::ai::blocklist::CommandExecutionPermissionAllowedReason;
use crate::ai::blocklist::InputType;
use crate::ai::mcp::TemplateVariable;
type LoginGatedFeature = &'static str;
use crate::channel::Channel;
#[cfg(feature = "local_fs")]
use crate::code::editor_management::CodeSource;
use crate::features::FeatureFlag;
use crate::launch_configs::save_modal::SaveState;
use crate::notebooks::telemetry::NotebookTelemetryAction;
use crate::notebooks::NotebookId;
use crate::notebooks::NotebookLocation;
use crate::palette::PaletteMode;
use crate::pane_group::PaneDragDropLocation;
use crate::prompt::editor_modal::OpenSource as PromptEditorOpenSource;
use crate::search::command_search::searcher::CommandSearchItemAction;
use crate::search::QueryFilter;
use crate::server::block::DisplaySetting;
use crate::server::ids::ObjectUid;
use crate::server::ids::ServerId;
use crate::settings::import::config::ParsedTerminalSetting;
use crate::settings::import::config::SettingType;
use crate::settings::import::model::TerminalType;
use crate::settings::AgentModeCodingPermissionsType;
use crate::tab::TabTelemetryAction;
use crate::terminal::block_list_viewport::InputMode;
use crate::terminal::cli_agent_sessions::CLIAgentInputEntrypoint;
use crate::terminal::cli_agent_sessions::CLIAgentRichInputCloseReason;
use crate::terminal::input::TelemetryInputSuggestionsMode;
use crate::terminal::model::ansi::WarpificationUnavailableReason;
use crate::terminal::model::block::BlockId;
use crate::terminal::model::session::SessionId;
use crate::terminal::model::terminal_model::BlockSelectionCardinality;
use crate::terminal::model::terminal_model::TmuxInstallationState;
use crate::terminal::settings::AltScreenPaddingMode;
use crate::terminal::shell::ShellType;
use crate::terminal::ssh::ssh_detection::SshInteractiveSessionDetected;
use crate::terminal::view::block_onboarding::onboarding_agentic_suggestions_block::OnboardingChipType;
use crate::terminal::view::inline_banner::ZeroStatePromptSuggestionTriggeredFrom;
use crate::terminal::view::inline_banner::ZeroStatePromptSuggestionType;
use crate::terminal::view::BlockEntity;
use crate::terminal::view::BlockSelectionDetails;
use crate::terminal::view::ContextMenuInfo;
use crate::terminal::view::GridHighlightedLink;
use crate::terminal::view::PromptPart;
use crate::terminal::view::{
    NotificationsDiscoveryBannerAction, NotificationsErrorBannerAction, NotificationsTrigger,
};
use crate::tips::WelcomeTipFeature;
#[cfg(feature = "local_fs")]
use crate::util::file::external_editor::settings::EditorLayout;
#[cfg(feature = "local_fs")]
use crate::util::openable_file_type::FileTarget;
use crate::workflows::WorkflowId;
use crate::workflows::WorkflowSelectionSource;
use crate::workflows::WorkflowSource;
use crate::workspace::tab_settings::TabCloseButtonPosition;
use crate::workspace::tab_settings::WorkspaceDecorationVisibility;
use crate::workspace::TabMovement;
use session_sharing_protocol::sharer::SessionSourceType;
use warp_core::interval_timer::TimingDataPoint;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum GenericStringObjectFormat {
    Removed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ObjectType {
    Notebook,
    Workflow,
    Folder,
    GenericStringObject(GenericStringObjectFormat),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Space {
    Personal,
    Team { team_uid: ServerId },
    Shared,
}

pub type GenericStringObjectId = ServerId;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloudObjectTypeAndId {
    Notebook(ServerId),
    Workflow(ServerId),
    Folder(ServerId),
    GenericStringObject {
        object_type: GenericStringObjectFormat,
        id: ServerId,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum DriveSortOrder {
    Removed,
}

pub enum NotificationSourceAgent {
    Oz,
    CLI(CLIAgentInputEntrypoint),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BootstrappingInfo {
    pub shell: &'static str,
    pub is_ssh: bool,
    pub is_subshell: bool,
    pub is_wsl: bool,
    pub is_msys2: bool,
    /// `true` if the bootstrapping process was triggered by an RC file snippet.
    ///
    /// This should only be true if `is_subshell` is true.
    pub was_triggered_by_rc_file: bool,
    /// The total time it took to bootstrap the shell, in seconds.
    pub bootstrap_duration_seconds: Option<f64>,
    /// The time it took to source the user's rcfiles, in seconds.  May be None
    /// if we weren't able to get that information from the shell.
    pub rcfiles_duration_seconds: Option<f64>,
    /// The difference between the total bootstrap time and the rcfile sourcing
    /// time, which roughly equals the time cost of running our bootstrap
    /// script.  Will be None if `bootstrap_duration_seconds` or
    /// `rcfiles_duration_seconds` is None.
    pub warp_attributed_bootstrap_duration_seconds: Option<f64>,
    pub shell_version: Option<String>,
    pub terminal_session_id: Option<SessionId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SlowBootstrapInfo {
    pub shell: &'static str,
    pub is_ssh: bool,
    pub is_subshell: bool,
    pub is_wsl: bool,
    pub is_msys2: bool,
    /// Contents of the bootstrap block when the slow bootstrap was detected.
    /// This includes both command and output content from the block.
    pub bootstrap_block_contents: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppStartupInfo {
    pub is_session_restoration_on: bool,
    /// Whether or not a screen reader is enabled at the time the app is
    /// launched.  Should be set to None if we do not know for sure.
    pub is_screen_reader_enabled: Option<bool>,
    pub from_relaunch: bool,
    pub timing_data: Vec<TimingDataPoint>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum DownloadSource {
    Website,
    Homebrew,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockLatencyInfo {
    pub command: &'static str,
    pub shell: &'static str,
    pub is_ssh: bool,
    pub execution_ms: u64,
}

// For use when recording what type of cloud object a particular telemetry is for.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TelemetryCloudObjectType {
    Workflow,
    Notebook,
    Folder,
    GenericStringObject(GenericStringObjectFormat),
}

impl From<&CloudObjectTypeAndId> for TelemetryCloudObjectType {
    fn from(cloud_object_type_and_id: &CloudObjectTypeAndId) -> Self {
        match cloud_object_type_and_id {
            CloudObjectTypeAndId::Notebook(_) => Self::Notebook,
            CloudObjectTypeAndId::Workflow(_) => Self::Workflow,
            CloudObjectTypeAndId::Folder(_) => Self::Folder,
            CloudObjectTypeAndId::GenericStringObject { object_type, .. } => {
                Self::GenericStringObject(*object_type)
            }
        }
    }
}

/// For use when recording how a user has access to a cloud object.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TelemetrySpace {
    /// The object is owned by the current user.
    Personal,
    /// The object is owned by a team the user is on.
    Team,
    /// The object was shared with the user.
    Shared,
}

impl From<Space> for TelemetrySpace {
    fn from(space: Space) -> Self {
        match space {
            Space::Personal => Self::Personal,
            Space::Team { .. } => Self::Team,
            Space::Shared => Self::Shared,
        }
    }
}

/// Common metadata to include in all Warp Drive telemetry events that act on a specific object.
/// Events that only apply to a single object type may use specific metadata like [`WorkflowTelemetryMetadata`],
/// [`NotebookTelemetryMetadata`], or [`EnvVarTelemetryMetadata`] instead.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudObjectTelemetryMetadata {
    pub object_type: TelemetryCloudObjectType,
    /// The server UID of the object. This only exists for objects that have been synced to the
    /// server.
    pub object_uid: Option<ServerId>,
    /// The space through which the user has access to the object.
    pub space: Option<TelemetrySpace>,
    /// If the object is owned by a team, this is the owning team's UID. For shared objects, the
    /// user might not be on the team.
    pub team_uid: Option<ServerId>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowTelemetryMetadata {
    pub workflow_categories: Option<Vec<String>>,
    pub workflow_source: WorkflowSource,
    pub workflow_space: Option<TelemetrySpace>,
    pub workflow_selection_source: WorkflowSelectionSource,
    // This field is only populated for cloud workflows that have been synced to the server
    pub workflow_id: Option<WorkflowId>,
    // Any referenced workflow enums that have been synced to the cloud
    pub enum_ids: Vec<GenericStringObjectId>,
}

/// Metadata to include in all notebook telemetry events.
///
/// There are 4 expected configurations:
/// * Personal cloud notebooks: `notebook_id` is `Some`, `team_uid` is `None`, and location is `PersonalCloud`
/// * Team cloud notebooks: `notebook_id` is `Some`, `team_uid` is `Some`, and location is `Team`
/// * Local file-based notebooks: `notebook_id` and `team_uid` are `None`, and location is `LocalFile`
/// * Remote file-based notebooks: `notebook_id` and `team_uid` are `None`, and location is `RemoteFile`
///
/// This representation allows for invalid combinations, but makes querying the data easier (for
/// example, to find all notebook events for a given team).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct NotebookTelemetryMetadata {
    /// The notebook ID, only available for cloud notebooks that have been synced to the server.
    pub notebook_id: Option<NotebookId>,
    /// The team UID, only available for cloud notebooks in a shared team.
    pub team_uid: Option<ServerId>,
    pub space: Option<TelemetrySpace>,
    /// Where the notebook is canonically located.
    pub location: NotebookLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_table_count: Option<usize>,
}

impl NotebookTelemetryMetadata {
    pub fn new(
        notebook_id: impl Into<Option<NotebookId>>,
        team_uid: impl Into<Option<ServerId>>,
        location: impl Into<NotebookLocation>,
        space: Option<TelemetrySpace>,
    ) -> Self {
        Self {
            notebook_id: notebook_id.into(),
            team_uid: team_uid.into(),
            location: location.into(),
            space,
            markdown_table_count: None,
        }
    }

    pub fn with_markdown_table_count(mut self, markdown_table_count: usize) -> Self {
        self.markdown_table_count = Some(markdown_table_count);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotebookActionEvent {
    #[serde(flatten)]
    pub action: NotebookTelemetryAction,
    #[serde(flatten)]
    pub metadata: NotebookTelemetryMetadata,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnvVarTelemetryMetadata {
    /// The object ID, only available for cloud env vars that have been synced to the server.
    pub object_id: Option<GenericStringObjectId>,
    /// The team UID, only available for cloud env vars in a shared team.
    pub team_uid: Option<ServerId>,
    pub space: TelemetrySpace,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MCPServerTelemetryMetadata {
    pub object_id: GenericStringObjectId,
    pub name: String,
    pub transport_type: MCPServerTelemetryTransportType,
    /// The MCP server string extracted from '@modelcontextprotocol/<...>'.
    pub mcp_server: Option<String>,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPTemplateCreationSource {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "conversion")]
    Conversion,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPTemplateInstallationSource {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "shared")]
    Shared,
    #[serde(rename = "gallery")]
    Gallery,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPServerModel {
    #[serde(rename = "legacy")]
    Legacy,
    #[serde(rename = "templatable")]
    Templatable,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MCPServerTelemetryTransportType {
    CLIServer,
    ServerSentEvents,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum MCPServerTelemetryError {
    Initialization(String),
    RequestCancelled,
    ResponseError(String),
    SerializationError(String),
    CapabilityUnsupported(String),
    InternalError(String),
    TransportError(String),
}

#[cfg(not(target_family = "wasm"))]
impl From<rmcp::RmcpError> for MCPServerTelemetryError {
    fn from(err: rmcp::RmcpError) -> Self {
        match err {
            rmcp::RmcpError::ClientInitialize(err) => Self::Initialization(err.to_string()),
            rmcp::RmcpError::ServerInitialize(err) => Self::Initialization(err.to_string()),
            rmcp::RmcpError::TransportCreation { error, .. } => {
                Self::TransportError(error.to_string())
            }
            rmcp::RmcpError::Runtime(err) => Self::InternalError(err.to_string()),
            rmcp::RmcpError::Service(err) => match err {
                rmcp::ServiceError::McpError(_) => Self::ResponseError(err.to_string()),
                rmcp::ServiceError::TransportSend(_) => Self::TransportError(err.to_string()),
                rmcp::ServiceError::TransportClosed => Self::TransportError(err.to_string()),
                rmcp::ServiceError::UnexpectedResponse => Self::ResponseError(err.to_string()),
                rmcp::ServiceError::Cancelled { .. } => Self::InternalError(err.to_string()),
                rmcp::ServiceError::Timeout { .. } => Self::TransportError(err.to_string()),
                // The enum is marked as non-exhaustive, so we need a catch-all.
                _ => Self::InternalError(err.to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenedSharingDialogEvent {
    pub source: SharingDialogSource,

    /// Metadata for the object being shared, if it's a Warp Drive object.
    #[serde(flatten)]
    pub object_metadata: Option<CloudObjectTelemetryMetadata>,

    /// Metadata for the session being shared, if there is one.
    pub session_id: Option<SharedSessionId>,
}

/// How the user opened the Warp Drive sharing dialog.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SharingDialogSource {
    /// The sharing button in the pane header.
    PaneHeader,
    /// The per-pane command palette entry (includes keybindings).
    CommandPalette,
    /// The Warp Drive index context menu.
    DriveIndex,
    /// The sharing dialog was auto-opened from shared session creation.
    StartedSessionShare,
    /// The user intented into Warp with an email address to invite.
    InviteeRequest,
    /// The user jumped from an inherited ACL to its definition on a parent object.
    InheritedPermission,
    /// The onboarding block shown after users create new personal objects.
    OnboardingBlock,
    /// The conversation list overflow menu.
    ConversationList,
    /// The AI block context menu.
    AIBlockContextMenu,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum TabRenameEvent {
    OpenedEditor,
    CustomNameSet,
    CustomNameCleared,
}

/// The possible sources notifications can turned on from.
#[derive(Clone, Serialize, Deserialize)]
pub enum NotificationsTurnedOnSource {
    Settings,
    Banner,
}

/// The possible types of toggles in the find bar
#[derive(Clone, Serialize, Deserialize)]
pub enum FindOption {
    CaseSensitive,
    FindInBlock,
    Regex,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum LinkOpenMethod {
    CmdClick,
    ToolTip,
    MiddleClick,
}

/// The possible ways to trigger command x-ray
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandXRayTrigger {
    Hover,
    Keystroke,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum PaletteSource {
    PrefixChange,
    Keybinding,
    CtrlTab { shift_pressed_initially: bool },
    WarpDrive,
    QuitModal,
    LogOutModal,
    IntegrationTest,
    ConversationManager,
    ContextChip,
    PaneHeader,
    RecentsViewAll,
    AgentTip,
    TitleBarSearchBar,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FileTreeSource {
    /// Opened from the pane header toolbelt button.
    PaneHeader,
    Keybinding,
    LeftPanelToolbelt,
    ForceOpened,
    /// Opened from the CLI agent view footer (e.g., Claude Code).
    CLIAgentView,
}

#[cfg(feature = "local_fs")]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodePanelsFileOpenEntrypoint {
    CodeReview,
    ProjectExplorer,
    GlobalSearch,
}

/// The CLI agent being used (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CLIAgentType {
    Claude,
    Gemini,
    Codex,
    Amp,
    Droid,
    OpenCode,
    Copilot,
    Pi,
    Auggie,
    Cursor,
    Unknown,
}

/// The kind of plugin chip shown or dismissed (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginChipTelemetryKind {
    Install,
    Update,
}

/// Identifies the agent variant that triggered a notification (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationAgentVariant {
    /// Warp's built-in agent (Oz).
    Oz,
    /// A CLI agent (e.g., Claude Code, Gemini CLI, etc.).
    CLIAgent(CLIAgentType),
}

impl From<NotificationSourceAgent> for NotificationAgentVariant {
    fn from(agent: NotificationSourceAgent) -> Self {
        match agent {
            NotificationSourceAgent::Oz => Self::Oz,
            NotificationSourceAgent::CLI(_cli_agent) => Self::CLIAgent(CLIAgentType::Unknown),
        }
    }
}

/// The action taken on a plugin chip (for telemetry purposes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginChipTelemetryAction {
    /// User clicked the auto-install button.
    Install,
    /// User clicked the auto-update button.
    Update,
    /// User clicked the manual install instructions button.
    InstallInstructions,
    /// User clicked the manual update instructions button.
    UpdateInstructions,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum WarpDriveSource {
    Legacy,
    LeftPanelToolbelt,
    ForceOpened,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CommandCorrectionAcceptedType {
    /// TODO: We don't use the Autosuggestion variant yet. We need to wire through
    /// when an autosuggestion is accepted to be able to check this.
    Autosuggestion,
    Banner,
    Keybinding,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CommandCorrectionEvent {
    Proposed {
        rule: &'static str,
    },
    Accepted {
        via: CommandCorrectionAcceptedType,
        rule: &'static str,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CommandSearchResultType {
    History,
    Workflow,
    ViewInWarpDrive,
    AIQuery,
    Project,
}

impl From<&CommandSearchItemAction> for CommandSearchResultType {
    fn from(action: &CommandSearchItemAction) -> Self {
        use crate::search::command_search::searcher::CommandSearchItemAction::*;
        match action {
            AcceptHistory(_) | ExecuteHistory(_) => Self::History,
            AcceptWorkflow(_) => Self::Workflow,
            AcceptAIQuery(_) | RunAIQuery(_) => Self::AIQuery,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloseTarget {
    App,
    Window,
    Tab,
    Pane,
    EditorTab,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum PtySpawnMode {
    /// The pty was spawned using the terminal server.
    TerminalServer,
    /// We tried to spawn the pty using the terminal server, but something went
    /// wrong so we fell back to spawning it directly.
    FallbackToDirect,
    /// The terminal server is not in use, and we spawned the pty directly
    /// (in tests, for example).
    Direct,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SaveAsWorkflowModalSource {
    Block,
    Input,
    AIWorkflowCard,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LaunchConfigUiLocation {
    CommandPalette,
    AppMenu,
    TabMenu,
    Uri,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AICommandSearchEntrypoint {
    ShortHandTrigger,
    Keybinding,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SecretInteraction {
    RevealSecret,
    HideSecret,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnonymousUserSignupEntrypoint {
    HitDriveObjectLimit,
    LoginGatedFeature,
    SignUpButton,
    RenotificationBlock,
    SignUpAIPrompt,
    NextCommandSuggestionsUpgradeBanner,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum UndoCloseItemType {
    Window,
    Tab,
    Pane,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PromptChoice {
    PS1,
    Default,
    Custom { builtin_chips: Vec<String> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ToggleBlockFilterSource {
    /// This includes the keybinding and the command palette items.
    Binding,
    ContextMenu,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TierLimitHitEvent {
    pub team_uid: ServerId,
    pub feature: String,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum KnowledgePaneEntrypoint {
    /// Triggered by either the command palette or the mac menus
    #[serde(rename = "global")]
    Global,

    #[serde(rename = "settings")]
    Settings,

    #[serde(rename = "warp_drive")]
    WarpDrive,

    #[serde(rename = "ai_blocklist")]
    AIBlocklist,

    #[serde(rename = "slash_command")]
    SlashCommand,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum MCPServerCollectionPaneEntrypoint {
    /// Triggered by either the command palette or the mac menus
    #[serde(rename = "global")]
    Global,

    #[serde(rename = "settings")]
    Settings,

    #[serde(rename = "warp_drive")]
    WarpDrive,

    #[serde(rename = "slash_command")]
    SlashCommand,

    #[serde(rename = "mcp_settings_tab")]
    MCPSettingsTab,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypointSelectionType {
    /// User entered Agent Mode by taking action on a blocklist text selection.
    Text,

    /// User entered Agent Mode by taking action on a block selection.
    Block,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeEntrypoint {
    /// The stars icon button in the tab bar.
    #[serde(rename = "tab_bar")]
    TabBar,

    /// This corresponds to _both_ triggering from the command palette and via keybinding.
    ///
    /// Unfortunately due to the way the command palette automatically surfaces any editable
    /// keybinding as an action, we don't have enough information to discern if the binding was
    /// triggered by the palette or keyboard.
    #[serde(rename = "new_pane_binding")]
    NewPaneBinding,

    /// The stars button in the hoverable block "toolbelt".
    #[serde(rename = "block_toolbelt")]
    BlockToolbelt,

    /// The "Ask Agent Mode" option from AI command search.
    #[serde(rename = "ai_command_search")]
    AICommandSearch,

    /// Context menu item(s) that attach a blocklist selection as context to an Agent Mode query.
    #[serde(rename = "context_menu")]
    ContextMenu {
        selection_type: AgentModeEntrypointSelectionType,
    },

    /// The Agent Mode chip in the prompt.
    #[serde(rename = "prompt_chip")]
    PromptChip,

    /// The Agent Management popup, where you can see all the most recent tasks for each terminal
    /// pane across all windows/tabs/panes.
    #[serde(rename = "agent_management_popup")]
    AgentManagementPopup,

    /// User manually switched between terminal and AI input modes in UDI interface
    #[serde(rename = "udi_terminal_input_switcher")]
    UDITerminalInputSwitcher,

    /// The agent management view, where you can see both local interactive and ambient agent tasks
    #[serde(rename = "agent_management_view")]
    AgentManagementView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutonomySettingToggleSource {
    Speedbump,
    SettingsPage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ToggleCodeSuggestionsSettingSource {
    Speedbump,
    Settings,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum InteractionSource {
    Button,
    Keybinding,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionViewType {
    TerminalView,
    AgentView,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeAttachContextMethod {
    #[serde(rename = "keyboard")]
    Keyboard,

    #[serde(rename = "mouse")]
    Mouse,
}

/// The entrypoint from which the rewind dialog was opened.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AgentModeRewindEntrypoint {
    /// The rewind button in the AI block header.
    Button,
    /// The context menu item "Rewind to before here".
    ContextMenu,
    /// The /rewind slash command.
    SlashCommand,
}

/// Reasons why we fell back to a prompt suggestion from a suggested code diff.
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum PromptSuggestionFallbackReason {
    /// Code file had too many lines, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_lines")]
    FileTooManyLines,
    /// Code file had too many bytes, hence we stopped triggering the suggested code diff.
    #[serde(rename = "file_too_many_bytes")]
    FileTooManyBytes,
    /// Missing file, when looking up filepaths in local file system.
    #[serde(rename = "missing_file")]
    MissingFile,
    /// Failed to retrieve file from local file system.
    #[serde(rename = "failed_to_retrieve_file")]
    FailedToRetrieveFile,
    /// In an SSH/remote session.
    #[serde(rename = "ssh_remote_session")]
    SSHRemoteSession,
    /// No read files permission.
    #[serde(rename = "no_read_files_permission")]
    NoReadFilesPermission,
    /// AI query timeout.
    #[serde(rename = "ai_query_timeout")]
    AIQueryTimeout,
    /// Failed to send AI request.
    #[serde(rename = "failed_to_send_ai_request")]
    FailedToSendAIRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeSetupProjectScopedRulesActionType {
    #[serde(rename = "link_from_existing")]
    LinkFromExisting(String),
    #[serde(rename = "generate_warp_md")]
    GenerateWarpMd,
    #[serde(rename = "skip_rules")]
    SkipRules,
    #[serde(rename = "regenerate_warp_md")]
    RegenerateWarpMd,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentModeSetupCodebaseContextActionType {
    #[serde(rename = "index_codebase")]
    IndexCodebase,
    #[serde(rename = "skip_indexing")]
    SkipIndexing,
    #[serde(rename = "view_index_status")]
    ViewIndexStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuUsageStats {
    /// The number of logical CPUs on the system.
    pub num_cpus: usize,

    /// The maximum CPU usage over the measurement interval.
    ///
    /// This number is in the range [0, num_cpus].  The CPU utilization, as a
    /// percentage, can be determined via `max_usage / num_cpus * 100`.
    pub max_usage: f32,

    /// The average CPU usage over the measurement interval.
    ///
    /// This number is in the range [0, num_cpus].  The CPU utilization, as a
    /// percentage, can be determined via `avg_usage / num_cpus * 100`.
    pub avg_usage: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryUsageStats {
    pub total_application_usage_bytes: usize,
    pub total_blocks: usize,
    pub total_lines: usize,

    /// Statistics about blocks that have been seen in the past 5 minutes.
    pub active_block_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [5m, 1h).
    pub inactive_5m_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [1h, 24h).
    pub inactive_1h_stats: BlockMemoryUsageStats,
    /// Statistics about blocks that haven't been seen since [24h, ..).
    pub inactive_24h_stats: BlockMemoryUsageStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMemoryUsageStats {
    pub num_blocks: usize,
    pub num_lines: usize,
    pub estimated_memory_usage_bytes: usize,
}

/// Entrypoints to toggle the input auto-detection setting for Agent Mode.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AgentModeAutoDetectionSettingOrigin {
    /// The "speed bump" banner shown that's shown to the user when input is autodetected.
    #[serde(rename = "banner")]
    Banner,

    /// The AI settings page.
    #[serde(rename = "settings_page")]
    SettingsPage,
}

/// Payload for the [`AgentModePotentialAutodetectionFalsePositive`] event.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentModeAutoDetectionFalsePositivePayload {
    /// Payload includes input text for dogfood channels.
    InternalDogfoodUsers { input_text: String },

    /// Do not include the misclassified input text in stable channels due to privacy concerns.
    ExternalUsers,
}

/// How the user triggered the [`AgentModeCodeFilesNavigated`] event.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum AgentModeCodeFileNavigationSource {
    /// User used the next/previous actions.
    NavigationCommand,
    /// User directly selected the file's tab.
    SelectedFileTab,
}

/// How the user triggered the [`AddTabWithShell`] event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum AddTabWithShellSource {
    CommandPalette,
    ShellSelectorMenu,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeContextDestination {
    Pty,
    AgentInput,
    RichInput,
}

#[derive(Clone, Debug, Serialize)]
pub enum AgentModeCitation {
    WarpDriveObject {
        object_type: ObjectType,
        uid: ObjectUid,
    },
    WarpDocs {
        page: String,
    },
    WebPage {
        // Don't serialize the URL to avoid leaking sensitive information.
        #[serde(skip_serializing)]
        url: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ImageProtocol {
    Kitty,
    ITerm,
}

#[derive(Clone, Copy, Debug, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InputUXChangeOrigin {
    #[default]
    Settings,
    ADELaunchModal,
}

#[derive(Clone, Debug, Serialize)]
pub enum AIAgentInput {
    UserQuery { query: String },
    AutoCodeDiffQuery { query: String },
    ResumeConversation,
    InitProjectRules { display_query: Option<String> },
    TriggerSuggestPrompt { trigger: PassiveSuggestionTrigger },
    ActionResult { action_id: AIAgentActionId },
    CreateNewProject { query: String },
    CloneRepository { url: String },
    CodeReview,
    FetchReviewComments,
    SummarizeConversation,
    InvokeSkill { skill_name: String },
    StartFromAmbientRunPrompt,
    MessagesReceivedFromAgents { message_count: usize },
    EventsFromAgents { event_count: usize },
    PassiveSuggestionResult,
}

impl From<FullAIAgentInput> for AIAgentInput {
    fn from(input: FullAIAgentInput) -> Self {
        match input {
            FullAIAgentInput::UserQuery { query, .. } => Self::UserQuery { query },
            FullAIAgentInput::AutoCodeDiffQuery { query, .. } => Self::AutoCodeDiffQuery { query },
            FullAIAgentInput::ResumeConversation { .. } => Self::ResumeConversation,
            FullAIAgentInput::InitProjectRules { display_query, .. } => {
                Self::InitProjectRules { display_query }
            }
            FullAIAgentInput::TriggerPassiveSuggestion { trigger, .. } => {
                Self::TriggerSuggestPrompt { trigger }
            }
            FullAIAgentInput::ActionResult { result, .. } => Self::ActionResult {
                action_id: result.id,
            },
            FullAIAgentInput::CreateNewProject { query, .. } => Self::CreateNewProject { query },
            FullAIAgentInput::CloneRepository { clone_repo_url, .. } => Self::CloneRepository {
                url: clone_repo_url.into_url(),
            },
            FullAIAgentInput::CodeReview { .. } => Self::CodeReview,
            FullAIAgentInput::FetchReviewComments { .. } => Self::FetchReviewComments,
            FullAIAgentInput::SummarizeConversation { .. } => Self::SummarizeConversation,
            FullAIAgentInput::InvokeSkill { skill, .. } => Self::InvokeSkill {
                skill_name: skill.name.clone(),
            },
            FullAIAgentInput::StartFromAmbientRunPrompt { .. } => Self::StartFromAmbientRunPrompt,
            FullAIAgentInput::MessagesReceivedFromAgents { messages } => {
                Self::MessagesReceivedFromAgents {
                    message_count: messages.len(),
                }
            }
            FullAIAgentInput::EventsFromAgents { events } => Self::EventsFromAgents {
                event_count: events.len(),
            },
            FullAIAgentInput::PassiveSuggestionResult { .. } => Self::PassiveSuggestionResult,
        }
    }
}

/// The origin of an agent view entry, for telemetry purposes.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryAgentViewEntryOrigin {
    Input { was_prompt_autodetected: bool },
    ConversationSelector,
    AgentModeHomepage,
    AgentViewBlock,
    AIDocument,
    AutoFollowUp,
    RestoreExistingConversation,
    SharedSessionSelection,
    AgentRequestedNewConversation,
    AcceptedPromptSuggestion,
    AcceptedUnitTestSuggestion,
    AcceptedPassiveCodeDiff,
    InlineCodeReview,
    AmbientAgent,
    Cli,
    ImageAdded,
    SlashCommand,
    CodeReviewContext,
    ContinueConversationButton,
    ViewPassiveCodeDiffDetails,
    ResumeConversationButton,
    CodexModal,
    LongRunningCommand,
    HistoryMenu,
    InlineConversationMenu,
    PromptChip,
    OnboardingCallout,
    ConversationListView,
    Onboarding,
    Keybinding,
    SlashInit,
    ProjectEntry,
    ClearBuffer,
    DefaultSessionMode,
    ChildAgent,
    LinearDeepLink,
    ThirdPartyCloudAgent,
}

impl From<AgentViewEntryOrigin> for TelemetryAgentViewEntryOrigin {
    fn from(origin: AgentViewEntryOrigin) -> Self {
        match origin {
            AgentViewEntryOrigin::Input {
                was_prompt_autodetected,
            } => Self::Input {
                was_prompt_autodetected,
            },
            AgentViewEntryOrigin::ConversationSelector => Self::ConversationSelector,
            AgentViewEntryOrigin::AgentModeHomepage => Self::AgentModeHomepage,
            AgentViewEntryOrigin::AgentViewBlock => Self::AgentViewBlock,
            AgentViewEntryOrigin::AIDocument => Self::AIDocument,
            AgentViewEntryOrigin::AutoFollowUp => Self::AutoFollowUp,
            AgentViewEntryOrigin::RestoreExistingConversation => Self::RestoreExistingConversation,
            AgentViewEntryOrigin::AgentRequestedNewConversation => {
                Self::AgentRequestedNewConversation
            }
            AgentViewEntryOrigin::AcceptedPromptSuggestion => Self::AcceptedPromptSuggestion,
            AgentViewEntryOrigin::AcceptedUnitTestSuggestion => Self::AcceptedUnitTestSuggestion,
            AgentViewEntryOrigin::AcceptedPassiveCodeDiff => Self::AcceptedPassiveCodeDiff,
            AgentViewEntryOrigin::InlineCodeReview => Self::InlineCodeReview,
            AgentViewEntryOrigin::Cli => Self::Cli,
            AgentViewEntryOrigin::ImageAdded => Self::ImageAdded,
            AgentViewEntryOrigin::SlashCommand { .. } => Self::SlashCommand,
            AgentViewEntryOrigin::CodeReviewContext => Self::CodeReviewContext,
            AgentViewEntryOrigin::LongRunningCommand => Self::LongRunningCommand,
            AgentViewEntryOrigin::ContinueConversationButton => Self::ContinueConversationButton,
            AgentViewEntryOrigin::ViewPassiveCodeDiffDetails => Self::ViewPassiveCodeDiffDetails,
            AgentViewEntryOrigin::ResumeConversationButton => Self::ResumeConversationButton,
            AgentViewEntryOrigin::CodexModal => Self::CodexModal,
            AgentViewEntryOrigin::InlineHistoryMenu => Self::HistoryMenu,
            AgentViewEntryOrigin::InlineConversationMenu => Self::InlineConversationMenu,
            AgentViewEntryOrigin::PromptChip => Self::PromptChip,
            AgentViewEntryOrigin::OnboardingCallout => Self::OnboardingCallout,
            AgentViewEntryOrigin::ConversationListView => Self::ConversationListView,
            AgentViewEntryOrigin::Onboarding => Self::Onboarding,
            AgentViewEntryOrigin::Keybinding => Self::Keybinding,
            AgentViewEntryOrigin::SlashInit => Self::SlashInit,
            AgentViewEntryOrigin::ProjectEntry => Self::ProjectEntry,
            AgentViewEntryOrigin::ClearBuffer => Self::ClearBuffer,
            AgentViewEntryOrigin::DefaultSessionMode => Self::DefaultSessionMode,
            AgentViewEntryOrigin::ChildAgent => Self::ChildAgent,
            AgentViewEntryOrigin::LinearDeepLink => Self::LinearDeepLink,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum SlashMenuSource {
    SlashButton,
    UserTyped,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoginEventSource {
    OnboardingSlide,
    AuthModal,
}

/// Details about which type of slash command was accepted
#[derive(Clone, Debug, Serialize)]
pub enum SlashCommandAcceptedDetails {
    /// A built-in static command with its specific name (e.g., "/init", "/diff-review")
    StaticCommand { command_name: String },
    /// A user-created saved prompt/workflow
    SavedPrompt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AutoReloadModalAction {
    #[serde(rename = "dismissed")]
    Dismissed,
    #[serde(rename = "enabled_auto_reload")]
    EnabledAutoReload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OutOfCreditsBannerAction {
    #[serde(rename = "dismissed")]
    Dismissed,
    #[serde(rename = "credits_purchased")]
    CreditsPurchased,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CLISubagentControlState {
    AgentInControl,
    UserInControl,
    AgentTaggedIn,
    AgentTaggedOut,
}
