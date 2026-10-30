mod action;
mod block_banner;
mod bookmarks;
pub mod init;
pub mod inline_banner;

mod link_detection;
mod open_in_warp;
mod pane_impl;
pub(crate) mod plugin_instructions_block;
pub mod rich_content;
mod shell_terminated_banner;
pub mod ssh_file_upload;
mod tab_metadata;
#[cfg(any(test, feature = "integration_tests"))]
mod testing;
mod tooltips;
pub mod use_agent_footer;

use warpui::clipboard_utils::get_image_filepaths_from_paths;

use std::ops::Deref as _;

use crate::terminal::cli_agent_toolbar::toolbar_item::AgentToolbarItemKind;

pub use crate::terminal::view::rich_content::{
    RichContent, RichContentInsertionPosition, RichContentMetadata,
};

use use_agent_footer::UseAgentToolbar;

use super::cli_agent;
use super::CLIAgent;

#[cfg(feature = "local_fs")]
use crate::util::file::external_editor::{settings::EditorLayout, EditorSettings};

pub use self::link_detection::GridHighlightedLink;
pub use self::link_detection::{RichContentLink, RichContentLinkTooltipInfo};

use crate::code_review::context::ReviewCommentPacket;
use crate::code_review::diff_state::GitDeltaPreference;
#[cfg(feature = "local_fs")]
use crate::code_review::git_status_update::{
    GitRepoStatusModel, GitStatusMetadata, GitStatusUpdateModel,
};
use crate::code_review::metadata::CodeReviewPaneEntrypoint;
use crate::projects::ProjectManagementModel;

use crate::settings::CodeSettings;

use crate::terminal::cli_agent_sessions::event::{
    parse_event, CLIAgentEvent, CLIAgentEventPayload, CLIAgentEventType,
    CLI_AGENT_NOTIFICATION_SENTINEL,
};
use crate::terminal::cli_agent_sessions::listener::{is_agent_supported, CLIAgentSessionListener};
use crate::terminal::cli_agent_sessions::plugin_manager::{plugin_manager_for, PluginModalKind};
use crate::terminal::cli_agent_sessions::{
    CLIAgentInputEntrypoint, CLIAgentInputState, CLIAgentSession, CLIAgentSessionContext,
    CLIAgentSessionStatus, CLIAgentSessionsModel, CLIAgentSessionsModelEvent,
};
pub use action::TerminalAction;

pub use block_banner::{WithinBlockBanner, BLOCK_BANNER_HEIGHT};
pub use init::{init, CANCEL_COMMAND_KEYBINDING};
use init::{INPUT_BOX_VISIBLE_KEY, TOGGLE_BLOCK_FILTER_KEYBINDING};
use inline_banner::{
    render_alias_expansion_banner, render_inline_notifications_discovery_banner,
    render_inline_notifications_error_banner, render_inline_ssh_wrapper_banner,
    render_open_in_warp_banner, render_shell_process_terminated_banner, render_vim_mode_banner,
    AliasExpansionBanner, AliasExpansionBannerAction, OpenInWarpBannerState, SSHBannerAction,
    SSHBannerState, VimModeBannerAction,
};
pub use inline_banner::{NotificationsDiscoveryBannerAction, NotificationsErrorBannerAction};
#[cfg(feature = "local_fs")]
use repo_metadata::repositories::{DetectedRepositories, RepoDetectionSource};
use ssh_file_upload::{FileUpload, FileUploadEvent};
use uuid::Uuid;
use warpui::elements::ChildView;
use warpui::fonts::Properties;
use warpui::{ViewHandle, WeakModelHandle};

#[cfg(feature = "local_fs")]
use crate::code::editor_management::CodeSource;
use crate::context_chips::prompt::Prompt;
use crate::context_chips::prompt_type::PromptType;
use crate::context_chips::ContextChipKind;
use crate::env_vars::{
    env_var_collection_block::{EnvVarCollectionBlock, EnvVarCollectionBlockEvent},
    EnvVar, EnvVarCollection,
};
use crate::pane_group::focus_state::PaneFocusHandle;
use crate::persistence::{self, FinishedCommandMetadata};
use crate::safe_warn;
#[cfg(feature = "local_fs")]
use crate::settings::import::model::ImportedConfigModel;
use crate::settings::import::view::{SettingsImportEvent, SettingsImportView};
use crate::settings::{
    AliasExpansionSettings, AppEditorSettings, BlockVisibilitySettings,
    BlockVisibilitySettingsChangedEvent, CLIAgentSettings, DebugSettings,
    DebugSettingsChangedEvent, EmacsBindingsSettings, FontSettings, FontSettingsChangedEvent,
    InputModeSettings, InputModeSettingsChangedEvent, InputSettings, PaneSettings,
    PaneSettingsChangedEvent, SelectionSettings, VimBannerSettings,
};
use crate::settings_view::flags;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::settings_view::SettingsSection;
use crate::shell_indicator::ShellIndicatorType;
use crate::terminal::alias::{check_for_alias_async, AliasedCommand};
use crate::terminal::alt_screen_reporting::{AltScreenReporting, AltScreenReportingChangedEvent};
use crate::terminal::block_filter::{
    filter_button_position_id, BlockFilterEditor, BlockFilterEditorEvent, BlockFilterQuery,
    OpenedFromClick,
};
use crate::terminal::block_list_viewport::OverhangingBlock;
use crate::terminal::block_list_viewport::ScrollPositionUpdate;
use crate::terminal::block_list_viewport::ScrollState;
use crate::terminal::general_settings::GeneralSettings;
use crate::terminal::grid_size_util::grid_cell_dimensions;
use crate::terminal::input::decorations::InputBackgroundJobOptions;
use crate::terminal::input::CommandExecutionSource;
use crate::terminal::ligature_settings::{should_use_ligature_rendering, LigatureSettings};
#[cfg(feature = "local_tty")]
use crate::terminal::local_tty::get_shell_starter;
#[cfg(feature = "local_tty")]
use crate::terminal::local_tty::shell::ShellStarter;
use crate::terminal::model::session::active_session::ActiveSession;
use crate::terminal::model::session::{Session, SessionId};
use crate::terminal::model::{ObfuscateSecrets, RespectObfuscatedSecrets, SecretHandle};
use crate::terminal::recorder::PtyRecorder;
use crate::terminal::safe_mode_settings::get_secret_obfuscation_mode;
use crate::terminal::session_settings::ToolbarChipSelection;
use crate::terminal::session_settings::{
    NotificationsMode, NotificationsSettings, SessionSettings,
};
use crate::terminal::session_settings::{
    SessionSettingsChangedEvent, DEFAULT_THRESHOLD_FOR_LONG_RUNNING_NOTIFICATION,
};
use crate::terminal::settings::{TerminalSettings, TerminalSettingsChangedEvent};
use crate::terminal::ssh::ssh_detection::SshInteractiveSessionDetected;
use crate::terminal::warpify::{
    render::render_subshell_separator, settings::WarpifySettings, SubshellSource,
};
use crate::terminal::ShellLaunchData;
use crate::terminal::{element_size_at_last_frame, HistoryEntry};
use crate::terminal::{height_in_range_approx, heights_approx_gt, SizeUpdate};
use crate::terminal::{heights_approx_eq, CellSizeAndWindowPadding};
use crate::terminal::{AudibleBell, SizeUpdateReason};
use crate::terminal::{BlockListSettings, BlockListSettingsChangedEvent};
use crate::themes::theme::WarpTheme;
use crate::ui_components::icons::{self};
use crate::util::bindings::{
    custom_tag_to_keystroke, keybinding_name_to_display_string, keybinding_name_to_keystroke,
    set_custom_keybinding, CustomAction,
};
use crate::util::clipboard::clipboard_content_with_escaped_paths;
#[cfg(feature = "local_fs")]
use crate::util::openable_file_type::{is_markdown_file, resolve_file_target, FileTarget};
use crate::view_components::{DismissibleToast, ToastFlavor};
use crate::workflows::workflow::Workflow;
use crate::workspace::sync_inputs::SyncedInputState;
use crate::workspace::{CommandSearchOptions, ToastStack, WorkspaceAction};

use crate::report_if_error;

use crate::ActiveSession as WindowActiveSession;
use warp_server_client::ids::SyncId;

use async_channel::{Receiver, Sender};
use chrono::{Local, NaiveDateTime};
use command_corrections::rules::{Rule, RuleId as CommandCorrectionsRuleId};
use command_corrections::{correct_command, Command, Correction, HistoryItem, SessionMetadata};
use enclose::enclose;
use instant::Instant;
use itertools::Itertools;
use lazy_static::lazy_static;
use markdown_parser::FormattedTextFragment;
use parking_lot::FairMutex;
use pathfinder_color::ColorU;
use regex::Regex;
use serde::Serialize;
use std::any::Any;
use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use vec1::vec1;
use warp_core::context_flag::ContextFlag;
use warp_core::user_preferences::GetUserPreferences as _;
#[cfg(feature = "local_fs")]
use warp_util::path::LineAndColumnArg;
use warp_util::path::ShellFamily;
use warpui::clipboard::ClipboardContent;
use warpui::elements::new_scrollable::{
    AxisConfiguration, ClippedAxisConfiguration, DualAxisConfig, NewScrollableElement,
    ScrollableAppearance, SingleAxisConfig,
};
use warpui::elements::{
    get_rich_content_position_id, ChildAnchor, ClippedScrollStateHandle, Container,
    DispatchEventResult, DropTarget, DropTargetData, Empty, EventHandler, Flex, NewScrollable,
    OffsetPositioning, ParentAnchor, ParentElement, ParentOffsetBounds, PositionedElementAnchor,
    PositionedElementOffsetBounds, Radius, ScrollableElement, ScrollbarWidth, Shrinkable, Text,
};
use warpui::event::ModifiersState;
use warpui::keymap::Keystroke;
use warpui::notification::{NotificationSendError, RequestPermissionsOutcome, UserNotification};
use warpui::platform::{Cursor, OperatingSystem};
use warpui::r#async::Timer;
use warpui::windowing::WindowManager;

use warpui::assets::asset_cache::{AssetCache, AssetCacheEvent};
use warpui::image_cache::ImageType;
use warpui::units::{IntoLines, IntoPixels, Lines, Pixels};
use warpui::{
    accessibility::{AccessibilityContent, ActionAccessibilityContent, WarpA11yRole},
    elements::SavePosition,
    elements::{
        Align, Clipped, ConstrainedBox, CornerRadius, Fill, Hoverable, Icon, MouseStateHandle,
        Rect, ScrollStateHandle, Scrollable,
    },
    fonts::{Cache as FontCache, FamilyId},
    ui_components::components::UiComponent,
    AccessibilityData, AppContext, BlurContext, Element, Entity, FocusContext, ModelHandle,
    TypedActionView, UpdateView, View, ViewContext, WeakViewHandle,
};
use warpui::{
    elements::Stack,
    end_trace_after_next,
    geometry::vector::{vec2f, Vector2F},
    record_trace_event, WindowId,
};

use warpui::{windowing, CursorInfo, EntityId, EventContext, ModelAsRef, SingletonEntity, Tracked};

use crate::appearance::{Appearance, AppearanceEvent};
use crate::banner::{
    Banner, BannerAction, BannerEvent, BannerState, BannerTextButton, BannerTextContent,
    DismissalType,
};
use crate::debounce::debounce;
use crate::editor::{CrdtOperation, EditorAction};
use crate::features::FeatureFlag;
use crate::pane_group::SplitPaneState;
use crate::pane_group::{
    CodeReviewPanelArg, PaneConfiguration, PaneEvent, PaneGroupAction, TerminalViewResources,
};
use crate::resource_center::{
    mark_feature_used_and_write_to_user_defaults, Tip, TipHint, TipsCompleted,
};
use crate::session_management::{CommandContext, SessionNavigationPromptElements};
use crate::terminal::alt_screen::alt_screen_element::AltScreenElement;
use crate::terminal::block_list_element::{
    render_hoverable_block_button, BlockListElement, BlockListMouseStates, BlockSelectAction,
    BlockTextSelectAction, SnackbarHeaderState, ToolbeltButtonTooltip,
};
use crate::terminal::block_list_viewport::AutoscrollBehavior;
use crate::terminal::block_list_viewport::{InputMode, ScrollPosition, ViewportState};
use crate::terminal::bootstrap::init_subshell_command;
use crate::terminal::event::TerminalMode;
use crate::terminal::event::UserBlockCompleted;
use crate::terminal::find::{BlockGridMatch, BlockListMatch, TerminalFindModel};
use crate::terminal::input::{MenuPositioning, MenuPositioningProvider};
use crate::terminal::metadata::ToggleBlockFilterSource;
use crate::terminal::model::block::BlockMetadata;
use crate::terminal::model::block::{Block, BlockId};
use crate::terminal::model::blocks::BlockList;
use crate::terminal::model::blocks::Gap;
use crate::terminal::model::escape_sequences::{self, EscCodes, ToEscapeSequence, C1};
use crate::terminal::model::grid::grid_handler::{FragmentBoundary, TermMode};
use crate::terminal::model::index::{Point, Side};
use crate::terminal::model::mouse::MouseState;
use crate::terminal::model::selection::{SelectAction, SelectionDirection};
use crate::terminal::model::session::{BootstrapSessionType, Sessions, SessionsEvent};
use crate::terminal::model::terminal_model::{BlockIndex, TerminalInputState};
use crate::terminal::model::terminal_model::{
    BlockSelectionCardinality, SelectedBlocks, WithinModel,
};
use crate::terminal::model::{
    ansi::{ClearMode, Handler},
    blocks::BlockListPoint,
};
use crate::terminal::view::inline_banner::{
    AliasExpansionBannerState, NotificationsDiscoveryBannerState, NotificationsErrorBannerState,
    VimModeBannerState,
};
use crate::terminal::view::ssh_file_upload::FileUploadId;
use crate::terminal::waterfall_gap_element::WaterfallGapElement;
use crate::terminal::ShellHost;
use crate::terminal::{
    block_list_element::BlockHoverAction,
    // find::{Event as FindEvent, Find, FindDirection},
    input::{Event as InputEvent, Input, INPUT_A11Y_HELPER, INPUT_A11Y_LABEL},
    model::block::SerializedBlock,
    shell::ShellType,
    terminal_size_element::TerminalSizeElement,
    TerminalModel,
};
use crate::view_components::find::{Event as FindEvent, Find, FindDirection, FindWithinBlockState};
use crate::workspace::metadata::PaletteSource;
use settings::{Setting, ToggleableSetting};
use warp_core::semantic_selection::SemanticSelection;
use warpui::text::SelectionType;

use self::link_detection::HighlightedLinkOption;
use super::available_shells::AvailableShell;
use super::block_list_viewport::FindMatchScrollLocation;
use super::event::SshLoginStatus;
use super::find::FindOptions;
use super::model::ansi::{SystemDetails, WarpificationUnavailableReason};
use super::model::block::{
    BlockSection, BlocklistEnvVarMetadata, LONG_RUNNING_COMMAND_DURATION_MS,
};
use super::model::completions::ShellCompletion;
use super::model::rich_content::RichContentType;

use super::model::selection::ExpandedSelectionRange;
use super::model::session::SessionBootstrappedEvent;
use super::settings::AltScreenPaddingMode;
use super::ssh::error::{SshErrorBlock, SshErrorBlockEvent, SSH_ERROR_BLOCK_VISIBLE_KEY};
use super::ssh::install_tmux::{
    install_root_tmux_script, install_tmux_script, SshInstallTmuxBlock, SshInstallTmuxBlockEvent,
    SshKeyEvent, TmuxInstallMethod,
};
use super::ssh::root_access::RootAccess;
use super::ssh::ssh_detection::evaluate_warpify_ssh_host;
use super::ssh::util::{
    convert_script_to_one_line, parse_interactive_ssh_command, InteractiveSshCommand,
    SshWarpifyCommand,
};
use super::ssh::warpify::{
    begin_warpify_ssh_session_command, warpify_ssh_session_command, SshWarpifyBlock,
    SshWarpifyBlockEvent,
};
use super::ssh::SSH_WARPIFY_TIMEOUT_DURATION;
use super::warpify::success_block::{WarpifySuccessBlock, WarpifySuccessBlockEvent};
use super::warpify::trigger_state::{SshBlockState, WarpifyState};
use super::warpify::WarpificationSource;
use super::GridType;
use crate::menu::{Event as MenuEvent, Menu, MenuItem, MenuItemFields};
use crate::terminal::links::should_directly_open_link;
use crate::terminal::model_events::{AnsiHandlerEvent, ModelEvent, ModelEventDispatcher};
use crate::terminal::{block_list_element::BlockListMenuSource, prompt};
use crate::terminal::{color, History, SizeInfo};
use crate::terminal::{color::List, model::block::LONG_RUNNING_BOTTOM_PADDING_LINES};
use crate::terminal::{event::AfterBlockCompletedEvent, event::BlockLatencyData, event::BlockType};
use crate::throttle::throttle;
use crate::util::color::darken;
use action::RememberForWarpification;
use block_banner::{render_warpification_banner, WarpificationMode, WarpifyBannerState};
use bookmarks::render_floating_block_snapshot;
use command_corrections::rules::generic::history::History as CommandCorrectionsHistoryRule;

use warp_core::command::ExitCode;

lazy_static! {
    // A set of commands that perform minimal work that we use as a baseline to measure the latency of blocks.
    // Note that while the empty command doesn't invoke pre-exec, it still does get a newline from
    // the shell, and runs precmd.
    static ref BASELINE_COMMANDS: HashSet<&'static str> = HashSet::from(["", "pwd", "whoami", "cd"]);

    // A regex to detect a class of error strings indicating the ControlMaster connection is
    // broken.
    pub static ref CONTROL_MASTER_ERROR_REGEX: regex::Regex =
        regex::Regex::new(r"(?m)^channel (\d)+: open failed:")
        .expect("The regex should compile");

    /// A regex to detect Unix- or Windows-style line feeds in text.
    pub static ref LINEFEED_REGEX: Regex = Regex::new("\r?\n").expect("should not fail to compile regex");

    /// Show the jump to bottom of block button if more than this height of the block is in view.
    static ref JUMP_TO_BOTTOM_OVERHANG_THRESHOLD_PX: Pixels = (70.).into_pixels();

    static ref JUMP_TO_BOTTOM_OF_BLOCK_ICON_SIZE_PX: Pixels = (20.).into_pixels();
    static ref JUMP_TO_BOTTOM_OF_BLOCK_BUTTON_PADDING_PX: Pixels = (4.).into_pixels();
    static ref JUMP_TO_BOTTOM_OF_BLOCK_CORNER_RADIUS_PX: Pixels = (4.).into_pixels();
    static ref JUMP_TO_BOTTOM_OF_BLOCK_TOOLTIP_OFFSET_Y_PX: Pixels = (-5.).into_pixels();


    static ref SUBSHELL_BANNER_DELAY_DURATION: Duration = if cfg!(feature = "integration_tests") {
        Duration::from_secs(0)
    } else {
        Duration::from_secs(1)
    };

    /// The delay between receiving the RC file snippet for subshell bootstrap and writing the
    /// subshell InitShell command to the PTY.
    ///
    /// This is necessary because some subshells may execute initialization commands (for example,
    /// `poetry shell` executes a command that sources the project's python virtualenv), and we
    /// want to submit the InitShell command _after_ those commands have finished execution.
    ///
    /// This is purely a heuristic and may be subject to change based on user reports.
    static ref TRIGGER_RC_FILE_SUBSHELL_BOOTSTRAP_DELAY: Duration = Duration::from_millis(100);

    static ref DEFAULT_IGNORED_RULES_FOR_COMMAND_CORRECTIONS: [CommandCorrectionsRuleId; 1] = [
        CommandCorrectionsHistoryRule.id()
    ];

    /// A list of alt-screen apps that are known to cause problems when resizing
    /// during initialization.
    ///
    /// See [`TerminalView::resize_alt_screen_redundantly`] for more details.
    static ref ALT_SCREEN_APPS_WITH_RESIZE_PROBLEMS: HashSet<&'static str> = HashSet::from(["emacs"]);

    /// A list of alt-screen apps that should never use custom-padding in the alt-screen
    /// and should instead match blocklist padding.
    ///
    /// See [`TerminalView::resize_alt_screen_redundantly`] for more details.
    static ref ALT_SCREEN_APPS_THAT_MUST_MATCH_BLOCKLIST_PADDING: HashSet<&'static str> = HashSet::from(["k9s", "lazygit"]);
}

pub const OVERFLOW_BUTTON_OFFSET_X: f32 = -3.;
pub const MAX_WAKEUPS_PER_SECOND: u64 = 60;
pub const WAKEUP_THROTTLE_PERIOD: Duration =
    Duration::from_micros(1000 * 1000 / MAX_WAKEUPS_PER_SECOND);

pub const EXECUTE_PENDING_COMMAND_DELAY: Duration = Duration::from_millis(100);

pub const WARP_PROMPT_HEIGHT_LINES: f32 = 0.9;

const SCROLLBAR_WIDTH: ScrollbarWidth = ScrollbarWidth::Auto;

/// Width of the bookmark indicator
const BOOKMARK_INDICATOR_WIDTH: f32 = 15.;
/// Offset from the right for the bookmark preview
const BOOKMARK_PREVIEW_OFFSET: f32 = 20.;
/// Minimum gap between two bookmark indicators
const BOOKMARK_MIN_GAP: f32 = 4.;
/// Height of a bookmark indicator
const BOOKMARK_INDICATOR_HEIGHT: f32 = 4.;

const BRACKETED_PASTE_PREFIX: &str = "\x1b[200~";
const BRACKETED_PASTE_SUFFIX: &str = "\x1b[201~";

/// Duration before we consider a session to have failed bootstrapping.
const BOOTSTRAP_FAILED_DURATION: Duration = Duration::from_secs(7);
/// Duration before we consider a session invoked from an env vars object to
/// have failed bootstrapping. The longer duration is meant to account for
/// a user needing to type in one or many secret manager passwords
/// during the bootstrap period.
const ENV_VAR_BOOTSTRAP_FAILED_DURATION: Duration = Duration::from_secs(60);
const KNOWN_ISSUES_URL: &str =
    "https://docs.warp.dev/support-and-community/troubleshooting-and-support/known-issues";

/// Link to supported custom prompts.
const PROMPT_COMPATIBILITY_URL: &str =
    "https://docs.warp.dev/terminal/appearance/prompt#custom-prompt-compatibility-table";

/// Link to troubleshooting steps for ControlMaster errors.
const CONTROLMASTER_ISSUES_URL: &str =
    "https://docs.warp.dev/terminal/warpify/ssh-legacy#troubleshooting";

/// Link to instructions on how to update p10k.
const P10K_UPDATE_INSTRUCTIONS_URL: &str =
    "https://github.com/romkatv/powerlevel10k#how-do-i-update-powerlevel10k";

const CONTEXT_MENU_WIDTH: f32 = 280.;

/// The minimum amount of mouse-drag to consider a selection to
/// be a text-selection as opposed to mouse-drag noise.
/// Roughly determined by trial-and-error.
const MIN_DELTA_FOR_TEXT_SELECTION: f32 = 0.5;

/// Notifications-specific info
/// TODO (suraj): add documentation for notifications in gitbook
const NOTIFICATIONS_LEARN_MORE_URL: &str =
    "https://docs.warp.dev/terminal/more-features/notifications";
pub const NOTIFICATIONS_TROUBLESHOOT_URL: &str =
    "https://docs.warp.dev/terminal/more-features/notifications#troubleshooting-notifications";

const DEBOUNCE_PERIOD: Duration = Duration::from_millis(40);

/// Key used in user preferences to persist the "don't show again" choice for the OSC 52
/// clipboard blocked banner.
const OSC52_CLIPBOARD_BANNER_SUPPRESSED_KEY: &str = "Osc52ClipboardBannerSuppressed";

/// Which type of OSC 52 clipboard operation was blocked.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Osc52ClipboardBlockedType {
    Read,
    Write,
}

/// Key used in user defaults to save whether the user has seen the banner.
pub const ALIAS_EXPANSION_BANNER_SEEN_KEY: &str = "AliasExpansionBannerSeen";

/// Delay between receiving preexec hook for a command we want to auto-warpify
/// and triggering the warpification (subshell bootstrapping).
/// Reached this number after experimenting with different values to find a reliable delay.
const AUTO_WARPIFY_DELAY: u64 = 1000;

/// Binding names to be customized if the user indicates they prefer
/// Emacs-style keybindings instead of IDE-style keybindings.
/// These are specific to non-MacOS desktop platforms.
const SELECT_ALL_BINDING_NAME: &str = "editor_view:select_all";
const MOVE_LINE_START_BINDING_NAME: &str = "editor_view:move_to_line_start";
const MOVE_LINE_END_BINDING_NAME: &str = "editor_view:move_to_line_end";

pub const LONG_RUNNING_AGENT_REQUESTED_COMMAND_CONTEXT_KEY: &str = "LongRunningRequestedCommand";
pub const LONG_RUNNING_AGENT_REQUESTED_COMMAND_USER_TOOK_OVER_CONTEXT_KEY: &str =
    "LongRunningRequestedUserTookOverCommand";

/// We only auto open the code review pane if the pane it's getting opened from has a certain width
const MINIMUM_WIDTH_TO_AUTO_OPEN_PANE: f32 = 600.0;

lazy_static! {
    static ref CTRL_SHIFT_A_KEYSTROKE: Keystroke = Keystroke {
        key: "A".into(),
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    static ref CTRL_A_KEYSTROKE: Keystroke = Keystroke {
        key: "a".into(),
        ctrl: true,
        ..Default::default()
    };
    static ref CTRL_E_KEYSTROKE: Keystroke = Keystroke {
        key: "e".into(),
        ctrl: true,
        ..Default::default()
    };

    /// The padding between the left of the element and where the grid contents (either via the
    /// `BlockList` or the `AltScreen`) should be rendered.
    pub static ref PADDING_LEFT: f32 = if FeatureFlag::LessHorizontalTerminalPadding.is_enabled() {
        16.
    } else {
        20.
    };
}

#[derive(Default)]
pub struct ControlMasterErrorBannerState {
    /// Whether or not the control master error banner is currently visible to
    /// the user.
    pub is_open: bool,
    /// The session ID where the error occurred.  This is used to avoid making
    /// additional requests to check for control master errors if we've already
    /// showed the user the banner for this particular session.
    pub associated_session_id: Option<SessionId>,
}

/// Closed => No need for an error banner
/// Triggered => The banner is not open, but should be
/// Open => The banner error is currently open
#[derive(Default)]
pub enum NotificationsErrorBannerType {
    #[default]
    Closed,
    Triggered,
    Open {
        state: NotificationsErrorBannerState,
    },
}

#[derive(Default)]
/// Describes the current state of the notifications error banner
pub struct NotificationsErrorBanner {
    /// The error details
    pub error: Option<NotificationSendError>,
    /// The current state of the error banner (is it open or not)
    pub banner_type: NotificationsErrorBannerType,
}

#[derive(Debug, Clone)]
pub struct BlockNotification {
    pub title: String,
    pub body: String,
}

/// The reason for sending/discovering the notification
#[derive(Copy, Clone, Debug, Serialize)]
pub enum NotificationsTrigger {
    LongRunningCommand(bool /* command_succeeded */, Duration),
    AgentTaskCompleted(bool /* task_succeeded */),
    NeedsAttention,
    /// TODO: Remove this once desktop notifs are unflagged.
    PasswordPrompt,
}

impl NotificationsTrigger {
    pub fn discovery_banner_copy(&self) -> &'static str {
        match self {
            NotificationsTrigger::LongRunningCommand(..) => {
                "Warper can notify you when long-running commands finish."
            }
            NotificationsTrigger::AgentTaskCompleted(..) => {
                "Warper can notify you when an agent finishes responding."
            }
            NotificationsTrigger::NeedsAttention => {
                "Warper can notify you when a command or agent needs your attention."
            }
            NotificationsTrigger::PasswordPrompt => {
                "Warper can notify you when you're prompted to enter a password."
            }
        }
    }

    /// Notifications have the following format
    /// - title: "'{start_of_command}...' {trigger_specific_details}"
    /// - body: "{additional_context} ...{end_of_output}"
    ///
    /// For the command, we show the prefix (if not the whole command) since the user
    /// will likely be able to identify the command more easily by its prefix
    /// e.g. 'ssh user@...' vs '...nux.a.b.com'
    ///
    /// For the output, we show the suffix (if not the whole output) since
    /// the end of the output is what the user likely missed when the terminal
    /// wasn't focused.
    ///
    /// Note: we trim the ends of commands and outputs to remove whitespace
    /// which cause unpleasing gaps in the MacOS notifications.
    pub fn create_notification_content(
        &self,
        command: String,
        output: String,
    ) -> BlockNotification {
        use NotificationsTrigger::*;

        let (title_suffix, body_prefix) = match self {
            LongRunningCommand(command_succeeded, block_duration) => {
                let status = if *command_succeeded {
                    "finished"
                } else {
                    "failed"
                };

                let duration_seconds = block_duration.as_secs_f32();
                let duration_seconds = if duration_seconds >= 1. {
                    format!("{}", duration_seconds.round() as usize)
                } else {
                    format!("{duration_seconds:.1}")
                };

                (
                    format!(" {status} after {duration_seconds}s"),
                    "Latest output: ".to_string(),
                )
            }
            AgentTaskCompleted(command_succeeded) => {
                if *command_succeeded {
                    (" finished".to_string(), "Latest output: ".to_string())
                } else {
                    (" failed".to_string(), "Error: ".to_string())
                }
            }
            NotificationsTrigger::NeedsAttention => (" blocked".to_string(), "".to_string()),
            PasswordPrompt => (
                " is waiting for a password".to_string(),
                "Latest output: ".to_string(),
            ),
        };

        // Get rid of newlines in the command and output because it causes the
        // content of the MacOS notification to appear cutoff or janky.
        let command = command.replace('\n', "\\n");
        let output = output.replace('\n', " ");

        // TITLE

        // Trim off any whitespace from the beginning of the command
        let base_command = command.trim_start();
        let base_command_char_len = base_command.chars().count();

        // Reduce the max character count of the command by 2 for the surrounding quotes
        let title_prefix_max_char_length =
            UserNotification::MAX_TITLE_LENGTH - title_suffix.chars().count() - 2;

        let title_prefix = if title_prefix_max_char_length >= base_command_char_len {
            // The command fits entirely within the title so we can use it as is
            format!("'{}'", base_command.trim_end())
        } else {
            // Otherwise, the command doesn't fit and we need to take the first
            // few characters (minus 3 for the ellipsis) to show
            let end = base_command
                .chars()
                .take(title_prefix_max_char_length - 3)
                .map(|c| c.len_utf8())
                .sum();
            format!("'{}...'", base_command[..end].trim_end())
        };

        // BODY

        // Trim any whitespace off the end of the output
        let base_output = output.trim_end();
        let base_output_char_len = base_output.chars().count();

        let body_suffix_max_char_length =
            UserNotification::MAX_BODY_LENGTH - body_prefix.chars().count();

        let body_suffix = if body_suffix_max_char_length >= base_output_char_len {
            // The output fits entirely within the body so we can use it as is
            base_output.trim_start().to_string()
        } else {
            // Otherwise, the output doesn't fit and we need to take the last
            // few characters (minus 3 for the ellipsis) to show
            let start: usize = base_output.len()
                - base_output
                    .chars()
                    .rev()
                    .take(body_suffix_max_char_length - 3)
                    .map(|c| c.len_utf8())
                    .sum::<usize>();
            format!("...{}", base_output[start..].trim_start())
        };

        BlockNotification {
            title: format!("{title_prefix}{title_suffix}"),
            body: format!("{body_prefix}{body_suffix}"),
        }
    }
}

/// Closed => There is no need for a notifications discovery banner right now
/// Triggered => There is some reason to show the discovery banner, but it's not open yet.
///              For example, the discovery banner for password notifications won't be open
///              till the block completes, but the trigger is non-None
/// Open => The discovery banner is currently open
#[derive(Default)]
pub enum NotificationsDiscoveryBanner {
    #[default]
    Unset,
    Closed,
    Triggered(NotificationsTrigger),
    Open {
        trigger: NotificationsTrigger,
        // Track the request outcome to determine messaging in the banner.
        // None means that the request was not yet responded to.
        request_outcome: Option<RequestPermissionsOutcome>,
        state: NotificationsDiscoveryBannerState,
    },
}

struct ShellProcessTerminatedBanner {
    banner_id: InlineBannerId,
    was_premature_termination: bool,
}

/// A unique identifier for an inline banner.
pub type InlineBannerId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum InlineBannerType {
    NotificationsDiscovery,
    NotificationsError,
    Ssh,

    AliasExpansion,
    ShellProcessTerminated,
    OpenInWarp,
    VimMode,
}

impl InlineBannerType {}

/// An inline banner with its unique ID and type metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InlineBannerItem {
    pub id: InlineBannerId,
    pub banner_type: InlineBannerType,
}

impl InlineBannerItem {
    pub fn new(id: InlineBannerId, banner_type: InlineBannerType) -> Self {
        Self { id, banner_type }
    }
}

/// A unique identifier for a subshell separator.
pub type SeparatorId = usize;

#[derive(Default)]
struct InlineBannersState {
    /// The ID for the next inline banner to be created.
    next_banner_id: InlineBannerId,

    /// State for the different notification banners.
    notifications_discovery_banner: NotificationsDiscoveryBanner,
    notifications_error_banner: NotificationsErrorBanner,

    /// A mapping from banner ID to state information for all SSH banners in
    /// this view.
    ssh_banners: HashMap<InlineBannerId, SSHBannerState>,

    alias_expansion_banner: AliasExpansionBanner,

    /// Information for a banner which notifies the user that the
    /// shell process has terminated, or None if there is no
    /// banner to display.
    shell_process_terminated_banner: Option<ShellProcessTerminatedBanner>,

    open_in_warp_banner: Option<OpenInWarpBannerState>,

    vim_banner_state: Option<VimModeBannerState>,
}

impl InlineBannersState {
    /// Returns the ID to assign to the next inline banner.
    fn next_banner_id(&mut self) -> InlineBannerId {
        let next_id = self.next_banner_id;
        self.next_banner_id += 1;
        next_id
    }

    /// Returns the ID of the last inline banner inserted.
    #[allow(dead_code)]
    fn last_banner_id(&self) -> Option<InlineBannerId> {
        #[allow(clippy::unnecessary_lazy_evaluations)]
        (self.next_banner_id > 0).then(|| self.next_banner_id - 1)
    }
}

/// Helper struct for creating SizeUpdates.
#[derive(Debug)]
struct SizeUpdateBuilder {
    /// The reason for the size update.
    update_reason: SizeUpdateReason,

    /// The last size info prior to the update.
    last_size: SizeInfo,

    /// The new pane size in pixels.
    new_pane_size_px: Vector2F,
}

impl SizeUpdateBuilder {
    fn for_refresh(last_size: SizeInfo) -> Self {
        // Refreshing doesn't actually change pane size or content element size.
        Self {
            update_reason: SizeUpdateReason::Refresh,
            last_size,
            new_pane_size_px: last_size.pane_size_px(),
        }
    }

    fn after_layout(last_size: SizeInfo, new_pane_size_px: Vector2F) -> Self {
        Self {
            update_reason: SizeUpdateReason::AfterLayout,
            last_size,
            new_pane_size_px,
        }
    }

    fn build(self, view: &TerminalView, ctx: &ViewContext<TerminalView>) -> SizeUpdate {
        let appearance = view.appearance(ctx);
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let model = view.model.lock();

        let new_size = create_size_info(
            self.new_pane_size_px,
            &model,
            view.sessions.as_ref(ctx),
            ctx.font_cache(),
            appearance.monospace_font_family(),
            appearance.monospace_font_size(),
            appearance.line_height_ratio(),
            ctx,
        );

        // Capture the pane-computed natural size.
        let natural_rows = new_size.rows;
        let natural_cols = new_size.columns;

        // Adjust the gap size to maintain the model invariant that the height of the
        // gap + all block_heights after the gap equals the height of the current
        // space in which to render the blocklist.  Note that we also need to run this
        // same logic when the input mode switches to Waterfall.
        let viewport = view.viewport_state(model.block_list(), input_mode, ctx);
        let new_gap_height = match (input_mode, model.block_list().active_gap()) {
            (InputMode::Waterfall, Some(gap)) => {
                let block_list_height_without_gap =
                    model.block_list().block_heights().summary().height - gap.height();
                let max_scroll_top = viewport.max_scroll_top_in_lines();
                let input_id = view.input.as_ref(ctx).save_position_id();
                let input_height =
                    element_size_at_last_frame(input_id.as_str(), ctx.window_id(), ctx)
                        .map_or(0., |r| r.y())
                        .into_pixels()
                        .to_lines(new_size.cell_height_px());

                let new_height = max_scroll_top
                    + new_size
                        .pane_height_px()
                        .to_lines(new_size.cell_height_px())
                    - block_list_height_without_gap
                    - input_height;
                (!heights_approx_eq(new_height, gap.height())).then_some(new_height)
            }
            (_, _) => None,
        };

        SizeUpdate {
            update_reason: self.update_reason,
            last_size: self.last_size,
            new_size,
            new_gap_height,
            natural_rows,
            natural_cols,
        }
    }
}

struct FindLinkArg {
    position: WithinModel<Point>,
    from_editor: TerminalEditor,
}

#[derive(Debug, Clone, Copy)]
pub enum TerminalEditor {
    Yes,
    No,
}

/// Different modes for how we consider a block to be "visible"
#[derive(Debug, Clone, Copy)]
pub enum BlockVisibilityMode {
    /// A block is visible if its top is on screen
    TopOfBlockVisible,

    /// A block is visible if its bottom is on screen
    BottomOfBlockVisible,
}

#[derive(Clone)]
pub enum ContextMenuAction {
    InsertSelectedText,
    CopySelectedText,
    CopyUrl {
        url_content: String,
    },
    CopyBlocks,
    CopyBlockCommands,
    CopyBlockOutputs,
    CopyBlockFilteredOutputs,
    FindWithinBlock,
    ToggleBookmark,
    ScrollToBottomOfBlock,
    ScrollToTopOfBlock,
    CopyPrompt {
        position: PromptPosition,
        part: PromptPart,
    },
    CopyRprompt,
    EditPrompt,

    EditCLIAgentToolbar,

    OpenWorkflowModal,
}

#[derive(Clone)]
pub enum InputContextMenuAction {
    CutSelectedText,
    CopySelectedText,
    SelectAll,
    Paste,
    ShowCommandSearch,

    SaveAsWorkflow,
    ToggleInputHintText,
}

// Manually implementing Debug to avoid leaking sensitive information in logs
impl fmt::Debug for ContextMenuAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ContextMenuAction::*;

        match self {
            InsertSelectedText => f.write_str("InsertSelectedText"),
            CopySelectedText => f.write_str("CopySelectedText"),
            CopyBlocks => f.write_str("CopyBlocks"),
            CopyBlockCommands => f.write_str("CopyBlockCommands"),
            CopyBlockOutputs => f.write_str("CopyBlockOutputs"),
            FindWithinBlock => f.write_str("FindWithinBlock"),
            ScrollToBottomOfBlock => f.write_str("ScrollToBottomOfBlock"),
            ScrollToTopOfBlock => f.write_str("ScrollToTopOfBlock"),
            ToggleBookmark => f.write_str("BookmarkBlock"),
            CopyPrompt { position, part } => {
                write!(f, "CopyPrompt {{ position: {position:?}, part: {part:?} }}")
            }
            CopyRprompt => f.write_str("CopyRprompt"),
            // CopyUrl's debug output is limited, since the URLs come from command output
            CopyUrl { .. } => f.write_str("CopyUrl"),
            EditPrompt => f.write_str("EditPrompt"),

            EditCLIAgentToolbar => f.write_str("EditCLIAgentToolbar"),

            OpenWorkflowModal => f.write_str("OpenWorkflowModal"),
            CopyBlockFilteredOutputs => f.write_str("CopyBlockFilteredOutput"),
        }
    }
}

// Manually implementing Debug to avoid leaking sensitive information in logs
impl fmt::Debug for InputContextMenuAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use InputContextMenuAction::*;

        match self {
            CutSelectedText => f.write_str("CutSelectedText"),
            CopySelectedText => f.write_str("CopySelectedText"),
            SelectAll => f.write_str("SelectAll"),
            Paste => f.write_str("Paste"),
            ShowCommandSearch => f.write_str("CommandSearch"),

            SaveAsWorkflow => f.write_str("SaveAsWorkflow"),
            ToggleInputHintText => f.write_str("ToggleInputHintText"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PromptPosition {
    Block(BlockIndex),
    Input,
}

impl PromptPosition {
    fn block<'a>(&self, model: &'a TerminalModel) -> Option<&'a Block> {
        match self {
            PromptPosition::Block(block_index) => model.block_list().block_at(*block_index),
            PromptPosition::Input => Some(model.block_list().active_block()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum PromptPart {
    EntirePrompt,
    CondaContext,
    Pwd,
    GitBranch,
    VirtualEnv,
    ContextChip(ContextChipKind),
}

/// Arg for calculating the next bookmark position.
struct IndicatorPositionArg {
    remaining_indicator_count: usize,
    /// Previous rendered indicator top.
    previous_indicator_top: Pixels,
}

impl IndicatorPositionArg {
    fn next_indicator_top(
        &mut self,
        block_start: Lines,
        total_block_height: Lines,
        content_height: Pixels,
    ) -> Pixels {
        self.remaining_indicator_count -= 1;

        // Total height an indicator will take (its height + minimum gap between indicators).
        let indicator_height = BOOKMARK_INDICATOR_HEIGHT + BOOKMARK_MIN_GAP;
        let mut top = (content_height * (block_start / total_block_height).as_f64().into_pixels())
            .max(self.previous_indicator_top + indicator_height.into_pixels());

        let remaining_space = content_height - (top + indicator_height.into_pixels());
        let remaining_indicator_required_space =
            (self.remaining_indicator_count as f32 * indicator_height).into_pixels();

        // Only move the indicator up if there is not enough space for the remaining
        // indicators AND the new position won't cause the indicators' ordering to change
        // or result in a negative top.
        if remaining_space < remaining_indicator_required_space
            && content_height - remaining_indicator_required_space > self.previous_indicator_top
        {
            top = content_height - remaining_indicator_required_space;
        }

        self.previous_indicator_top = top;
        top
    }
}

#[derive(Clone)]
pub struct ExecuteCommandEvent {
    pub command: String,
    pub session_id: SessionId,

    /// If the command was executed from a saved local workflow, pass its ID here.
    pub workflow_id: Option<SyncId>,
    /// If the command was executed from a saved workflow, store the
    /// templated command here.
    pub workflow_command: Option<String>,

    /// `true` if the executed command should be added to session history.
    pub should_add_command_to_history: bool,

    pub source: CommandExecutionSource,
}

pub enum Event {
    AppStateChanged,
    Escape,
    Exited,
    BlockListCleared,
    SendNotification(BlockNotification),
    BlockCompleted {
        block: Arc<SerializedBlock>,
        is_local: bool,
    },
    Pane(PaneEvent),
    OpenSettings(SettingsSection),
    /// Event propogates terminal inputs up to the workspace,
    /// to be processed on the way back down through the view hierarchy.
    SyncInput(SyncEvent),
    /// Event used to propagate a state change for one of the terminal views
    /// inside this pane group.
    TerminalViewStateChanged,
    ShowCommandSearch(CommandSearchOptions),
    // Tell the pane group to open the workflow modal.
    OpenWorkflowModalWithCommand(String),
    // Tell the pane group to open the workflow modal with an existing cloud workflow.
    OpenWorkflowModalWithLocalWorkflow(SyncId),
    // Tell the pane group to open the workflow modal with an unsaved workflow.
    OpenWorkflowModalWithTemporary(Box<Workflow>),

    OpenPromptEditor,

    OpenCLIAgentToolbarEditor,

    CtrlD,
    ShutdownPty,
    // TODO: break this event down into higer-level events that hide the
    // `bytes` detail from the view.
    WriteBytesToPty {
        bytes: Cow<'static, [u8]>,
    },

    Resize {
        size_update: SizeUpdate,
    },
    ExecuteCommand(ExecuteCommandEvent),
    BlockStarted {
        is_for_in_band_command: bool,
    },
    /// Tell the pane group to open a file within Warp.
    OpenFileInWarp {
        path: PathBuf,
        /// The session that the file belongs to.
        session: Arc<Session>,
    },
    #[cfg(feature = "local_fs")]
    OpenCodeInWarp {
        source: CodeSource,
        layout: EditorLayout,
    },
    #[cfg(feature = "local_fs")]
    PreviewCodeInWarp {
        source: CodeSource,
    },

    OpenCodeReviewPane(CodeReviewPanelArg),
    ToggleCodeReviewPane(CodeReviewPanelArg),

    CloseRequested,
    /// Used to focus and bring this session to the foreground.
    FocusSession,
    SelectedBlocksChanged,
    SelectedTextChanged,
    /// The input editor was locally edited and
    /// peers should be notified, if applicable.
    InputEditorUpdated {
        /// The block ID associated to the buffer that
        /// these operations were made in.
        block_id: BlockId,

        /// The CRDT-compliant operations.
        operations: Rc<Vec<CrdtOperation>>,
    },
    /// Emitted when a pending command (e.g. tab config setup commands) has
    /// been submitted and its block has completed.
    PendingCommandCompleted,
    SessionBootstrapped,
    ShellSpawned(ShellType),

    /// This terminal pane has initiated a file upload to a remote host.
    CopyFileToRemote {
        command: String,
        upload_id: FileUploadId,
    },
    /// This terminal pane is taking care of a file upload to a remote host
    /// and requires a password.
    FileUploadPasswordPending,
    /// This terminal pane was taking care of a file upload to a remote host
    /// and just finished a block.
    FileUploadFinished(ExitCode),
    /// Open the terminal pane that is taking care of a file upload to
    /// this pane's remote host.
    OpenFileUploadSession(FileUploadId),
    /// Terminate the session that took care of a file upload for this pane's
    /// remote host.
    TerminateFileUploadSession(FileUploadId),
    RunNativeShellCompletions {
        buffer_text: String,
        results_tx: async_channel::Sender<Vec<ShellCompletion>>,
    },
    OpenThemeChooser,

    OpenFilesPalette {
        source: PaletteSource,
    },
    #[cfg(feature = "local_fs")]
    OpenFileWithTarget {
        path: PathBuf,
        target: FileTarget,
        line_col: Option<LineAndColumnArg>,
    },
    /// Emitted when a file in the file tree is renamed.
    #[cfg(feature = "local_fs")]
    FileRenamed {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    /// Emitted when a file in the file tree is deleted.
    #[cfg(feature = "local_fs")]
    FileDeleted {
        path: PathBuf,
    },
    /// Toggle the left panel to a specific view
    ToggleLeftPanel {
        target_view: LeftPanelTargetView,
        force_open: bool,
    },
    SlowBootstrap,

    OpenPluginInstructionsPane(CLIAgent, PluginModalKind),
    ShowToast {
        message: String,
        flavor: ToastFlavor,
    },
    /// A pluggable notification triggered via OSC 9 or OSC 777 escape sequences.
    /// Used to show an in-app toast notification.
    PluggableNotification {
        title: Option<String>,
        body: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum LeftPanelTargetView {
    FileTree,
}

#[derive(Clone)]
pub struct SyncEvent {
    /// Used to prevent updating the source of the changes.
    /// Note: `StartSyncing` and `StopSyncing` don't use `source_view_id`
    /// because they should be acted on regardless of where
    /// the event originated (e.g., a terminal view should sync itself).
    pub source_view_id: EntityId,
    pub data: SyncInputType,
}

/// Event used to propogate the keyboard events from one terminal to others.
#[derive(Clone)]
pub enum SyncInputType {
    /// Event for when the input editor's buffer contents changed.
    InputEditorContentsChanged {
        /// Note: Using Arc because to make efficient cloning of large string possible
        contents: Arc<String>,
    },
    /// Event to handle user keyboard input to
    /// the alt-screen or long-running commands/
    NonEditorTyped {
        /// Characters the user inputted
        /// Note: Using Arc because to make efficient cloning of large string possible
        chars: Arc<Vec<u8>>,
    },
    /// Event used to to run commands in all synced terminals with
    /// visible input editors.
    RanCommand,
    /// Event used to notify that this terminal should be synced but we don't
    /// need to update its input editor or write to its PTY.
    StartSyncing,
    /// Event tells us we should stop syncing this terminal.
    StopSyncing,
}

#[derive(Debug, Copy, Clone)]
pub enum ContextMenuType {
    /// Opened via right-clicking within any block or using a block's 3-dot menu.
    BlockList { menu_source: BlockListMenuSource },
    /// Opened via right-clicking anywhere on the alt-screen.
    AltScreen { position: Vector2F },
    /// Opened via right-clicking on the input prompt.
    Prompt { position: Vector2F },
    /// Opened via right-clicking on the input box.
    Input { position: Vector2F },
}

impl ContextMenuType {
    pub fn origin(&self) -> Option<Vector2F> {
        match self {
            ContextMenuType::BlockList { menu_source } => match menu_source {
                BlockListMenuSource::RegularBlockRightClick {
                    position_in_terminal_view,
                    ..
                } => Some(*position_in_terminal_view),
                BlockListMenuSource::OutsideBlockRightClick {
                    position_in_terminal_view,
                    ..
                } => Some(*position_in_terminal_view),
                // We may be able to get the point from the row/col
                BlockListMenuSource::BlockOverflowButton { .. } => None,
                BlockListMenuSource::BlockKeybinding { .. } => None,
                BlockListMenuSource::RegularTextRightClick {
                    position_in_terminal_view,
                } => Some(*position_in_terminal_view),
                BlockListMenuSource::RichContentBlockRightClick {
                    position_in_terminal_view,
                    ..
                } => Some(*position_in_terminal_view),
                BlockListMenuSource::RichContentTextRightClick { .. } => None,
            },
            ContextMenuType::AltScreen { position } => Some(*position),
            ContextMenuType::Prompt { position } => Some(*position),
            ContextMenuType::Input { position } => Some(*position),
        }
    }
}

#[derive(Copy, Clone)]
pub struct ContextMenuInfo {
    menu_type: ContextMenuType,
}

impl ContextMenuInfo {
    pub fn type_for_event_metadata(&self) -> &'static str {
        match self.menu_type {
            ContextMenuType::BlockList { .. } => "Block",
            ContextMenuType::Prompt { .. } => "Prompt",
            ContextMenuType::Input { .. } => "Input",
            ContextMenuType::AltScreen { .. } => "AltScreen",
        }
    }

    pub fn open_method_for_event_metadata(&self) -> &'static str {
        match self.menu_type {
            ContextMenuType::BlockList { menu_source } => match menu_source {
                BlockListMenuSource::BlockOverflowButton { .. } => "BlockOverflowButton",
                BlockListMenuSource::BlockKeybinding { .. } => "Keybinding",
                BlockListMenuSource::RegularBlockRightClick { .. } => "RightClick",
                BlockListMenuSource::RegularTextRightClick { .. } => "RightClick",
                BlockListMenuSource::RichContentBlockRightClick { .. } => "OutsideBlockRightClick",
                BlockListMenuSource::RichContentTextRightClick { .. } => "OutsideBlockRightClick",
                BlockListMenuSource::OutsideBlockRightClick { .. } => "OutsideBlockRightClick",
            },
            ContextMenuType::Prompt { .. } => "RightClick",
            ContextMenuType::Input { .. } => "RightClick",
            ContextMenuType::AltScreen { .. } => "AltScreen",
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct ContextMenuState {
    menu_type: ContextMenuType,
}

#[derive(Copy, Clone)]
pub enum BlockEntity {
    Command,
    Output,
    FilteredOutput,
    CommandAndOutput,
}

impl BlockEntity {
    pub fn as_str(&self) -> &'static str {
        match self {
            BlockEntity::Command => "Command",
            BlockEntity::Output => "Output",
            BlockEntity::CommandAndOutput => "Both",
            BlockEntity::FilteredOutput => "FilteredOutput",
        }
    }
}

/// Groups together some structs to represent the state of the Terminal View for the
/// current frame. Passed to `AltScreenElement` and `BlockListElement`.
pub struct TerminalViewRenderContext {
    pub size_info: SizeInfo,
    pub scroll_position: ScrollPosition,
    pub highlighted_url: Option<GridHighlightedLink>,
    pub link_tool_tip: Option<GridHighlightedLink>,
    pub is_terminal_focused: bool,
    pub is_terminal_selecting: bool,
    pub is_context_menu_open: bool,
    pub is_waterfall_gap_mode: bool,
    pub pane_state: SplitPaneState,
    pub active_session_state: ActiveSessionState,
    pub selected_blocks: SelectedBlocks,
    /// Identifier for retrieving the position information of the input box element.
    pub input_box_element_key: String,
    /// Unique view id for saving active cursor position.
    pub terminal_view_id: EntityId,
    /// This map contains the IDs of sessions that were subshells as keys. Their corresponding
    /// values are the command that spawned the subshell, which is needed to paint the "flag"
    pub spawning_command_for_subshell_sessions: HashMap<SessionId, SubshellSource>,

    pub obfuscate_secrets: ObfuscateSecrets,
    pub hovered_secret: Option<SecretHandle>,

    pub horizontal_clipped_scroll_state: ClippedScrollStateHandle,
}

#[derive(Default)]
struct TerminalViewMouseStates {
    grid_link_tooltip: MouseStateHandle,
    rich_content_link_tooltip: MouseStateHandle,

    // Shared across Grid and Rich Content secrets tooltips (only 1 can be open at a time).
    toggle_secrets_tooltip: MouseStateHandle,
    copy_secrets_tooltip: MouseStateHandle,

    #[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
    open_in_warp_tooltip: MouseStateHandle,
    jump_to_bottom_of_block_button: MouseStateHandle,
}

/// Where content was routed when sent to a CLI agent.
/// Returned by [`TerminalView::try_send_text_to_cli_agent_or_rich_input`]
/// so callers can report the correct telemetry destination without a
/// separate read of the rich input state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliAgentRouting {
    /// Content was inserted into CLI agent rich input.
    RichInput,
    /// Content was written directly to the PTY.
    Pty,
}

/// An enum representing the different states that a terminal view can be in,
/// based on any commands it's actively running and the result of the most
/// recent command that it finished.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TerminalViewState {
    /// The most recent command had a non-successful exit code.
    Errored,
    /// Currently running a command.
    LongRunning,
    /// Not running any commands, and the last command it ran (if any) was
    /// successful.
    Normal,
}

/// A struct containing information about a state change event for a particular
/// terminal view.
#[derive(Copy, Clone)]
pub struct TerminalViewStateChange {
    pub state: TerminalViewState,
    pub timestamp: Instant,
}

impl Default for TerminalViewStateChange {
    fn default() -> TerminalViewStateChange {
        TerminalViewStateChange {
            state: TerminalViewState::Normal,
            timestamp: Instant::now(),
        }
    }
}

/// Whether or not this is the active terminal session. The active session for a pane group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveSessionState {
    Active,
    Inactive,
}

enum SecretTooltip {
    Grid { tooltip: WithinModel<SecretHandle> },
}

#[derive(Debug, Clone)]
pub struct TerminalDropTargetData {
    pub terminal_view: WeakViewHandle<TerminalView>,
}

impl DropTargetData for TerminalDropTargetData {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct TerminalView {
    pub model: Arc<FairMutex<TerminalModel>>,
    view_handle: WeakViewHandle<Self>,

    /// The session's size data. This is wrapped in a [`Tracked`] to
    /// guarantee that the [`TerminalView`] is redrawn whenever the
    /// size info changes.
    size_info: Tracked<SizeInfo>,

    /// The input area at the bottom of the viewport.
    input: ViewHandle<Input>,

    /// Colors used for rendering.
    colors: color::List,

    /// The current scroll position.
    scroll_position: ScrollState,

    /// Scroll state for scrolling vertically in the blocklist.
    blocklist_vertical_scroll_state: ScrollStateHandle,

    /// Scroll state for scrolling vertically in the alt screen.
    alt_screen_vertical_scroll_state: ScrollStateHandle,
    /// Lines from the top of the content we are scrolled in the alt screen.
    alt_screen_scroll_top: Lines,

    /// Scroll state for scrolling horizontally.
    horizontal_clipped_scroll_state: ClippedScrollStateHandle,

    /// Whether there is an active text selection.
    is_selecting: bool,

    context_menu: ViewHandle<Menu<TerminalAction>>,

    /// None iff there is no context menu open currently.
    context_menu_state: Option<ContextMenuState>,

    /// The search bar at the top of the terminal view.
    find_bar: ViewHandle<Find<TerminalFindModel>>,

    /// The block whose filter we are actively editing.
    active_filter_editor_block_index: Option<BlockIndex>,
    block_filter_editor: ViewHandle<BlockFilterEditor>,

    hovered_block_index: Option<BlockIndex>,

    selected_blocks: SelectedBlocks,

    // Whether any session contains blocks from a remote session. Cached to improve performance.
    // Blocks don't necessarily need to be finished for this to be true (e.g. it's true for
    // an empty ssh session where just the active block is remote).
    any_session_contains_remote_blocks: bool,

    // Whether any session contains restored blocks from a remote session. Cached to improve performance.
    any_session_contains_restored_remote_blocks: bool,

    /// Mouse state for our block list element.
    block_list_mouse_states: BlockListMouseStates,

    /// All state related to the floating command header ("the snackbar")
    snackbar_header_state: SnackbarHeaderState,

    /// The block index of the block the user has moused down on. This is a
    /// temporary state to determine if a single block has been clicked.
    mouse_down_block_index: Option<BlockIndex>,

    mouse_states: TerminalViewMouseStates,

    /// A sender used to handle messages for whenever the entire terminal view
    /// changes size.  Note that this size contains not just the content element
    /// but also the input.
    resize_tx: Sender<Vector2F>,

    find_link_tx: Sender<FindLinkArg>,

    /// Highlighted link (could be url or file path) on the screen.
    highlighted_link: HighlightedLinkOption,
    open_grid_link_tool_tip: Option<GridHighlightedLink>,

    open_rich_content_link_tool_tip: Option<RichContentLinkTooltipInfo>,

    last_hover_fragment_boundary: Option<WithinModel<FragmentBoundary>>,

    bootstrap_start: Option<Instant>,
    is_login_shell_bootstrapped: bool,
    /// Set when a pending command is submitted to the shell. Cleared on the
    /// next `AfterBlockCompleted`, at which point `Event::PendingCommandCompleted`
    /// is emitted so subscribers know the command has finished.
    awaiting_pending_command_completion: bool,
    /// (i.e. after `PendingCommandCompleted` is emitted). Set by
    /// `pane_tree_from_template_recursive` when a tab config has both
    /// commands and `PaneMode::Agent`.
    slow_bootstrap_banner: ViewHandle<Banner<TerminalAction>>,
    is_slow_bootstrap_banner_open: bool,

    /// The handle to any currently hovered secret. Used to determine whether the
    /// secret gets a special hovered treatment.
    hovered_secret: Option<SecretHandle>,

    /// The details of a currently focused secret tooltip (either grid or rich content).
    open_secret_tool_tip: Option<SecretTooltip>,

    control_master_error_banner: ViewHandle<Banner<TerminalAction>>,
    control_master_error_banner_state: ControlMasterErrorBannerState,

    /// Banner to show if we detect a configuration in the user's rc files that
    /// is incompatible with Warp.
    incompatible_configuration_banner: ViewHandle<Banner<TerminalAction>>,
    is_incompatible_configuration_banner_open: bool,

    /// Non-MacOS banner to ask if the user prefers MacOS bindings
    /// or Emacs-style bindings for `ctrl-a` and `ctrl-e`.
    emacs_bindings_banner: ViewHandle<Banner<TerminalAction>>,
    is_emacs_bindings_banner_open: bool,

    /// Banner shown when an OSC 52 clipboard operation is blocked by the user's setting.
    osc52_clipboard_blocked_banner: ViewHandle<Banner<TerminalAction>>,
    /// Which type of clipboard operation was blocked (if the banner is visible).
    osc52_clipboard_blocked_type: Option<Osc52ClipboardBlockedType>,
    /// Whether the user has permanently dismissed the clipboard blocked banner.
    osc52_clipboard_banner_suppressed: bool,

    pane_configuration: ModelHandle<PaneConfiguration>,
    focus_handle: Option<PaneFocusHandle>,

    sessions: ModelHandle<Sessions>,
    active_block_metadata: Option<BlockMetadata>,

    block_text_selection_start_position: Option<Vector2F>,

    inline_banners_state: InlineBannersState,

    /// Most recent command correction encountered, if any, used for the keyboard shortcut action.
    most_recent_command_correction: Option<Correction>,

    /// Set of block indexes that are bookmarked, including the mouse states for their indicators
    bookmarked_blocks: HashMap<BlockIndex, MouseStateHandle>,

    #[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
    file_link_scanning_join_handle: Option<JoinHandle<()>>,

    last_focus_ts: Option<NaiveDateTime>,
    tips_completed: ModelHandle<TipsCompleted>,

    /// Whether or not this terminal session was ever active.
    was_ever_visible: bool,

    /// The [`EntityId`] for this terminal view.
    view_id: EntityId,

    current_state: TerminalViewStateChange,

    /// Whether we've already emitted a chrome refresh for the active block after it crossed the
    /// long-running threshold. Reset when the active command starts and finishes.
    did_notify_long_running: bool,

    /// This field is an "&&" combination of two other pieces of state:
    ///   1. Whether this View (or one of its children) is the focused View.
    ///   2. Whether this View's window is the active window.
    ///
    /// We need to derive and cache this state on this View in order to correctly implement focus
    /// reporting. Because focus is window-scoped, i.e. warpui does not consider activating a
    /// different window as blurring the focused View in the previously active window, we cannot
    /// simply rely on the warpui::View::on_blur and on_focus methods to report focus-in/out to the
    /// PTY, as those methods will not trigger when changing active windows. The singleton model
    /// [`warpui::windowing::State`] will allow us to subscribe to active window change. So, we can
    /// subscribe to that and have that callback also report focus-in/out. However, that will still
    /// leave cases for potential double-reporting, as a single click can trigger both
    /// [`warpui::View::on_focus`] and emit a [`warpui::windowing::StateEvent`]. This field will
    /// guard against that double- reporting case, though it needs to be kept in sync with the
    /// focused view and active window.
    is_focused_and_active: bool,

    current_prompt: ModelHandle<PromptType>,

    model_event_sender: Option<SyncSender<persistence::ModelEvent>>,

    /// The child views that represent rich content. These can be inserted into the block list with
    /// the `insert_rich_content` helper function.
    rich_content_views: Vec<RichContent>,

    settings_import_block: Option<ViewHandle<SettingsImportView>>,

    /// The type of the subshell that we will bootstrap/"warpify"" on the next [`AfterBlockStarted`]
    /// terminal model event. Will only be `Some` with a [`ShellType`] we can bootstrap.
    pending_auto_bootstrap_shell_type: Option<ShellType>,
    env_vars: Vec<EnvVar>,

    show_snackbar: bool,
    hover_near_snackbar_area: bool,

    pending_env_var_collection: Option<EnvVarCollection>,

    /// The ID of the containing window.
    window_id: WindowId,

    /// The position ID of the currently rendered terminal "content" element; either the blocklist
    /// element or the alt screen element depending on which is currently rendered.
    content_element_position_id: String,

    /// The position ID of the terminal input.
    ///
    /// This is cached, as opposed to read from `Input` on demand, to prevent otherwise-possible
    /// circular view references that could occur because `TerminalView` implements the `MenuPositioningProvider`
    /// that's used as a dependency of certain `Input` methods. `MenuPositioningProvider`
    /// internally relies on reading the last-frame position of `Input`, which would otherwise
    /// require reading the position ID directly from `Input` and cause a circular ref panic.
    input_position_id: String,

    /// A handle for the [`Hoverable`] that we render the [`Input`] view in.
    ///
    /// While the [`Input`] itself might internally render with a [`Hoverable`]
    /// around it, we use a dedicated [`Hoverable`] at the [`TerminalView`] level because
    /// 1. the [`Input`] implementation might change, and
    /// 2. we have specific hover behaviour at the [`TerminalView`] level
    ///    (e.g. a hover-out delay)
    input_hoverable_handle: MouseStateHandle,

    find_model: ModelHandle<TerminalFindModel>,

    warpify_state: WarpifyState,

    /// The keystroke bound to canceling a command.
    ///
    /// This is cached on the view because the UI framework APIs needed to lookup keystroke for an
    /// action only exist on `AppContext`, which is not accessible at render time. Sigh.
    cancel_command_keystroke: Option<Keystroke>,

    /// Whether the terminal view is currently a drop target for a file. If it is, we render an overlay.
    is_file_drop_target: bool,

    /// Whether this terminal pane is taking care of uploading a file over SSH.
    is_ssh_file_uploader: bool,

    /// The file uploads initiated in this terminal pane.
    ssh_file_upload: ViewHandle<FileUpload>,

    /// The type of the shell that this terminal pane is running, derived and
    /// cached on the view from [`ShellLaunchdata`]. Used to render an indicator
    /// in the tab bar.
    shell_indicator_type: Option<ShellIndicatorType>,

    /// Used to describe the active shell to the user.
    shell_detail: Option<String>,

    /// Position ID for this view.
    position_id: String,

    /// Position ID for the active terminal cursor.
    cursor_position_id: String,

    #[cfg_attr(not(test), allow(unused))]
    active_session: ModelHandle<ActiveSession>,

    pty_spawn_failed: bool,

    model_events_handle: ModelHandle<ModelEventDispatcher>,

    /// Per-repo git status model for the current repository, if any.
    #[cfg(feature = "local_fs")]
    git_repo_status: Option<ModelHandle<GitRepoStatusModel>>,

    /// Deferred code review open request, stashed when [`GitDeltaPreference::OnlyDirty`] is
    /// requested but git status metadata has not loaded yet. Consumed in
    /// [`Self::handle_git_repo_status_event`].
    #[cfg(feature = "local_fs")]
    deferred_code_review_open: Option<DeferredCodeReviewOpen>,

    /// A list of callbacks to run on the next

    /// Path to the current repository, or None if not currently in a repo.
    current_repo_path: Option<PathBuf>,

    terminal_title: String,

    ignore_next_set_title_event: bool,

    use_agent_footer: ViewHandle<UseAgentToolbar>,

    /// Weak handle to the [`PaneStack`] this view is part of, allowing push/pop operations.
    pane_stack: Option<WeakModelHandle<crate::pane_group::pane::PaneStack<Self>>>,

    /// Active /init flow model, if any. Cleared when cancelled or completed.

    /// Whether we're waiting for the result of an AWS CLI login command.
    /// Used to detect "command not found" errors when AWS CLI isn't installed.
    /// TODO: In the future, when we support GCP/Azure cloud CLIs, this should be
    /// converted to `pending_cloud_cli_login: Option<CloudProvider>` where CloudProvider
    /// is an enum with variants like Aws, Gcp, Azure.
    /// `true` if this view explicitly requested a PTY shutdown.
    ///
    /// Once set, this remains true for the rest of the view's lifecycle and
    /// suppresses `AgentExitedShellProcess` telemetry so manual shutdown paths
    /// (tab close, update relaunch, etc.) are not attributed to agent commands.
    manual_pty_shutdown_requested: bool,

    /// Tracks the view ID of an inserted pending user query block, if any.
    /// Used to remove the block when summarization completes or is cancelled.

    /// (via `/queue`, `/compact-and`, etc.) does not wipe unrelated callbacks.

    /// Per-session PTY recorder for writing PTY bytes to a file.
    pty_recorder: ModelHandle<PtyRecorder>,
}

/// Parameters stashed when a code review pane open is requested with
/// [`GitDeltaPreference::OnlyDirty`] but git status metadata is not yet available.
/// Consumed once the per-repo [`GitRepoStatusModel`] delivers its first update.
#[cfg(feature = "local_fs")]
struct DeferredCodeReviewOpen {
    git_delta_preference: GitDeltaPreference,
    focus_new_pane: bool,
}

#[derive(Copy, Clone, Serialize)]
pub enum BlockSelectionDelta {
    // User first selects a block, or selects a block with click
    New,
    // User already has block selected, and selects previous block
    Previous,
    // User already has block selected, and selects next block
    Next,
}

#[derive(Copy, Clone, Serialize)]
pub struct BlockSelectionDetails {
    cardinality: BlockSelectionCardinality,
    delta: BlockSelectionDelta,
    is_cmd_down: bool,
    is_shift_down: bool,
}

impl TerminalView {
    /// Returns the path to the current repository, if any.
    pub fn current_repo_path(&self) -> Option<&PathBuf> {
        self.current_repo_path.as_ref()
    }

    /// Create a SyncEvent for other terminals to use based on
    /// the state of this terminal. If this terminal view has an active input
    /// editor, other terminals should match those contents.
    /// Otherwise, they should just start syncing.
    pub fn create_sync_event_based_on_terminal_state(&self, app_ctx: &AppContext) -> SyncEvent {
        if !matches!(
            self.model.lock().terminal_input_state(),
            TerminalInputState::InputEditor,
        ) {
            return SyncEvent {
                source_view_id: self.view_id,
                data: SyncInputType::StartSyncing,
            };
        }

        let input_buffer = self.input().as_ref(app_ctx).buffer_text(app_ctx);

        SyncEvent {
            source_view_id: self.view_id,
            data: SyncInputType::InputEditorContentsChanged {
                contents: Arc::new(input_buffer),
            },
        }
    }

    /// Receives a SyncEvent and performs actions dictated by that event
    /// on this terminal view.
    pub fn receive_sync_input_event(&mut self, event: &SyncEvent, ctx: &mut ViewContext<Self>) {
        // The source terminal shouldn't process it's own data sync event.
        if event.source_view_id == self.view_id {
            return;
        }

        let terminal_input_state = self.model.lock().terminal_input_state();

        match &event.data {
            SyncInputType::InputEditorContentsChanged { contents } => {
                if matches!(
                    terminal_input_state,
                    TerminalInputState::InputEditor | TerminalInputState::NotBootstrapped
                ) {
                    self.input.update(ctx, |input, ctx| {
                        input.send_input_buffer_to_terminal_editor(Arc::clone(contents), ctx);
                    })
                }
            }
            SyncInputType::NonEditorTyped { chars: typed_chars } => {
                if matches!(
                    terminal_input_state,
                    TerminalInputState::LongRunningCommand | TerminalInputState::AltScreen,
                ) {
                    self.write_to_pty_for_syncing_long_running_commands(typed_chars.to_vec(), ctx);
                }
            }
            SyncInputType::RanCommand => {
                if terminal_input_state == TerminalInputState::InputEditor {
                    self.input.update(ctx, |input, ctx| {
                        input.run_command_in_synced_terminal_input(ctx);
                    });
                }
            }
            // For start and stop syncing we only need to change the input box
            // show/hide logic and that's handled before this match statement.
            SyncInputType::StartSyncing => (),
            SyncInputType::StopSyncing => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resources: TerminalViewResources,
        wakeups_rx: Receiver<()>,
        model_events_handle: ModelHandle<ModelEventDispatcher>,
        model: Arc<FairMutex<TerminalModel>>,
        sessions: ModelHandle<Sessions>,
        size_info: SizeInfo,
        colors: List,
        model_event_sender: Option<SyncSender<persistence::ModelEvent>>,
        current_prompt: ModelHandle<PromptType>,
        inactive_pty_reads_rx: Option<async_broadcast::InactiveReceiver<Arc<Vec<u8>>>>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let terminal_view_id = ctx.view_id();
        let active_session = ctx.add_model(|ctx| {
            ActiveSession::new(sessions.clone(), model_events_handle.clone(), ctx)
        });

        let find_model = ctx.add_model(|_| TerminalFindModel::new(model.clone()));

        ctx.subscribe_to_model(
            &TerminalSettings::handle(ctx),
            |me, terminal_settings, event, ctx| match event {
                TerminalSettingsChangedEvent::MaximumGridSize { .. } => {
                    let mut model = me.model.lock();
                    model.update_max_grid_size(
                        *terminal_settings.as_ref(ctx).maximum_grid_size.value(),
                    );
                }
                TerminalSettingsChangedEvent::Spacing { .. } => {
                    let appearance = Appearance::as_ref(ctx);
                    let terminal_spacing = terminal_settings
                        .as_ref(ctx)
                        .terminal_spacing(appearance.line_height_ratio(), ctx);
                    me.model.lock().update_blockheight_items(
                        terminal_spacing.block_padding,
                        terminal_spacing.subshell_separator_height,
                    );
                    ctx.notify();
                }
                TerminalSettingsChangedEvent::AltScreenPadding { .. } => {
                    if me.model.lock().is_alt_screen_active() {
                        me.refresh_size(ctx);
                    }
                }
                _ => {}
            },
        );

        ctx.subscribe_to_model(&PaneSettings::handle(ctx), |_, _, event, ctx| {
            if matches!(
                event,
                PaneSettingsChangedEvent::ShouldDimInactivePanes { .. }
            ) {
                ctx.notify();
            }
        });

        ctx.subscribe_to_model(
            &Appearance::handle(ctx),
            move |me, _, event, ctx| match event {
                AppearanceEvent::ThemeChanged => {
                    me.handle_theme_change(ctx);
                }
                AppearanceEvent::MonospaceFontSizeChanged { .. }
                | AppearanceEvent::LineHeightRatioChanged { .. }
                | AppearanceEvent::MonospaceFontFamilyChanged { .. }
                | AppearanceEvent::MonospaceFontWeightChanged { .. }
                | AppearanceEvent::UiFontFamilyChanged { .. } => {
                    me.refresh_size(ctx);
                }
            },
        );

        ctx.subscribe_to_model(&FontSettings::handle(ctx), |_, _, event, ctx| {
            if matches!(
                event,
                FontSettingsChangedEvent::EnforceMinimumContrast { .. }
            ) {
                ctx.notify();
            }
        });

        ctx.subscribe_to_model(&GeneralSettings::handle(ctx), move |_, _, _, ctx| {
            ctx.notify();
        });

        ctx.subscribe_to_model(&InputModeSettings::handle(ctx), |me, _, event, ctx| {
            if matches!(event, InputModeSettingsChangedEvent::InputModeState { .. }) {
                let current_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
                if current_mode == InputMode::Waterfall {
                    // Run the resize logic when switching into Waterfall to potentially update the gap size.
                    me.refresh_size(ctx);
                }

                me.model
                    .lock()
                    .block_list_mut()
                    .set_is_inverted(current_mode.is_inverted_blocklist());

                ctx.notify();
            }
        });

        ctx.subscribe_to_model(
            &DebugSettings::handle(ctx),
            |me, debug_settings, event, ctx| {
                if let DebugSettingsChangedEvent::ShowMemoryStats { .. } = event {
                    me.model.lock().block_list_mut().set_show_memory_stats(
                        debug_settings.as_ref(ctx).should_show_memory_stats(),
                    );
                }
            },
        );

        let (resize_tx, resize_rx) = async_channel::unbounded();
        let (find_link_tx, find_link_rx) = async_channel::unbounded();
        ctx.subscribe_to_model(&model_events_handle, |me, _, event, ctx| {
            me.handle_terminal_event(event, ctx);
        });

        let _ = ctx.spawn_stream_local(
            throttle(WAKEUP_THROTTLE_PERIOD, wakeups_rx),
            Self::handle_terminal_wakeup,
            |_, _| {}, /* on_done */
        );

        let _ = ctx.spawn_stream_local(
            debounce(DEBOUNCE_PERIOD, find_link_rx),
            Self::handle_find_link,
            |_, _| {}, /* on_done */
        );

        let _ = ctx.spawn_stream_local(resize_rx, Self::after_terminal_view_layout, |_, _| {});

        let menu_positioning_provider = Arc::new(TerminalViewMenuPositioningProvider {
            parent: ctx.handle(),
        });

        let terminal_content_element_position_id =
            format!("terminal_content_element_{}", ctx.view_id());

        let input: ViewHandle<Input> = ctx.add_typed_action_view(|ctx| {
            Input::new(
                model.clone(),
                resources.tips_completed.clone(),
                sessions.clone(),
                size_info,
                menu_positioning_provider,
                current_prompt.clone(),
                terminal_view_id,
                None, // current_repo_path - will be set when CWD is determined
                model_events_handle.clone(),
                ctx,
            )
        });

        let suggestions_mode_model = input.as_ref(ctx).suggestions_mode_model().clone();
        ctx.subscribe_to_model(&suggestions_mode_model, |_, _, _, ctx| {
            ctx.notify();
        });

        let input_position_id = input.read(ctx, |input, _| input.save_position_id());
        ctx.subscribe_to_view(&input, move |me, _, event, ctx| {
            me.handle_input_event(event, ctx);
        });

        ctx.subscribe_to_model(&CLIAgentSessionsModel::handle(ctx), |me, _, event, ctx| {
            if let CLIAgentSessionsModelEvent::Ended {
                terminal_view_id, ..
            } = event
            {
                let _ = terminal_view_id;
            }
            me.handle_cli_agent_sessions_event(event, ctx)
        });
        let find_bar = ctx.add_typed_action_view(|ctx| Find::new(find_model.clone(), ctx));
        ctx.subscribe_to_view(&find_bar, move |me, _, event, ctx| {
            me.handle_find_event(event, ctx);
        });

        let block_filter_editor = ctx.add_typed_action_view(BlockFilterEditor::new);
        ctx.subscribe_to_view(&block_filter_editor, move |me, _, event, ctx| {
            me.handle_block_filter_event(event, ctx);
        });

        let context_menu = ctx.add_typed_action_view(|_| {
            Menu::new()
                .prevent_interaction_with_other_elements()
                .with_drop_shadow()
        });
        ctx.subscribe_to_view(&context_menu, move |me, _, event, ctx| {
            me.handle_menu_event(event, ctx);
        });

        let slow_bootstrap_banner = ctx.add_typed_action_view(|_| {
            Banner::<TerminalAction>::new_with_buttons(
                BannerTextContent::formatted_text(vec![
                    FormattedTextFragment::plain_text(
                        "Seems like your shell is taking a while to start...  ",
                    ),
                    FormattedTextFragment::hyperlink("More info", KNOWN_ISSUES_URL),
                ]),
                vec![BannerTextButton::new(
                    "Show initialization block".to_string(),
                    Rc::new(|event_ctx, _ctx, _position| {
                        event_ctx.dispatch_typed_action(BannerAction::<TerminalAction>::Action(
                            TerminalAction::ShowInitializationBlock,
                        ));
                    }),
                )],
                true,
            )
        });
        ctx.subscribe_to_view(&slow_bootstrap_banner, |me, _, event, ctx| {
            me.handle_slow_bootstrap_banner_event(event, ctx);
        });

        ctx.subscribe_to_model(&sessions, |me, _, event, ctx| {
            me.handle_sessions_event(event.clone(), ctx);
        });

        let control_master_error_banner = ctx.add_typed_action_view(|_| {
            Banner::new(BannerTextContent::formatted_text(vec![
                FormattedTextFragment::plain_text("Seems like your completions are not working ("),
                FormattedTextFragment::hyperlink("more info", CONTROLMASTER_ISSUES_URL),
                FormattedTextFragment::plain_text("). Enabling tmux warpification in "),
                FormattedTextFragment::hyperlink_action(
                    "settings",
                    TerminalAction::ShowWarpifySettings,
                ),
                FormattedTextFragment::plain_text(" may resolve this issue."),
            ]))
        });

        ctx.subscribe_to_view(&control_master_error_banner, |me, _, event, ctx| {
            me.handle_controlmaster_error_banner_event(event, ctx);
        });

        let incompatible_configuration_banner = ctx.add_typed_action_view(|_| {
            Banner::new(BannerTextContent::formatted_text(vec![
                FormattedTextFragment::plain_text(
                    "Your shell configuration is incompatible with Warper...  ",
                ),
                FormattedTextFragment::hyperlink("More info", KNOWN_ISSUES_URL),
            ]))
        });

        ctx.subscribe_to_view(&incompatible_configuration_banner, |me, _, event, ctx| {
            me.handle_incompatible_configuration_banner_event(event, ctx);
        });

        let emacs_bindings_banner = ctx.add_typed_action_view(|_| {
            Banner::new_with_buttons(
                BannerTextContent::formatted_text(vec![
                    FormattedTextFragment::plain_text("Did you intend "),
                    FormattedTextFragment::inline_code("ctrl-a"),
                    FormattedTextFragment::plain_text("/"),
                    FormattedTextFragment::inline_code("ctrl-e"),
                    FormattedTextFragment::plain_text(" to move the cursor?"),
                ]),
                // Here, we use DismissalType::Temporary and DismissalType::Permanent variants
                // as stand-ins for changing bindings vs. leaving them as-is.
                // TODO(Linear PLAT-512): update Banner to support generic event type.
                vec![
                    BannerTextButton::new(
                        String::from("Yes, use Emacs-style bindings"),
                        Rc::new(|event_ctx, _app_ctx, _| {
                            event_ctx.dispatch_typed_action(
                                BannerAction::<TerminalAction>::Dismiss(DismissalType::Temporary),
                            );
                        }),
                    ),
                    BannerTextButton::new(
                        String::from("No, keep IDE bindings"),
                        Rc::new(|event_ctx, _app_ctx, _| {
                            event_ctx.dispatch_typed_action(
                                BannerAction::<TerminalAction>::Dismiss(DismissalType::Permanent),
                            );
                        }),
                    ),
                ],
                /* with_close_button */ false,
            )
            .with_icon(icons::Icon::HelpCircle)
        });

        if OperatingSystem::get().is_linux() {
            ctx.subscribe_to_view(&emacs_bindings_banner, |me, _, event, ctx| {
                me.handle_emacs_bindings_banner_clicked(event, ctx);
            });
        }

        let osc52_clipboard_blocked_banner = ctx.add_typed_action_view(|_| {
            Banner::<TerminalAction>::new_with_buttons(
                BannerTextContent::plain_text(
                    "A terminal program tried to access your clipboard. This is disabled by default for security reasons.",
                ),
                vec![
                    BannerTextButton::new(
                        "Allow".to_string(),
                        Rc::new(|event_ctx, _ctx, _position| {
                            event_ctx.dispatch_typed_action(BannerAction::<TerminalAction>::Action(
                                TerminalAction::Osc52AllowBlockedClipboardOperation,
                            ));
                        }),
                    ),
                    BannerTextButton::new(
                        "Don't show again".to_string(),
                        Rc::new(|event_ctx, _ctx, _position| {
                            event_ctx.dispatch_typed_action(
                                BannerAction::<TerminalAction>::Dismiss(DismissalType::Permanent),
                            );
                        }),
                    ),
                ],
                true,
            )
            .with_icon(icons::Icon::AlertTriangle)
        });
        ctx.subscribe_to_view(&osc52_clipboard_blocked_banner, |me, _, event, ctx| {
            me.handle_osc52_clipboard_banner_event(event, ctx);
        });

        let osc52_clipboard_banner_suppressed = ctx
            .private_user_preferences()
            .read_value(OSC52_CLIPBOARD_BANNER_SUPPRESSED_KEY)
            .ok()
            .flatten()
            .is_some_and(|v| v == "true");

        let windowing_state_handle = WindowManager::handle(ctx);
        ctx.subscribe_to_model(&windowing_state_handle, |me, _handle, evt, ctx| match evt {
            windowing::StateEvent::ValueChanged { current, previous } => {
                me.handle_windowing_state_update((current, previous), ctx);
            }
        });

        let ligature_handle = LigatureSettings::handle(ctx);
        ctx.subscribe_to_model(&ligature_handle, |_, _, _, ctx| ctx.notify());

        let block_visibility_settings_handle = BlockVisibilitySettings::handle(ctx);
        ctx.subscribe_to_model(
            &block_visibility_settings_handle,
            |me, block_visibility_settings_handle, event, ctx| match event {
                BlockVisibilitySettingsChangedEvent::ShouldShowBootstrapBlock { .. } => {
                    let should_show_bootstrap_block = *block_visibility_settings_handle
                        .as_ref(ctx)
                        .should_show_bootstrap_block
                        .value();
                    let mut model = me.model.lock();
                    model
                        .block_list_mut()
                        .set_show_bootstrap_block(should_show_bootstrap_block);
                    ctx.notify();
                }
                BlockVisibilitySettingsChangedEvent::ShouldShowInBandCommandBlocks { .. } => {
                    let should_show_in_band_command_blocks = *block_visibility_settings_handle
                        .as_ref(ctx)
                        .should_show_in_band_command_blocks
                        .value();
                    let mut model = me.model.lock();
                    model
                        .block_list_mut()
                        .set_show_in_band_command_blocks(should_show_in_band_command_blocks);
                    ctx.notify();
                }
                BlockVisibilitySettingsChangedEvent::ShouldShowSSHBlock { .. } => {}
            },
        );

        let block_list_settings_handle = BlockListSettings::handle(ctx);
        ctx.subscribe_to_model(&block_list_settings_handle, |_, _, evt, ctx| match evt {
            BlockListSettingsChangedEvent::ShowJumpToBottomOfBlockButton { .. } => ctx.notify(),
            BlockListSettingsChangedEvent::SnackbarEnabled { .. } => ctx.notify(),
            BlockListSettingsChangedEvent::ShowBlockDividers { .. } => ctx.notify(),
        });

        ctx.subscribe_to_model(&SessionSettings::handle(ctx), move |me, _, evt, ctx| {
            me.handle_session_settings_event(evt, ctx);
        });

        // Re-evaluate git status subscription when the prompt configuration
        // changes (e.g. chips added/removed, input type toggled).
        ctx.subscribe_to_model(&Prompt::handle(ctx), |me, _, _, ctx| {
            me.update_git_status_subscription(ctx);
        });

        ctx.subscribe_to_model(&AltScreenReporting::handle(ctx), move |me, _, evt, ctx| {
            me.handle_reporting_settings_event(evt, ctx);
        });

        let initial_title = model.lock().shell_launch_state().display_name().to_string();

        let pane_configuration = ctx.add_model(|_| PaneConfiguration::new(initial_title));

        ctx.observe(
            &WindowActiveSession::handle(ctx),
            |me, active_session, ctx| {
                let active_session = active_session.as_ref(ctx);
                let state =
                    if active_session.terminal_view_id(ctx.window_id()) == Some(ctx.view_id()) {
                        ActiveSessionState::Active
                    } else {
                        ActiveSessionState::Inactive
                    };
                me.set_active_session_state(state, ctx);
            },
        );
        ctx.subscribe_to_model(&KeybindingChangedNotifier::handle(ctx), |me, _, _, ctx| {
            me.cancel_command_keystroke =
                keybinding_name_to_keystroke(CANCEL_COMMAND_KEYBINDING, ctx);

            me.refresh_pane_header(ctx);
            ctx.notify();
        });

        let ssh_file_upload = ctx.add_typed_action_view(|_| FileUpload::new());

        if FeatureFlag::SshDragAndDrop.is_enabled() {
            ctx.subscribe_to_view(&ssh_file_upload, |_terminal, _file_upload, event, ctx| {
                // Pass the file upload events up so they can be processed by the pane group.
                match event {
                    FileUploadEvent::CopyFileToRemote { command, upload_id } => {
                        ctx.emit(Event::CopyFileToRemote {
                            command: command.clone(),
                            upload_id: *upload_id,
                        });
                    }
                    FileUploadEvent::OpenUploadSession(upload_id) => {
                        ctx.emit(Event::OpenFileUploadSession(*upload_id));
                    }
                    FileUploadEvent::TerminateUploadSession(upload_id) => {
                        ctx.emit(Event::TerminateFileUploadSession(*upload_id));
                    }
                }
            });
        }

        // Here we intialize the block list mouse states for block zero.
        // Afterwards, we initialize all block list mouse states for a block when the
        // previous block sends a `BlockCompleted` event.
        let mut block_list_mouse_states = BlockListMouseStates::default();
        block_list_mouse_states
            .label_mouse_states
            .entry(BlockIndex::zero())
            .or_default();
        block_list_mouse_states
            .bookmark_mouse_states
            .entry(BlockIndex::zero())
            .or_default();
        block_list_mouse_states
            .filter_mouse_states
            .entry(BlockIndex::zero())
            .or_default();

        ctx.subscribe_to_model(&AssetCache::handle(ctx), |me, _, event, _| match event {
            AssetCacheEvent::ImagesEvicted { image_ids } => {
                let mut terminal_model = me.model.lock();
                for &image_id in image_ids {
                    terminal_model.remove_image_id_to_metadata_entry(image_id);
                }
            }
        });

        let terminal_view_id = ctx.view_id();
        let agent_input_footer = input.as_ref(ctx).agent_input_footer().clone();
        let use_agent_button_bar = ctx.add_view(|ctx| {
            UseAgentToolbar::new(
                terminal_view_id,
                model.clone(),
                &model_events_handle,
                agent_input_footer.clone(),
                ctx,
            )
        });
        let window_id = ctx.window_id();
        let mut terminal_view = Self {
            model,
            input,
            view_handle: ctx.handle(),
            size_info: size_info.into(),
            snackbar_header_state: Default::default(),
            colors,
            scroll_position: ScrollState::new(ScrollPosition::FollowsBottomOfMostRecentBlock),
            blocklist_vertical_scroll_state: Default::default(),
            alt_screen_vertical_scroll_state: Default::default(),
            alt_screen_scroll_top: Lines::zero(),
            horizontal_clipped_scroll_state: Default::default(),
            is_selecting: false,
            context_menu_state: None,
            context_menu,
            hovered_secret: None,
            open_secret_tool_tip: None,
            hovered_block_index: None,
            selected_blocks: Default::default(),
            block_list_mouse_states,
            any_session_contains_remote_blocks: false,
            any_session_contains_restored_remote_blocks: false,
            mouse_down_block_index: None,
            mouse_states: Default::default(),
            open_grid_link_tool_tip: None,
            open_rich_content_link_tool_tip: None,
            find_bar,
            resize_tx,
            find_link_tx,
            highlighted_link: HighlightedLinkOption::default(),
            last_hover_fragment_boundary: None,
            bootstrap_start: None,
            is_login_shell_bootstrapped: false,
            awaiting_pending_command_completion: false,
            slow_bootstrap_banner,
            is_slow_bootstrap_banner_open: false,
            incompatible_configuration_banner,
            is_incompatible_configuration_banner_open: false,
            emacs_bindings_banner,
            is_emacs_bindings_banner_open: false,
            control_master_error_banner,
            control_master_error_banner_state: Default::default(),
            osc52_clipboard_blocked_banner,
            osc52_clipboard_blocked_type: None,
            osc52_clipboard_banner_suppressed,
            pane_configuration,
            focus_handle: None,
            sessions,
            active_block_metadata: None,
            block_text_selection_start_position: None,
            inline_banners_state: Default::default(),
            bookmarked_blocks: Default::default(),
            file_link_scanning_join_handle: None,
            last_focus_ts: None,
            tips_completed: resources.tips_completed.clone(),
            was_ever_visible: false,
            view_id: ctx.view_id(),
            current_state: TerminalViewStateChange::default(),
            did_notify_long_running: false,
            is_focused_and_active: true,
            current_prompt,
            model_event_sender,
            block_filter_editor,
            active_filter_editor_block_index: None,
            rich_content_views: Vec::new(),
            settings_import_block: None,
            pending_auto_bootstrap_shell_type: None,
            pending_env_var_collection: None,
            env_vars: Vec::new(),
            show_snackbar: true,
            hover_near_snackbar_area: false,
            window_id,
            content_element_position_id: terminal_content_element_position_id,
            input_position_id,
            input_hoverable_handle: Default::default(),
            find_model,
            warpify_state: Default::default(),
            cancel_command_keystroke: keybinding_name_to_keystroke(CANCEL_COMMAND_KEYBINDING, ctx),
            is_file_drop_target: false,
            is_ssh_file_uploader: false,
            ssh_file_upload,
            most_recent_command_correction: None,
            shell_indicator_type: None,
            shell_detail: None,
            position_id: format!("terminal_view_{}", ctx.view_id()),
            cursor_position_id: format!("terminal_view:cursor_{}", ctx.view_id()),
            active_session,
            pty_spawn_failed: false,
            model_events_handle,
            #[cfg(feature = "local_fs")]
            git_repo_status: None,
            #[cfg(feature = "local_fs")]
            deferred_code_review_open: None,
            current_repo_path: None,
            terminal_title: Default::default(),
            ignore_next_set_title_event: false,
            use_agent_footer: use_agent_button_bar,
            manual_pty_shutdown_requested: false,
            pane_stack: None,
            pty_recorder: ctx
                .add_model(|ctx| PtyRecorder::new(inactive_pty_reads_rx, window_id, ctx)),
        };
        terminal_view.register_subscriptions_for_use_agent_footer(ctx);

        terminal_view.any_session_contains_restored_remote_blocks =
            terminal_view.contains_restored_remote_blocks();

        terminal_view
    }

    #[cfg(feature = "local_fs")]
    fn handle_git_repo_status_event(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(deferred) = self.deferred_code_review_open.take() {
            self.toggle_code_review_pane(
                deferred.git_delta_preference,
                CodeReviewPaneEntrypoint::Other,
                None,
                deferred.focus_new_pane,
                ctx,
            );
        }
        self.refresh_pane_header(ctx);
        ctx.emit(Event::TerminalViewStateChanged);
        ctx.notify();
    }

    /// Drop the per-repo git status subscription without clearing the input's
    /// repo path. Use this when unsubscribing because the subscription is no
    /// longer needed (e.g. the git chip was removed) but the user is still in
    /// the same repository.
    #[cfg(feature = "local_fs")]
    fn clear_git_repo_status_subscription(&mut self, ctx: &mut ViewContext<Self>) {
        self.git_repo_status = None;
        self.deferred_code_review_open = None;

        self.current_prompt.update(ctx, |prompt_type, ctx| {
            if let PromptType::Dynamic { prompt } = prompt_type {
                prompt.update(ctx, |current_prompt, ctx| {
                    current_prompt.set_git_repo_status(None, ctx);
                });
            }
        });
    }

    /// Fully clear the per-repo git status handle, including the input's repo
    /// path. Use this when navigating out of a git repository.
    #[cfg(feature = "local_fs")]
    fn clear_git_repo_status(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_git_repo_status_subscription(ctx);
        self.input.update(ctx, |input, ctx| {
            input.update_repo_path(None, ctx);
        });
    }

    /// Helper to read metadata from the per-repo sub-model.
    #[cfg(feature = "local_fs")]
    fn git_status_metadata<'a>(&'a self, ctx: &'a AppContext) -> Option<&'a GitStatusMetadata> {
        self.git_repo_status
            .as_ref()
            .and_then(|h| h.as_ref(ctx).metadata())
    }

    /// Subscribe to git status when the Warp prompt includes git statistics.
    #[cfg(feature = "local_fs")]
    fn should_subscribe_to_git_status(&self, ctx: &AppContext) -> bool {
        // Terminal prompt path: the Warp prompt is active when honor_ps1 is
        // off, or when UDI overrides PS1. GitDiffStats must also be in the
        // configured chip list.
        let is_using_warp_prompt = !*SessionSettings::as_ref(ctx).honor_ps1
            || InputSettings::as_ref(ctx).is_universal_developer_input_enabled(ctx);
        is_using_warp_prompt
            && Prompt::as_ref(ctx)
                .chip_kinds()
                .contains(&ContextChipKind::GitDiffStats)
    }

    /// No-op when the `local_fs` feature is disabled – git status is not
    /// available so there is nothing to subscribe to.
    #[cfg(not(feature = "local_fs"))]
    fn update_git_status_subscription(&mut self, _ctx: &mut ViewContext<Self>) {}

    /// Re-evaluate whether this terminal view should be subscribed to git
    /// status updates and subscribe/unsubscribe accordingly.
    #[cfg(feature = "local_fs")]
    fn update_git_status_subscription(&mut self, ctx: &mut ViewContext<Self>) {
        let should_subscribe = self.should_subscribe_to_git_status(ctx);
        if should_subscribe {
            // Subscribe if we have a repo path but no active subscription.
            if self.git_repo_status.is_none() {
                if let Some(repo_path) = self.current_repo_path.clone() {
                    let result = GitStatusUpdateModel::handle(ctx)
                        .update(ctx, |model, ctx| model.subscribe(&repo_path, ctx));
                    match result {
                        Ok(handle) => {
                            ctx.subscribe_to_model(&handle, |me, _, _, ctx| {
                                me.handle_git_repo_status_event(ctx);
                            });
                            let weak_for_prompt = handle.downgrade();
                            self.git_repo_status = Some(handle);
                            self.current_prompt.update(ctx, |prompt_type, ctx| {
                                if let PromptType::Dynamic { prompt } = prompt_type {
                                    prompt.update(ctx, |current_prompt, ctx| {
                                        current_prompt
                                            .set_git_repo_status(Some(weak_for_prompt), ctx);
                                    });
                                }
                            });
                        }
                        Err(err) => {
                            log::warn!("GitStatusUpdateModel subscribe failed: {err}");
                        }
                    }
                }
            }
        } else if self.git_repo_status.is_some() {
            self.clear_git_repo_status_subscription(ctx);
        }
    }

    /// Returns true if the window is wide enough to auto-open side panels.
    pub fn can_auto_open_panel(&self) -> bool {
        self.size_info.pane_width_px().as_f32() > MINIMUM_WIDTH_TO_AUTO_OPEN_PANE
    }

    fn toggle_or_open_code_review_pane(
        &mut self,
        delta_pref: GitDeltaPreference,
        entrypoint: CodeReviewPaneEntrypoint,
        cli_agent: Option<super::CLIAgent>,
        focus_new_pane: bool,
        event_constructor: impl Fn(CodeReviewPanelArg) -> Event,
        ctx: &mut ViewContext<Self>,
    ) {
        let arg = CodeReviewPanelArg {
            repo_path: self.current_repo_path.clone(),
            terminal_view: self.view_handle.clone(),
            entrypoint,
            focus_new_pane,
            cli_agent,
        };

        match delta_pref {
            GitDeltaPreference::Always => {
                ctx.emit(event_constructor(arg));
            }
            GitDeltaPreference::OnlyDirty => {
                // Check if repo has uncommitted changes via the per-repo sub-model.
                #[cfg(feature = "local_fs")]
                {
                    let is_dirty = self
                        .git_status_metadata(ctx)
                        .map(|m| !m.stats_against_head.has_no_changes());
                    match is_dirty {
                        Some(true) => ctx.emit(event_constructor(arg)),
                        // Metadata not loaded yet — defer until the next
                        // git repo status update delivers it.
                        None => {
                            self.deferred_code_review_open = Some(DeferredCodeReviewOpen {
                                git_delta_preference: delta_pref,
                                focus_new_pane,
                            });
                        }
                        Some(false) => {}
                    }
                }
            }
        }
    }

    pub fn toggle_code_review_pane(
        &mut self,
        delta_pref: GitDeltaPreference,
        entrypoint: CodeReviewPaneEntrypoint,
        cli_agent: Option<super::CLIAgent>,
        focus_new_pane: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        self.toggle_or_open_code_review_pane(
            delta_pref,
            entrypoint,
            cli_agent,
            focus_new_pane,
            Event::ToggleCodeReviewPane,
            ctx,
        )
    }

    pub fn open_code_review_pane(
        &mut self,
        delta_pref: GitDeltaPreference,
        entrypoint: CodeReviewPaneEntrypoint,
        cli_agent: Option<super::CLIAgent>,
        focus_new_pane: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        self.toggle_or_open_code_review_pane(
            delta_pref,
            entrypoint,
            cli_agent,
            focus_new_pane,
            Event::OpenCodeReviewPane,
            ctx,
        )
    }

    fn handle_windowing_state_update(
        &mut self,
        (current, previous): (&windowing::State, &windowing::State),
        ctx: &mut ViewContext<Self>,
    ) {
        let window_changed = previous.active_window != current.active_window;
        let is_active_window_current_window = Some(ctx.window_id()) == current.active_window;

        if window_changed {
            if let Some(focus_out_window_id) = previous.active_window {
                if focus_out_window_id == ctx.window_id() && ctx.is_self_or_child_focused() {
                    self.maybe_report_focus_out(ctx);
                }
            }

            if let Some(focus_in_window_id) = current.active_window {
                if focus_in_window_id == ctx.window_id() && ctx.is_self_or_child_focused() {
                    self.maybe_report_focus_in(ctx);
                }
            }
        }

        // When we change windows, we need to update the timestamp of the newly focused terminal view.
        if window_changed && is_active_window_current_window && ctx.is_self_or_child_focused() {
            self.last_focus_ts = Some(chrono::Local::now().naive_local());
        }
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn sessions<'a, A: warpui::ModelAsRef>(&self, ctx: &'a A) -> &'a Sessions {
        self.sessions.as_ref(ctx)
    }

    #[cfg(test)]
    pub fn model_event_dispatcher(&self) -> &ModelHandle<ModelEventDispatcher> {
        &self.model_events_handle
    }

    pub fn sessions_model(&self) -> &ModelHandle<Sessions> {
        &self.sessions
    }

    /// Returns `None` for local sessions, `Some("user@hostname")` for remote.
    /// Used to key per-host plugin install failure tracking.
    fn active_session_remote_host<C: ModelAsRef>(&self, ctx: &C) -> Option<String> {
        self.active_block_session_id().and_then(|session_id| {
            let session = self.sessions.as_ref(ctx).get(session_id)?;
            if session.is_local() {
                None
            } else {
                Some(format!("{}@{}", session.user(), session.hostname()))
            }
        })
    }

    /// Returns whether or not the active session is a local session.  Returns
    /// None if there is no active session.
    pub fn active_session_is_local<C: ModelAsRef>(&self, ctx: &C) -> Option<bool> {
        self.active_block_session_id().and_then(|session_id| {
            let current_session = self.sessions.as_ref(ctx).get(session_id)?;
            Some(current_session.is_local())
        })
    }

    /// Returns the active session's launch shell, if it is specified.
    /// Returns None if there is no active session or if the current session does not
    /// have a launch shell.
    pub fn active_session_shell<C: ModelAsRef>(&self, ctx: &C) -> Option<ShellLaunchData> {
        self.active_block_session_id().and_then(|session_id| {
            let current_session = self.sessions.as_ref(ctx).get(session_id)?;
            current_session.launch_data().cloned()
        })
    }

    /// Returns the active session's WSL distribution information, if it exists.
    /// Returns None if there is no active session or if the current session is
    /// not a WSL session.
    pub fn active_session_wsl_distro<C: ModelAsRef>(&self, ctx: &C) -> Option<String> {
        self.active_block_session_id().and_then(|session_id| {
            let current_session = self.sessions.as_ref(ctx).get(session_id)?;
            let distro_name = current_session.wsl_distro_name();
            distro_name.map(|name| name.to_string())
        })
    }

    pub fn active_block_session_id(&self) -> Option<SessionId> {
        self.active_block_metadata
            .as_ref()
            .and_then(BlockMetadata::session_id)
    }

    pub fn active_session_shell_type<C: ModelAsRef>(&self, ctx: &C) -> Option<ShellType> {
        self.active_block_session_id()
            .and_then(|id| self.sessions.as_ref(ctx).get(id))
            .map(|s| s.shell().shell_type())
    }

    pub fn active_session_path_if_local<C: ModelAsRef>(&self, ctx: &C) -> Option<PathBuf> {
        if self.active_session_is_local(ctx) == Some(true) {
            self.active_block_metadata
                .as_ref()
                .and_then(BlockMetadata::current_working_directory)
                .and_then(|cwd| {
                    self.active_block_session_id()
                        .and_then(|active_session_id| {
                            self.sessions.as_ref(ctx).get(active_session_id)
                        })
                        .and_then(|active_session| {
                            active_session
                                .launch_data()
                                .and_then(|data| data.maybe_convert_absolute_path(cwd))
                        })
                })
                // Checking if the pwd from the active session actually exists
                // and if not (ie. directory was removed) - return None.
                .filter(|path| path.is_dir())
        } else {
            None
        }
    }

    pub fn input(&self) -> &ViewHandle<Input> {
        &self.input
    }

    pub fn active_session(&self) -> &ModelHandle<ActiveSession> {
        &self.active_session
    }

    pub fn find_bar(&self) -> &ViewHandle<Find<TerminalFindModel>> {
        &self.find_bar
    }

    pub fn has_highlighted_link(&self) -> bool {
        self.highlighted_link.is_some()
    }

    pub fn hovered_block_index(&self) -> Option<BlockIndex> {
        self.hovered_block_index
    }

    pub fn is_context_menu_open(&self) -> bool {
        self.context_menu_state.is_some()
    }

    pub fn last_focus_ts(&self) -> Option<NaiveDateTime> {
        self.last_focus_ts
    }

    pub fn is_read_only(&self) -> bool {
        self.model.lock().is_read_only()
    }

    /// Whether this terminal pane is responsible for uploading a file.
    pub fn is_ssh_uploader(&self) -> bool {
        self.is_ssh_file_uploader
    }

    pub fn set_is_ssh_uploader(&mut self, is_uploader: bool) {
        self.is_ssh_file_uploader = is_uploader;
    }

    pub fn ssh_file_upload(&self) -> &ViewHandle<FileUpload> {
        &self.ssh_file_upload
    }

    fn should_report_focus(&self, ctx: &mut ViewContext<Self>) -> bool {
        let model = self.model.lock();
        let focus_reporting_enabled = *AltScreenReporting::as_ref(ctx)
            .focus_reporting_enabled
            .value();
        focus_reporting_enabled
            && model.is_alt_screen_active()
            && model.alt_screen().is_mode_set(TermMode::FOCUS_IN_OUT)
    }

    fn maybe_report_focus_in(&mut self, ctx: &mut ViewContext<Self>) {
        if self.should_report_focus(ctx) && !self.is_focused_and_active {
            self.write_to_pty(EscCodes::FOCUS_IN, ctx);
        }
        self.is_focused_and_active = true;
    }

    fn contains_restored_remote_blocks(&self) -> bool {
        !self
            .model
            .lock()
            .block_list()
            .blocks()
            .iter()
            .all(|block| block.restored_block_was_local().unwrap_or(true))
    }

    fn maybe_report_focus_out(&mut self, ctx: &mut ViewContext<Self>) {
        if self.should_report_focus(ctx) && self.is_focused_and_active {
            self.write_to_pty(EscCodes::FOCUS_OUT, ctx);
        }
        self.is_focused_and_active = false;
    }

    /// Returns the `EntityId` of this view.
    pub fn id(&self) -> EntityId {
        self.view_id
    }

    pub fn pane_configuration(&self) -> &ModelHandle<PaneConfiguration> {
        &self.pane_configuration
    }

    pub fn is_input_box_visible(&self, model: &TerminalModel, app: &AppContext) -> bool {
        if model.is_read_only() {
            return false;
        }
        if self.has_active_cli_agent_input_session(app) {
            return true;
        }
        if model.is_alt_screen_active() || self.active_env_var_collection_block(app).is_some() {
            return false;
        }
        let block_list = model.block_list();
        !block_list.active_block().is_active_and_long_running()
            || block_list.is_writing_or_executing_in_band_command()
            || !block_list.is_bootstrapped()
    }

    pub fn has_active_env_var_block(&self, app: &AppContext) -> bool {
        self.active_env_var_collection_block(app).is_some()
    }

    /// Shuts down the pty and event loop, terminating the shell process.
    /// Also marks this view as manually shut down for telemetry attribution.
    pub fn shutdown_pty(&mut self, ctx: &mut ViewContext<Self>) {
        self.manual_pty_shutdown_requested = true;
        ctx.emit(Event::ShutdownPty);
    }

    fn user_write_ctrl_c_to_pty(&mut self, ctx: &mut ViewContext<Self>) {
        self.write_user_bytes_to_pty(vec![escape_sequences::C0::ETX], ctx);
    }

    fn handle_ctrl_c_input_event(
        &mut self,
        _cleared_buffer_len: usize,
        ctx: &mut ViewContext<Self>,
    ) {
        self.ctrl_c(ctx);
    }

    /// Windows users expect ctrl-c to copy if there is selected text. Otherwise,
    /// we perform the normal ctrl-c action.
    fn ctrl_c(&mut self, ctx: &mut ViewContext<Self>) {
        let (has_block_list_selection, has_alt_screen_selection, is_long_running) = {
            let model = self.model.lock();
            (
                model.block_list().selection().is_some(),
                model.alt_screen().selection().is_some(),
                model
                    .block_list()
                    .active_block()
                    .is_active_and_long_running(),
            )
        };
        self.ctrl_c_internal(
            !self.selected_blocks.is_empty(),
            has_block_list_selection,
            has_alt_screen_selection,
            is_long_running,
            ctx,
        );
        self.redetermine_global_focus(ctx);
        ctx.notify();
    }

    /// Copy if there is a selection. Otherwise, we defer to the normal ctrl-c
    /// behaviour.
    #[cfg(windows)]
    fn ctrl_c_internal(
        &mut self,
        has_copiable_block_selection: bool,
        has_block_list_selection: bool,
        has_alt_screen_selection: bool,
        is_long_running: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if has_block_list_selection {
            self.copy(ctx);
            self.clear_selections_without_focusing_input(ctx);
            return;
        } else if has_alt_screen_selection {
            self.copy(ctx);
            self.model.lock().alt_screen_mut().clear_selection();
            return;
        } else if has_copiable_block_selection {
            // If there are blocks selected, we want to copy them but
            // not prevent the normal ctrl-c behaviour.
            self.copy(ctx);
            self.clear_selections_without_focusing_input(ctx);
        }

        self.ctrl_c_to_active_block(is_long_running, ctx);
    }

    #[cfg(not(windows))]
    fn ctrl_c_internal(
        &mut self,
        has_copiable_block_selection: bool,
        has_block_list_selection: bool,
        has_alt_screen_selection: bool,
        is_long_running: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if has_block_list_selection || has_copiable_block_selection {
            self.clear_selections_without_focusing_input(ctx);
        } else if has_alt_screen_selection {
            self.model.lock().alt_screen_mut().clear_selection();
        }
        self.ctrl_c_to_active_block(is_long_running, ctx);
    }

    fn ctrl_c_to_active_block(&mut self, is_long_running: bool, ctx: &mut ViewContext<Self>) {
        if is_long_running {
            self.user_write_ctrl_c_to_pty(ctx);
        }
    }

    fn ctrl_d(&mut self, ctx: &mut ViewContext<Self>) {
        let arc = self.model.clone();
        let mut model = arc.lock();

        // Only write EOT to the PTY if the input box is not visible, which would
        // happen iff there is a long-running block. The one exception is when
        // the PTY is still bootstrapping, in which case the input would be shown
        // but we still want EOT written to the PTY in case there is a program
        // waiting for input during bootstrapping (e.g. omz update).
        if !self.is_input_box_visible(&model, ctx) || !model.block_list().is_bootstrapped() {
            // This is relevant for the case where the user enters CTRL-d while the
            // ssh wrapper command is being run. The EOT character doesn't stop
            // the session immediately, instead it waits until the command passed
            // to it is complete. This means that the SSH wrapper command will
            // still send the InitShell message to the terminal. In order to
            // prevent it from being processed, we keep state and clear it on
            // the next precmd (i.e. when the command completes).
            model.ignore_bootstrapping_messages();

            // Drop the model before writing bytes to the pty, otherwise we
            // get a deadlock.  This model locking is a bit of a mess.
            drop(model);
            self.write_user_bytes_to_pty(&[escape_sequences::C0::EOT][..], ctx);
        }
    }

    pub fn is_long_running(&self) -> bool {
        let model = self.model.lock();
        model
            .block_list()
            .active_block()
            .is_active_and_long_running()
            && !model.is_read_only()
    }

    pub fn was_ever_visible(&self) -> bool {
        self.was_ever_visible
    }

    pub fn content_element_height_lines(&self, app: &AppContext) -> Lines {
        element_size_at_last_frame(&self.content_element_position_id, self.window_id, app)
            .map(|size| Pixels::new(size.y()))
            .unwrap_or(self.size_info.pane_height_px())
            .to_lines(self.size_info.cell_height_px)
    }

    pub fn content_element_height_px(&self, app: &AppContext) -> f32 {
        element_size_at_last_frame(&self.content_element_position_id, self.window_id, app)
            .map(|size| size.y())
            .unwrap_or(self.size_info.pane_height_px().as_f32())
    }

    pub fn content_element_width_px(&self, app: &AppContext) -> f32 {
        element_size_at_last_frame(&self.content_element_position_id, self.window_id, app)
            .map(|size| size.x())
            .unwrap_or(self.size_info.pane_height_px().as_f32())
    }

    fn user_input_sequence(&mut self, code: &[u8], ctx: &mut ViewContext<Self>) {
        let sequence = EscCodes::build_escape_sequence(self.model.lock().deref(), code);
        self.control_sequence_on_terminal(&sequence, ctx);
    }

    fn control_sequence_on_terminal(&mut self, bytes: &[u8], ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            self.on_ssh_warpification_key_event(Some(SshKeyEvent::from_bytes(bytes)), ctx);
            self.write_user_bytes_to_pty(bytes.to_owned(), ctx);
        } else {
            safe_warn!(
                safe: ("command not long-running. ignoring control seq on terminal."),
                full: ("command not long-running. ignoring control seq on terminal: {:?}", bytes)
            )
        }
    }

    /// Emits an event indicating that this session has an active alt-screen or
    /// long-running and received keyboard input.
    /// Emits if at least one terminal inputs is synced, so receivers of this
    /// event must determine how to process this event.
    fn emit_non_editor_typed_event(&self, chars: Vec<u8>, ctx: &mut ViewContext<Self>) {
        if SyncedInputState::as_ref(ctx).is_syncing_any_inputs(ctx.window_id()) {
            ctx.emit(Event::SyncInput(SyncEvent {
                source_view_id: self.view_id,
                data: SyncInputType::NonEditorTyped {
                    chars: Arc::new(chars.clone()),
                },
            }));
        }
    }

    fn update_scroll_position_locking(
        &mut self,
        update: ScrollPositionUpdate,
        ctx: &mut ViewContext<Self>,
    ) {
        let mut model = self.model.lock();
        // Clear the cached pre-filter scroll position if a non-filter user
        // event is detected.
        if !matches!(
            update,
            ScrollPositionUpdate::AfterFilter { .. } | ScrollPositionUpdate::AfterResize
        ) {
            model.block_list_mut().clear_scroll_position_before_filter();
        }
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let viewport = self.viewport_state(model.block_list(), input_mode, ctx);
        if self.scroll_position.update(viewport, update, ctx) {
            ctx.notify();
            // Dismiss any visible tooltips when the scroll position changes
            drop(model);
            self.dismiss_tooltips(ctx);
        }
    }

    pub fn set_show_pane_accent_border(
        &mut self,
        show_accent_border: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        self.pane_configuration.update(ctx, |pane_config, ctx| {
            pane_config.set_show_accent_border(show_accent_border, ctx);
            ctx.notify();
        });
    }

    /// Receiving the warpui::Event::KeyDown event from a child element.
    /// Generally, this should be control characters rather than printable characters.
    fn keydown_on_terminal(&mut self, characters: &str, ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            self.on_ssh_warpification_key_event(Some(SshKeyEvent::from_chars(characters)), ctx);
            self.highlighted_link.invalidate();
            self.report_possible_typeahead(characters);
            self.write_user_bytes_to_pty(characters.as_bytes().to_vec(), ctx);
        } else {
            // When it's not a long-running command, we want to clear the selected block
            // and focus the editor. We specifically don't want to insert
            // anything into the input box. Characters that belong there should go through
            // `typed_characters_on_terminal` rather than through here.
            self.clear_selected_blocks(ctx);
            self.clear_selected_text(ctx);

            self.update_scroll_position_locking(ScrollPositionUpdate::AfterKeydownOnTerminal, ctx);
            self.redetermine_global_focus(ctx);
        }
    }

    fn should_write_typed_chars_to_pty(&self, ctx: &mut ViewContext<Self>) -> bool {
        // Lock the model once and hold it throughout the function
        let model = self.model.lock();

        // If the active block hasn't started yet, we don't want to write to the pty.
        // Note that we check block started and NOT block.is_long_running(), because
        // the block starts on enter but only becomes long running on receiving Preexec.
        // We want to make sure we capture any input between enter and receiving Preexec.
        if !model.block_list().active_block().started() {
            return false;
        }

        // Make sure we don't write any text to the pty until we've echoed out
        // the bootstrap script, otherwise the user could accidentally interfere
        // with bootstrap script execution.
        let was_bootstrap_script_echoed = self
            .sessions
            .as_ref(ctx)
            .has_pending_or_bootstrapped_session();
        was_bootstrap_script_echoed
    }
    /// Receiving a warpui::Event::TypedCharacters event from a child element.
    /// We can assume `characters` consists of all printable characters, and therefore,
    /// can go into the input box.
    fn typed_characters_on_terminal(&mut self, characters: &str, ctx: &mut ViewContext<Self>) {
        self.on_ssh_warpification_key_event(Some(SshKeyEvent::from_chars(characters)), ctx);
        if self.should_write_typed_chars_to_pty(ctx) {
            self.highlighted_link.invalidate();
            self.report_possible_typeahead(characters);
            self.write_user_bytes_to_pty(characters.as_bytes().to_vec(), ctx);
        } else {
            self.clear_selected_blocks(ctx);
            self.clear_selected_text(ctx);
            self.update_scroll_position_locking(ScrollPositionUpdate::AfterTypedCharacters, ctx);
            self.input
                .update(ctx, |input, ctx| input.system_insert(characters, ctx));
        }
    }

    /// Handles a file-tree drag-and-drop onto the terminal by piping the dropped text
    /// through the same path as user-typed characters for the active command.
    pub fn handle_file_tree_drop_on_active_command(
        &mut self,
        text: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        self.typed_characters_on_terminal(text, ctx);
    }

    fn set_marked_text_on_terminal(
        &mut self,
        marked_text: &str,
        selected_range: &Range<usize>,
        ctx: &mut ViewContext<Self>,
    ) {
        if !FeatureFlag::ImeMarkedText.is_enabled() {
            return;
        }
        self.model
            .lock()
            .set_marked_text(marked_text, selected_range);
        ctx.notify();
    }

    fn clear_marked_text_on_terminal(&mut self, ctx: &mut ViewContext<Self>) {
        if !FeatureFlag::ImeMarkedText.is_enabled() {
            return;
        }
        self.model.lock().clear_marked_text();
        ctx.notify();
    }

    pub(crate) fn write_to_pty<B: Into<Cow<'static, [u8]>>>(
        &mut self,
        data: B,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.emit(Event::WriteBytesToPty { bytes: data.into() });
    }

    /// Ends the current line before writing 1000 byte chunks to the pty with a small delay in
    /// between to work around a macos pty bug.
    fn clear_line_editor_and_write_to_pty_with_mac_workaround_hack<B: Into<Cow<'static, [u8]>>>(
        &mut self,
        data: B,
        ctx: &mut ViewContext<Self>,
    ) {
        // Ctrl-u + ctrl-k clears everything before the cursor, then everything after the cursor.
        // Ctrl-c is dangerous because it could cancel an ongoing command. We add an arbitrary space
        // first so that the ctrl-u always clears at least one character, avoiding the audible bell.
        let mut to_write = vec![
            b' ',
            escape_sequences::C0::VT,  // ctrl-k to clear forward
            escape_sequences::C0::NAK, // ctrl-u to clear backward
        ];
        to_write.extend_from_slice(&data.into());

        for (i, chunk) in to_write.chunks(1000).enumerate() {
            let chunk = chunk.to_vec();
            ctx.spawn(
                Timer::after(Duration::from_millis(i as u64 * 10)),
                move |me, _, ctx| me.write_to_pty(chunk, ctx),
            );
        }
    }

    /// Ends the current line before writing the given bytes to the PTY.
    fn clear_line_editor_and_write_to_pty<B: Into<Cow<'static, [u8]>>>(
        &mut self,
        data: B,
        ctx: &mut ViewContext<Self>,
    ) {
        // Ctrl-u + ctrl-k clears everything before the cursor, then everything after the cursor.
        // Ctrl-c is dangerous because it could cancel an ongoing command. We add an arbitrary space
        // first so that the ctrl-u always clears at least one character, avoiding the audible bell.
        let mut to_write = vec![
            b' ',
            escape_sequences::C0::VT,  // ctrl-k to clear forward
            escape_sequences::C0::NAK, // ctrl-u to clear backward
        ];
        to_write.extend_from_slice(&data.into());
        self.write_to_pty(to_write, ctx);
    }

    /// Writes to the PTY, resets selected blocks and updates scroll position.
    /// Also calls logic to emit a sync event.
    fn write_user_bytes_to_pty<B: Into<Cow<'static, [u8]>>>(
        &mut self,
        data: B,
        ctx: &mut ViewContext<Self>,
    ) {
        {
            let mut terminal_model = self.model.lock();
            let active_block = terminal_model.block_list().active_block();

            if active_block.is_active_and_long_running() && !active_block.has_received_user_input()
            {
                terminal_model
                    .block_list_mut()
                    .active_block_mut()
                    .mark_received_user_input();
            }
        }

        let bytes = data.into();
        let bytes_vec = bytes.to_vec();
        self.clear_selected_blocks(ctx);
        self.update_scroll_position_locking(ScrollPositionUpdate::AfterWriteUserBytesToPty, ctx);
        self.write_to_pty(bytes, ctx);
        self.emit_non_editor_typed_event(bytes_vec, ctx);
    }

    /// Write to the PTY if the session has finished bootstrapping and
    /// has an active long-running command.
    /// Never emits a sync event.
    fn write_to_pty_for_syncing_long_running_commands(
        &mut self,
        characters: Vec<u8>,
        ctx: &mut ViewContext<Self>,
    ) {
        let was_bootstrap_script_echoed = self
            .sessions
            .as_ref(ctx)
            .has_pending_or_bootstrapped_session();
        // Make sure we don't write any text to the pty until we've echoed out
        // the bootstrap script, otherwise the user could accidentally interfere
        // with bootstrap script execution.
        if was_bootstrap_script_echoed && self.is_long_running() {
            self.clear_selected_blocks(ctx);
            self.update_scroll_position_locking(
                ScrollPositionUpdate::AfterWriteUserBytesToPty,
                ctx,
            );
            self.write_to_pty(characters, ctx);
        }
    }

    /// Report user input to the terminal's typeahead model as potential typeahead.
    /// The model matches the input recorded here against the actual characters
    /// echoed to the pty to determine what is typeahead.
    fn report_possible_typeahead(&mut self, input: &str) {
        self.model.lock().push_user_input(input);
    }

    pub fn set_pending_command(&self, exec: &str, ctx: &mut ViewContext<Self>) {
        self.input.update(ctx, |input, ctx| {
            input.set_pending_command(exec, ctx);
        })
    }

    fn alt_scroll_cmd_sequence(&self, lines_to_scroll: i32) -> Vec<u8> {
        let cmd = if lines_to_scroll > 0 {
            EscCodes::ARROW_UP
        } else {
            EscCodes::ARROW_DOWN
        };
        EscCodes::build_escape_sequence_with_c1(C1::SS3, &[cmd])
    }

    fn alt_scroll_sequences(&mut self, lines_to_scroll: i32) -> Vec<u8> {
        let cmd = self.alt_scroll_cmd_sequence(lines_to_scroll);
        let lines = lines_to_scroll.unsigned_abs();
        let mut content = Vec::with_capacity(lines as usize * 3);

        for _ in 0..lines {
            content.extend_from_slice(&cmd);
        }
        content
    }

    fn alt_scroll(&mut self, lines_to_scroll: i32, ctx: &mut ViewContext<Self>) {
        // Scrolling on the alt screen can cause the grid content to change, so any link highlights are
        // no longer valid.
        self.highlighted_link.invalidate();

        let content = self.alt_scroll_sequences(lines_to_scroll);
        self.write_user_bytes_to_pty(content, ctx);
        ctx.notify();
    }

    pub fn input_size_at_last_frame(&self, app: &AppContext) -> Option<Vector2F> {
        app.element_position_by_id_at_last_frame(self.window_id, &self.input_position_id)
            .map(|bounds| bounds.size())
    }

    pub fn viewport_state<'a>(
        &self,
        block_list: &'a BlockList,
        input_mode: InputMode,
        app: &AppContext,
    ) -> ViewportState<'a> {
        let content_element_size =
            element_size_at_last_frame(&self.content_element_position_id, self.window_id, app)
                .unwrap_or(self.size_info.pane_size_px());
        ViewportState::new(
            block_list,
            self.snackbar_header_state.clone(),
            input_mode,
            *self.size_info,
            self.scroll_position.position(),
            None,
            self.horizontal_clipped_scroll_state.clone(),
            content_element_size,
            self.input_size_at_last_frame(app).unwrap_or_default(),
            AutoscrollBehavior::Always,
        )
    }

    /// Dismisses any open tooltips on the grid, returning whether any were actually closed.
    pub fn dismiss_tooltips(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        let was_open = self.is_any_tooltip_open();
        self.open_grid_link_tool_tip = None;
        self.open_secret_tool_tip = None;
        self.open_rich_content_link_tool_tip = None;

        if was_open {
            ctx.notify();
            // The mouse cursor may have been over the tooltip before it was dismissed. Reset it to
            // clear any lingering alternate pointers.
            ctx.reset_cursor();
        }
        was_open
    }

    fn is_any_tooltip_open(&self) -> bool {
        self.open_grid_link_tool_tip.is_some()
            || self.open_secret_tool_tip.is_some()
            || self.open_rich_content_link_tool_tip.is_some()
    }

    #[cfg(feature = "integration_tests")]
    pub fn is_secret_tooltip_open(&self) -> bool {
        self.open_secret_tool_tip.is_some()
    }

    fn handle_sessions_event(&mut self, event: SessionsEvent, ctx: &mut ViewContext<Self>) {
        match event {
            SessionsEvent::SessionInitialized { .. } => {
                self.handle_session_initialized(ctx);
            }
            SessionsEvent::SessionBootstrapped(event) => {
                self.handle_session_bootstrapped(*event, ctx);
            }
            _ => {}
        }
    }

    fn scroll(&mut self, delta: Lines, ctx: &mut ViewContext<Self>) {
        self.dismiss_tooltips(ctx);
        self.update_scroll_position_locking(
            ScrollPositionUpdate::AfterScrollEvent {
                scroll_delta: delta,
            },
            ctx,
        );
        ctx.notify();
    }

    fn handle_typeahead_event(&mut self, ctx: &mut ViewContext<Self>) {
        let mut model = self.model.lock();
        let Some((typeahead, num_typeahead_chars_inserted)) = model
            .block_list_mut()
            .early_output_mut()
            .advance_typeahead()
        else {
            return;
        };
        self.input.update(ctx, |input, ctx| {
            input.insert_typeahead_text(num_typeahead_chars_inserted, typeahead, ctx)
        });
        ctx.notify();
    }

    /// This function is invoked every time there is some form of view event
    /// such as a state change or terminal wakeup to update the view context.
    fn handle_terminal_wakeup(&mut self, _: (), ctx: &mut ViewContext<Self>) {
        // If find bar is active, we update the matches for the last/active block or the alt screen.
        if self.find_model.as_ref(ctx).is_find_bar_open() {
            self.find_model.update(ctx, |find_model, ctx| {
                find_model.rerun_find_on_active_grid(ctx);
            });
        }

        // For simplicity, we simply rescan the entire block for block filter matches.
        self.model
            .lock()
            .block_list_mut()
            .maybe_refilter_active_block_output();

        // If the block filter editor is open on an active block we update the
        // number of line matches.
        if let Some(block_index) = self.active_filter_editor_block_index {
            let model = self.model.lock();
            let active_block_index = model.block_list().active_block_index();
            let num_matched_lines = model
                .block_list()
                .num_matched_lines_in_filter_for_block(block_index);
            if block_index == active_block_index {
                self.block_filter_editor.update(ctx, |filter_editor, ctx| {
                    filter_editor.set_num_matched_lines(num_matched_lines);
                    ctx.notify();
                });
            }
        }

        // The active block height could have changed since the last time it was calculated, as
        // one cause of the Wakeup signal is the long-running process timer. Make sure that the
        // model is up-to-date with the current height information.
        if !self.model.lock().is_alt_screen_active() {
            let mut model = self.model.lock();
            model.block_list_mut().update_background_block_height();
            model.block_list_mut().update_active_block_height();
        }
        self.maybe_emit_terminal_view_state_changed_for_long_running_block(ctx);
        self.use_agent_footer.update(ctx, |footer, ctx| {
            footer.notify_and_notify_children(ctx);
        });

        // Need to re-render both the alt screen and the blocklist on keypresses.
        ctx.notify();
    }

    /// This function is invoked whenever we detect an SSH ControlMaster error,
    /// in which case completions will not work as expected.
    fn handle_control_master_error(&mut self, ctx: &mut ViewContext<Self>) {
        let active_session_id = self.active_block_session_id();
        // We don't want to display the error banner a second time in a given session
        // if the user has already closed it.  When we open the banner initially, we
        // store the session ID in here, so if the stored value matches the current
        // session, we've already shown the banner.
        //
        // TODO(vorporeal): This logic falls apart for nested ssh sessions - we could
        // show the banner in the outer ssh session, show it again for the inner ssh
        // session, then forgot that we already showed it for the outer session.  This
        // probably won't happen often, but it's something that we might want to clean
        // up eventually.
        if self.control_master_error_banner_state.associated_session_id != active_session_id {
            self.control_master_error_banner_state = ControlMasterErrorBannerState {
                is_open: true,
                associated_session_id: active_session_id,
            };

            ctx.notify();
        }
    }

    fn read_from_clipboard(
        shell_family: Option<ShellFamily>,
        ctx: &mut ViewContext<Self>,
    ) -> String {
        let content = ctx.clipboard().read();
        clipboard_content_with_escaped_paths(content, shell_family, false)
    }

    fn middle_click_paste_content(
        shell_family: Option<ShellFamily>,
        ctx: &mut ViewContext<Self>,
    ) -> String {
        let content = SelectionSettings::handle(ctx).update(ctx, |selection, ctx| {
            selection.read_for_middle_click_paste(ctx)
        });

        content
            .map(|content| clipboard_content_with_escaped_paths(content, shell_family, false))
            .unwrap_or_default()
    }

    /// Turns the active session into a bootstrapped subshell by writing the InitShell DCS hook
    fn trigger_subshell_bootstrap(
        &mut self,
        shell_type: Option<ShellType>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.dismiss_warpify_banner(&RememberForWarpification::DoNotRememberSubshellCommand, ctx);

        // Record the active long-running block so we can hide it later once the remote
        // actually confirms subshell bootstrap is in progress.
        // If the remote never emits InitShell, the block stays visible.
        {
            let model = self.model.lock();
            if model
                .block_list()
                .active_block()
                .is_active_and_long_running()
            {
                let block_id = model.block_list().active_block_id().clone();
                self.warpify_state.set_block_id(block_id);
            }
        }

        self.write_init_subshell_bytes_to_pty(shell_type, ctx);

        if !self.env_vars.is_empty() {
            self.start_bootstrap_timer(ENV_VAR_BOOTSTRAP_FAILED_DURATION, ctx);
            self.env_vars = Vec::new();
        } else {
            self.start_bootstrap_timer(BOOTSTRAP_FAILED_DURATION, ctx);
        }
    }

    /// Util method to update the ssh block, with a lock
    fn update_long_running_ssh_block_with_lock(&self, f: impl FnOnce(&mut Block)) -> bool {
        if let Some(block_id) = self.warpify_state.block_id() {
            if let Some(block) = self
                .model
                .lock()
                .block_list_mut()
                .mut_block_from_id(&block_id)
            {
                f(block);
                return true;
            }
        }
        false
    }

    fn cancel_bootstrap_workflow(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_ssh_blocks(ctx);
        self.update_long_running_ssh_block_with_lock(|block| {
            block.unhide();
        });
        self.warpify_state.delete_state();
        ctx.notify();
    }

    fn remove_ssh_block_by_id(&mut self, view_id: EntityId) {
        self.model
            .lock()
            .block_list_mut()
            .remove_rich_content(view_id);
    }

    fn clear_ssh_blocks(&mut self, ctx: &mut ViewContext<Self>) {
        self.dismiss_warpify_banner(&RememberForWarpification::DoNotRememberSSHHost, ctx);
        if let Some(ssh_block) = self.warpify_state.ssh_block_state() {
            let view_id = ssh_block.get_block_view_id();

            self.remove_ssh_block_by_id(view_id);

            self.redetermine_global_focus(ctx);

            self.warpify_state.clear_ssh_block_state();
        }
    }

    /// Collapses any expanded UX within SSH blocks.
    /// To ensure we can always see what we're typing, we collapse
    /// the SSH block when typing.
    fn on_ssh_warpification_key_event(
        &mut self,
        key_event: Option<SshKeyEvent>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.warpify_state.ssh_block_state().is_some() {
            if key_event.is_some_and(|key| key.is_ctrl_c()) {
                self.cancel_bootstrap_workflow(ctx);
            } else if self.warpify_state.should_prevent_input() {
                self.warpify_state.focus(ctx);
                self.warpify_state.collapse_ssh_block(ctx);
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterRichBlockUpdated,
                    ctx,
                );
                ctx.notify();
            }
        }
    }

    fn handle_remote_warpification_is_unavailable(
        &mut self,
        reason: WarpificationUnavailableReason,
        ctx: &mut ViewContext<Self>,
    ) {
        // Stop the pending timeout on warpification.
        self.warpify_state.abort_ssh_warpify_timeout();
        match &reason {
            WarpificationUnavailableReason::TmuxNotInstalled {
                system_details,
                root_access,
            } => {
                if system_details.writable_home != Some(true) {
                    if let Some(shell_type) = ShellType::from_name(&system_details.shell) {
                        self.trigger_subshell_bootstrap(Some(shell_type), ctx);
                        return;
                    }
                }

                if let Some(tmux_install_script) = install_tmux_script(system_details, ctx) {
                    let root_access = RootAccess::from_str(root_access).unwrap_or_default();
                    let tmux_root_install_script = if root_access == RootAccess::NoRootAccess {
                        None
                    } else {
                        install_root_tmux_script(
                            system_details,
                            ctx,
                            root_access == RootAccess::CanRunSudo,
                        )
                    };
                    self.add_ssh_install_tmux_block(
                        system_details,
                        tmux_install_script,
                        tmux_root_install_script,
                        false,
                        ctx,
                    );
                    return;
                }
            }
            WarpificationUnavailableReason::UnsupportedTmuxVersion { system_details } => {
                if system_details.writable_home != Some(true) {
                    if let Some(shell_type) = ShellType::from_name(&system_details.shell) {
                        self.trigger_subshell_bootstrap(Some(shell_type), ctx);
                        return;
                    }
                }

                if let Some(tmux_install_script) = install_tmux_script(system_details, ctx) {
                    self.add_ssh_install_tmux_block(
                        system_details,
                        tmux_install_script,
                        None,
                        true,
                        ctx,
                    );
                    return;
                }
            }
            _ => {}
        }
        self.add_ssh_error_block(reason, ctx);
    }

    fn add_ssh_warpify_prompt(
        &mut self,
        command: &str,
        ssh_host: Option<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        self.clear_ssh_blocks(ctx);
        self.handle_action(
            &TerminalAction::ShowWarpifySshBanner(command.to_owned(), ssh_host),
            ctx,
        );
    }

    /// This method assumes the active block in the blocklist is a long-running SSH command.
    fn add_ssh_warpifying_block(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_ssh_blocks(ctx);

        let show_ssh_block_debug = BlockVisibilitySettings::as_ref(ctx)
            .should_show_ssh_block
            .value();
        let (full_ssh_command, hidden_ssh_block_id) = {
            let mut model = self.model.lock();
            if !show_ssh_block_debug {
                model.block_list_mut().active_block_mut().hide();
            }

            (
                model.block_list().active_block().command_to_string(),
                model.block_list().active_block_id().clone(),
            )
        };

        let ssh_warpify_block_handle =
            ctx.add_typed_action_view(|_| SshWarpifyBlock::new(full_ssh_command));
        ctx.subscribe_to_view(&ssh_warpify_block_handle, move |me, _, event, ctx| {
            me.handle_ssh_warpify_block_event(event, ctx);
        });

        self.insert_rich_content(
            None,
            ssh_warpify_block_handle.clone(),
            Some(RichContentMetadata::SshWarpifyBlock {
                ssh_warpify_block_handle: ssh_warpify_block_handle.clone(),
            }),
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: true,
            },
            ctx,
        );

        ctx.focus(&ssh_warpify_block_handle);

        self.warpify_state.set_block_id(hidden_ssh_block_id);
        self.warpify_state
            .set_ssh_block_state(SshBlockState::Warpifying {
                handle: ssh_warpify_block_handle,
            });

        self.warpify_ssh_session(ctx);
    }

    /// This method assumes the active block in the blocklist is a long-running SSH command.
    fn add_ssh_install_tmux_block(
        &mut self,
        system_details: &SystemDetails,
        tmux_install_script: String,
        tmux_root_install_script: Option<String>,
        outdated_version: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        self.clear_ssh_blocks(ctx);

        let show_ssh_block_debug = BlockVisibilitySettings::as_ref(ctx)
            .should_show_ssh_block
            .value();
        let (full_ssh_command, hidden_ssh_block_id) = {
            let mut model = self.model.lock();
            if !show_ssh_block_debug {
                model.block_list_mut().active_block_mut().hide();
            }
            (
                model.block_list().active_block().command_to_string(),
                model.block_list().active_block_id().clone(),
            )
        };

        let ssh_host = self.warpify_state.get_pending_ssh_host();

        let ssh_install_tmux_block_handle = ctx.add_typed_action_view(|_| {
            SshInstallTmuxBlock::new(
                system_details.clone(),
                tmux_install_script,
                tmux_root_install_script,
                full_ssh_command,
                ssh_host,
                outdated_version,
            )
        });
        ctx.subscribe_to_view(&ssh_install_tmux_block_handle, move |me, _, event, ctx| {
            me.handle_ssh_install_tmux_block_event(event, ctx);
        });

        self.insert_rich_content(
            None,
            ssh_install_tmux_block_handle.clone(),
            Some(RichContentMetadata::SshInstallTmuxBlock {
                ssh_install_tmux_block_handle: ssh_install_tmux_block_handle.clone(),
            }),
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: true,
            },
            ctx,
        );

        ctx.focus(&ssh_install_tmux_block_handle);

        self.warpify_state.set_block_id(hidden_ssh_block_id);
        self.warpify_state
            .set_ssh_block_state(SshBlockState::InstallTmux {
                handle: ssh_install_tmux_block_handle,
            });
    }

    fn add_ssh_error_block(
        &mut self,
        error_reason: WarpificationUnavailableReason,
        ctx: &mut ViewContext<Self>,
    ) {
        // If there's already an error block showing, don't overwrite the existing one.
        if matches!(
            self.warpify_state.ssh_block_state(),
            Some(SshBlockState::Error { .. })
        ) {
            return;
        }

        self.clear_ssh_blocks(ctx);
        self.update_long_running_ssh_block_with_lock(|block| {
            block.unhide();
        });

        let ssh_host = self.warpify_state.take_pending_ssh_host();

        let ssh_error_block_handle =
            ctx.add_typed_action_view(|_| SshErrorBlock::new(error_reason.clone(), ssh_host));
        ctx.subscribe_to_view(&ssh_error_block_handle, move |me, _, event, ctx| {
            me.handle_ssh_error_block_events(event, ctx);
        });

        self.insert_rich_content(
            None,
            ssh_error_block_handle.clone(),
            Some(RichContentMetadata::SshErrorBlock {
                ssh_error_block_handle: ssh_error_block_handle.clone(),
            }),
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: true,
            },
            ctx,
        );

        self.warpify_state
            .set_ssh_block_state(SshBlockState::Error {
                handle: ssh_error_block_handle,
            });
        self.warpify_state.focus(ctx);
    }

    fn add_bootstrap_success_block(
        &mut self,
        SessionBootstrappedEvent {
            spawning_command,
            subshell_info,
            shell,
            session_type,
            ..
        }: SessionBootstrappedEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        let show_ssh_block_debug = BlockVisibilitySettings::as_ref(ctx)
            .should_show_ssh_block
            .value();
        if !show_ssh_block_debug {
            self.update_long_running_ssh_block_with_lock(|block| {
                block.hide();
            });
        }

        let warpification_source = match session_type {
            BootstrapSessionType::WarpifiedRemote => WarpificationSource::Ssh,
            BootstrapSessionType::Local => WarpificationSource::Subshell,
        };
        let disable_tmux = FeatureFlag::SSHTmuxWrapper.is_enabled()
            && matches!(warpification_source, WarpificationSource::Ssh)
            && { !self.model.lock().tmux_control_mode_active() };
        let ssh_success_block_handle = ctx.add_typed_action_view(|ctx| {
            WarpifySuccessBlock::new(
                warpification_source,
                spawning_command,
                subshell_info,
                shell,
                disable_tmux,
                ctx,
            )
        });
        ctx.subscribe_to_view(&ssh_success_block_handle, move |me, _, event, ctx| {
            me.handle_ssh_success_block_events(event, ctx);
        });

        self.clear_ssh_blocks(ctx);
        self.insert_rich_content(
            Some(RichContentType::WarpifySuccessBlock),
            ssh_success_block_handle.clone(),
            Some(RichContentMetadata::WarpifySuccessBlock {
                bootstrap_success_block_handle: ssh_success_block_handle.clone(),
            }),
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: false,
            },
            ctx,
        );
        self.warpify_state
            .set_ssh_block_state(SshBlockState::WarpifySuccess {
                handle: ssh_success_block_handle,
            });
        let active_session_id = self.active_block_session_id();
        self.warpify_state.on_warpify_start(active_session_id);
        self.refresh_warp_prompt(ctx);
    }

    fn handle_ssh_warpify_block_event(
        &mut self,
        event: &SshWarpifyBlockEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        fn dismiss_ssh_warpify_block(me: &mut TerminalView, ctx: &mut ViewContext<TerminalView>) {
            me.cancel_bootstrap_workflow(ctx);
        }

        match event {
            SshWarpifyBlockEvent::Cancel => {
                self.warpify_state.replace_timeout_id();
                dismiss_ssh_warpify_block(self, ctx);
            }
            SshWarpifyBlockEvent::Interrupt => {
                dismiss_ssh_warpify_block(self, ctx);
                self.warpify_state.abort_ssh_warpify_timeout();
                self.user_write_ctrl_c_to_pty(ctx);
            }
            SshWarpifyBlockEvent::WarpifySession => {
                self.add_ssh_warpifying_block(ctx);
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterRichBlockUpdated,
                    ctx,
                );
                ctx.notify();
            }
        }
    }

    fn handle_ssh_install_tmux_block_event(
        &mut self,
        event: &SshInstallTmuxBlockEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        fn cancel_tmux_install(me: &mut TerminalView, ctx: &mut ViewContext<TerminalView>) {
            me.cancel_bootstrap_workflow(ctx);
        }

        match event {
            SshInstallTmuxBlockEvent::Cancel => {
                cancel_tmux_install(self, ctx);
            }
            SshInstallTmuxBlockEvent::Interrupt => {
                cancel_tmux_install(self, ctx);
                self.warpify_state.abort_ssh_warpify_timeout();
                self.user_write_ctrl_c_to_pty(ctx);
            }
            SshInstallTmuxBlockEvent::InstallTmuxAndWarpify(install_method) => {
                self.clear_ssh_blocks(ctx);
                self.install_tmux_and_warpify(ctx, install_method);
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterRichBlockUpdated,
                    ctx,
                );
                ctx.notify();
            }
            SshInstallTmuxBlockEvent::ToggleScriptVisibility => {
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterRichBlockUpdated,
                    ctx,
                );
                ctx.notify();
            }
            SshInstallTmuxBlockEvent::ToggleTmuxInstallVisibility => {
                if let Some(ssh_block_id) = self.warpify_state.block_id() {
                    if let Some(is_visible) = self
                        .model
                        .lock()
                        .block_list_mut()
                        .toggle_visibility_of_block(&ssh_block_id)
                    {
                        if is_visible {
                            ctx.focus_self();
                        }
                    }
                    ctx.notify();
                }
            }
            SshInstallTmuxBlockEvent::UnhideTmuxInstall => {
                if let Some(ssh_block_id) = self.warpify_state.block_id() {
                    self.model
                        .lock()
                        .block_list_mut()
                        .unhide_block(&ssh_block_id);
                    ctx.focus_self();
                    ctx.notify();
                }
            }
        }
    }

    fn handle_ssh_error_block_events(
        &mut self,
        event: &SshErrorBlockEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            SshErrorBlockEvent::WarpifyWithoutTmux => {
                let shell_type = self.warpify_state.get_shell_type();
                self.clear_ssh_blocks(ctx);
                self.trigger_subshell_bootstrap(shell_type, ctx);
            }
            SshErrorBlockEvent::ContinueWithoutWarpification => {
                self.cancel_bootstrap_workflow(ctx);
            }
        }
    }

    fn handle_ssh_success_block_events(
        &mut self,
        event: &WarpifySuccessBlockEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            WarpifySuccessBlockEvent::OpenWarpifySettings => {
                ctx.emit(Event::OpenSettings(SettingsSection::Warpify));
            }
        }
    }

    fn dismiss_warpify_banner(
        &mut self,
        remember_command: &RememberForWarpification,
        ctx: &mut ViewContext<Self>,
    ) {
        {
            let mut model = self.model.lock();
            model.block_list_mut().set_active_block_banner(None);
        }

        // Also clear the warpify footer so it doesn't linger after warpification
        // starts, fails, or is cancelled.
        if FeatureFlag::WarpifyFooter.is_enabled() {
            self.use_agent_footer.update(ctx, |footer, ctx| {
                footer.clear_warpify_mode(ctx);
            });
        }

        match remember_command {
            RememberForWarpification::RememberSubshellCommand(command) => {
                WarpifySettings::handle(ctx).update(ctx, |warpify, ctx| {
                    warpify.denylist_subshell_command(command, ctx);
                });
            }
            RememberForWarpification::RememberSSHHost(host) => {
                WarpifySettings::handle(ctx).update(ctx, |warpify, ctx| {
                    warpify.denylist_ssh_host(host, ctx);
                });
            }
            RememberForWarpification::DoNotRememberSubshellCommand
            | RememberForWarpification::DoNotRememberSSHHost => {}
        }
    }

    fn show_warpify_banner(
        &mut self,
        input: WarpificationMode,
        title: &str,
        lowercase_title: &str,
        warpify_keybinding: Option<Keystroke>,
        ctx: &mut ViewContext<Self>,
    ) {
        if FeatureFlag::WarpifyFooter.is_enabled() {
            return;
        }

        let mut model = self.model.lock();

        // Don't show the warpify banner when an agent is monitoring the command either.

        let a11y_message = match &warpify_keybinding {
            Some(keystroke) => format!(
                "You can press {} to Warpify this {} for more Warper features.",
                keystroke.displayed(),
                lowercase_title
            ),
            None => format!("You can Warpify this {lowercase_title} for more Warper features."),
        };

        model
            .block_list_mut()
            .set_active_block_banner(Some(WithinBlockBanner::WarpifyBanner(
                WarpifyBannerState::new(input, warpify_keybinding),
            )));

        let a11y_content = AccessibilityContent::new(
            format!("{title} recognized."),
            a11y_message,
            WarpA11yRole::TextRole,
        );
        ctx.emit_a11y_content(a11y_content);

        ctx.notify();
    }

    fn insert_most_recent_command_correction(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(most_recent_command_correction) = self.most_recent_command_correction.as_ref() {
            self.input.update(ctx, |input, ctx| {
                input.replace_buffer_content(most_recent_command_correction.command.as_str(), ctx);
                ctx.notify()
            });
        }
    }

    fn alias_expansion_banner_action(
        &mut self,
        action: AliasExpansionBannerAction,
        ctx: &mut ViewContext<Self>,
    ) {
        use AliasExpansionBannerAction::*;

        match action {
            Enable => {
                let mut should_dismiss_banner = true;
                AliasExpansionSettings::handle(ctx).update(ctx, |settings, ctx| {
                    if let Err(e) = settings.alias_expansion_enabled.set_value(true, ctx) {
                        should_dismiss_banner = false;
                        log::error!("Failed to enable alias expansion setting from banner: {e}");
                    }
                });
                if should_dismiss_banner {
                    self.dismiss_alias_expansion_banner(ctx);
                }
            }
            Dismiss => {
                self.dismiss_alias_expansion_banner(ctx);
            }
        };
    }

    fn dismiss_alias_expansion_banner(&mut self, ctx: &mut ViewContext<Self>) {
        if let AliasExpansionBanner::Open { state } =
            &self.inline_banners_state.alias_expansion_banner
        {
            self.model
                .lock()
                .block_list_mut()
                .remove_inline_banner(state.id);
            self.inline_banners_state.alias_expansion_banner = AliasExpansionBanner::Closed;
        }
        ctx.notify();
    }

    /// Inserts a notifications discovery banner into the block list.
    fn insert_notifications_discovery_banner(
        &mut self,
        trigger: NotificationsTrigger,
        ctx: &mut ViewContext<Self>,
    ) {
        // Don't show if the user has dismissed the banner in this session.
        if matches!(
            self.inline_banners_state.notifications_discovery_banner,
            NotificationsDiscoveryBanner::Closed
        ) {
            return;
        }

        let banner = &self.inline_banners_state.notifications_discovery_banner;
        // Prevent stacking multiple banners or leaving empty space.
        if let NotificationsDiscoveryBanner::Open { state, .. } = banner {
            self.model
                .lock()
                .block_list_mut()
                .remove_inline_banner(state.banner_id);
        }

        let banner_id = self.inline_banners_state.next_banner_id();
        self.inline_banners_state.notifications_discovery_banner =
            NotificationsDiscoveryBanner::Open {
                trigger,
                state: NotificationsDiscoveryBannerState {
                    banner_id,
                    mouse_states: Default::default(),
                },
                request_outcome: None,
            };
        self.model
            .lock()
            .block_list_mut()
            .append_inline_banner(InlineBannerItem::new(
                banner_id,
                InlineBannerType::NotificationsDiscovery,
            ));

        let a11y_content = AccessibilityContent::new(
            trigger.discovery_banner_copy(),
            "You can enable notifications through the command palette.",
            WarpA11yRole::TextRole,
        );
        ctx.emit_a11y_content(a11y_content);

        ctx.notify();
    }

    /// Inserts a notifications error banner into the block list.
    fn insert_notifications_error_banner(&mut self, ctx: &mut ViewContext<Self>) {
        let banner_id = self.inline_banners_state.next_banner_id();

        self.inline_banners_state
            .notifications_error_banner
            .banner_type = NotificationsErrorBannerType::Open {
            state: NotificationsErrorBannerState {
                banner_id,
                mouse_states: Default::default(),
            },
        };
        self.model
            .lock()
            .block_list_mut()
            .append_inline_banner(InlineBannerItem::new(
                banner_id,
                InlineBannerType::NotificationsError,
            ));

        let banner_title = self
            .inline_banners_state
            .notifications_error_banner
            .error
            .as_ref()
            .map(|e| e.notifications_error_banner_title())
            .unwrap_or("Error sending notification");

        let a11y_content = AccessibilityContent::new(
            banner_title,
            "Make sure you have enabled access for Warper notifications in System Settings.",
            WarpA11yRole::TextRole,
        );
        ctx.emit_a11y_content(a11y_content);

        ctx.notify();
    }

    fn insert_command_correction(&mut self, correction: &Correction, ctx: &mut ViewContext<Self>) {
        self.input.update(ctx, |input, ctx| {
            input.replace_buffer_content(correction.command.as_str(), ctx);
            ctx.notify()
        });
    }

    fn insert_alias_expansion_banner(
        &mut self,
        aliased_command: AliasedCommand,
        ctx: &mut ViewContext<Self>,
    ) {
        if let AliasExpansionBanner::Open { .. } = self.inline_banners_state.alias_expansion_banner
        {
            // We only show this banner once to the user.
            log::warn!("Tried to insert more than one alias expansion banner");
            return;
        }
        let banner_id = self.inline_banners_state.next_banner_id();
        self.inline_banners_state.alias_expansion_banner = AliasExpansionBanner::Open {
            state: AliasExpansionBannerState {
                id: banner_id,
                aliased_command,
                yes_button_mouse_state: Default::default(),
                no_button_mouse_state: Default::default(),
            },
        };

        self.model
            .lock()
            .block_list_mut()
            .append_inline_banner(InlineBannerItem::new(
                banner_id,
                InlineBannerType::AliasExpansion,
            ));
        ctx.notify();
    }

    /// Inserts a vim keybinding banner into the blocklist.
    fn insert_vim_mode_banner(&mut self, ctx: &mut ViewContext<Self>) {
        let banner_id = self.inline_banners_state.next_banner_id();
        self.inline_banners_state.vim_banner_state = Some(VimModeBannerState {
            id: banner_id,
            yes_button_mouse_state: Default::default(),
            no_button_mouse_state: Default::default(),
        });

        self.model
            .lock()
            .block_list_mut()
            .append_inline_banner(InlineBannerItem::new(banner_id, InlineBannerType::VimMode));

        ctx.notify();
    }

    fn remove_vim_mode_banner(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(banner_state) = self.inline_banners_state.vim_banner_state.take() {
            self.model
                .lock()
                .block_list_mut()
                .remove_inline_banner(banner_state.id);
        }
        ctx.notify();
    }

    fn enable_vim_keybindings(&mut self, ctx: &mut ViewContext<Self>) {
        AppEditorSettings::handle(ctx).update(ctx, |editor_settings, ctx| {
            if editor_settings.vim_mode.set_value(true, ctx).is_ok() {}
        });
    }

    fn handle_vim_banner_action(
        &mut self,
        action: VimModeBannerAction,
        ctx: &mut ViewContext<Self>,
    ) {
        if action == VimModeBannerAction::Enable {
            self.enable_vim_keybindings(ctx);
        }
        self.remove_vim_mode_banner(ctx);
        VimBannerSettings::handle(ctx).update(ctx, |banner_settings, model_ctx| {
            report_if_error!(banner_settings
                .vim_keybindings_banner_state
                .set_value(BannerState::Dismissed, model_ctx));
        });
    }

    /// Inserts a banner notifying the user that the shell process has terminated.
    fn insert_shell_process_terminated_banner(
        &mut self,
        termination_type: shell_terminated_banner::TerminationType,
        ctx: &mut ViewContext<Self>,
    ) {
        // If we successfully bootstrapped, show a simple "Shell exited" banner.
        if self.is_login_shell_bootstrapped {
            let banner_id = self.inline_banners_state.next_banner_id();
            self.inline_banners_state.shell_process_terminated_banner =
                Some(ShellProcessTerminatedBanner {
                    banner_id,
                    was_premature_termination: !self.is_login_shell_bootstrapped,
                });
            // In this case, the active block is actually the last block that was run
            // before exiting; it's not a special hidden block. In other words, the "active" block
            // is read-only and no additional blocks will be added to the block list. That's why
            // we need to explicitly insert _after_ the active block.
            let active_block_index = self.model.lock().block_list().active_block_index();
            self.model
                .lock()
                .block_list_mut()
                .insert_inline_banner_after_block(
                    active_block_index,
                    InlineBannerItem::new(banner_id, InlineBannerType::ShellProcessTerminated),
                );
        } else {
            let banner = ctx.add_typed_action_view(|ctx| {
                shell_terminated_banner::ShellTerminatedBanner::new(termination_type, ctx)
            });

            self.insert_rich_content(
                None,
                banner,
                None,
                RichContentInsertionPosition::Append {
                    insert_below_long_running_block: true,
                },
                ctx,
            );
        }

        ctx.notify();
    }

    /// Redetermine focus in the terminal view -- note that this will not steal focus
    /// from other parts of the app, the find bar, or the block filter editor.
    ///
    /// See [`Self::redetermine_global_focus`] to change focus without checking that the terminal is focused.
    fn redetermine_terminal_focus(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        // Only reset the focus if this terminal is currently focused, don't steal it from
        // another part of the app
        let reset_focus = ctx.is_self_or_child_focused()
            && !self.find_bar.is_self_or_child_focused(ctx)
            && !self.block_filter_editor.is_self_or_child_focused(ctx);
        if reset_focus {
            self.redetermine_global_focus(ctx);
        }

        reset_focus
    }

    /// Recomputes the chip values for the Warp prompt (i.e. _not_ PS1).
    fn refresh_warp_prompt(&mut self, ctx: &mut ViewContext<Self>) {
        // Ask the per-repo sub-model to re-fetch metadata so the chip values
        // reflect the latest git state (branch, diff stats, etc.).
        #[cfg(feature = "local_fs")]
        if let Some(handle) = &self.git_repo_status {
            handle.update(ctx, |model, ctx| {
                model.refresh_metadata(ctx);
            });
        }

        self.input.update(ctx, |input, ctx| {
            input.update_prompt_display_chips(ctx);
        });

        self.current_prompt.update(ctx, |prompt_type, ctx| {
            if let PromptType::Dynamic { prompt } = prompt_type {
                prompt.update(ctx, |current_prompt, ctx| {
                    current_prompt
                        .update_context(self.model.lock().block_list().active_block(), ctx);
                });
            }
        });
    }

    pub fn current_state(&self) -> TerminalViewStateChange {
        self.current_state
    }

    #[cfg(feature = "integration_tests")]
    pub fn current_prompt(&self) -> ModelHandle<PromptType> {
        self.current_prompt.clone()
    }

    fn set_current_state(&mut self, new_state: TerminalViewState, ctx: &mut ViewContext<Self>) {
        self.current_state = TerminalViewStateChange {
            state: new_state,
            timestamp: Instant::now(),
        };

        ctx.emit(Event::TerminalViewStateChanged);

        // Notify pane header to re-render (error indicator may change).
        self.pane_configuration.update(ctx, |config, ctx| {
            config.notify_header_content_changed(ctx);
        });
    }

    fn maybe_emit_terminal_view_state_changed_for_long_running_block(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.did_notify_long_running || !self.is_long_running() {
            return;
        }

        self.did_notify_long_running = true;
        ctx.emit(Event::TerminalViewStateChanged);
        self.update_pane_configuration(ctx);

        // Redetermine focus when the block becomes long-running. This recovers focus for
        // queued commands: when the previous block completes, focus moves to the input box
        // (because no new block is live yet), and nothing moves it back once the queued
        // block starts. By the time we arrive here `is_active_and_long_running()` is true,
        // so `redetermine_global_focus` correctly returns focus to the terminal view.
        //
        // Skip this pre-bootstrap: long-running pre-bootstrap blocks (e.g. a `.zshrc` that
        // issues a `read` prompt) need the input box to remain focused so the user can type
        // a response to unblock bootstrap.
        if self.model.lock().block_list().is_bootstrapped() {
            self.redetermine_terminal_focus(ctx);
        }
    }

    fn on_user_block_completed(&mut self, _block_id: &BlockId, _ctx: &mut ViewContext<Self>) {
        self.model
            .lock()
            .clear_pending_warp_initiated_control_mode();
        self.model.lock().end_notify_on_ssh_login_complete();
    }

    fn active_block_is_considered_remote(&self, app: &AppContext) -> bool {
        let model = self.model.lock();
        let active_block = model.block_list().active_block();
        self.is_block_considered_remote(
            active_block.session_id(),
            Some(&active_block.command_to_string()),
            app,
        )
    }

    /// Returns true if the block is considered remote.
    ///
    /// Note that we don't know for sure if a block is remote, because we can only detect
    /// warpified remote blocks.
    ///
    /// For some organizations, we accept a regex list that we run against commands to
    /// further make the determination.
    fn is_block_considered_remote(
        &self,
        session_id: Option<SessionId>,
        _command: Option<&str>,
        app: &AppContext,
    ) -> bool {
        session_id
            .and_then(|id| self.sessions.as_ref(app).get(id))
            .is_some_and(|session| !session.is_local())
    }

    fn handle_terminal_event(&mut self, event: &ModelEvent, ctx: &mut ViewContext<Self>) {
        match event {
            ModelEvent::TerminalClear => {
                self.handle_terminal_wakeup((), ctx);
                self.update_scroll_position_locking(ScrollPositionUpdate::AfterClear, ctx);
                ctx.notify();
            }
            ModelEvent::Title(title) => {
                self.terminal_title = title.to_owned();
                if self.ignore_next_set_title_event {
                    self.ignore_next_set_title_event = false;
                } else {
                    self.update_pane_configuration(ctx);
                }
            }
            ModelEvent::ClipboardStore(_, contents) => {
                let access = *TerminalSettings::as_ref(ctx).osc52_clipboard_access;
                if access.allows_write() {
                    ctx.clipboard()
                        .write(ClipboardContent::plain_text(contents.to_owned()));
                } else {
                    log::info!(
                        "OSC 52 clipboard write blocked by terminal.osc52_clipboard_access setting"
                    );
                    self.show_osc52_clipboard_blocked_banner(Osc52ClipboardBlockedType::Write, ctx);
                }
            }
            ModelEvent::ClipboardLoad(_, format) => {
                let access = *TerminalSettings::as_ref(ctx).osc52_clipboard_access;
                if access.allows_read() {
                    self.write_to_pty(
                        format(&TerminalView::read_from_clipboard(
                            Some(self.shell_family(ctx)),
                            ctx,
                        ))
                        .into_bytes(),
                        ctx,
                    );
                } else {
                    log::info!(
                        "OSC 52 clipboard read blocked by terminal.osc52_clipboard_access setting"
                    );
                    self.show_osc52_clipboard_blocked_banner(Osc52ClipboardBlockedType::Read, ctx);
                }
            }
            ModelEvent::CursorBlinkingChange(_) => {}
            ModelEvent::MouseCursorDirty => {}
            ModelEvent::Bell => {
                if *TerminalSettings::as_ref(ctx).use_audible_bell {
                    if let Err(e) = AudibleBell::as_ref(ctx).ring() {
                        log::warn!("Unable to play bell: {e:#}");
                    }
                }
                // TODO(vorporeal): Remove this once we have a visual bell
                // indicator in terminal tabs.
                ctx.request_user_attention();
            }
            ModelEvent::Exit { .. } => {
                // If the pty spawn has failed, we've already inserted a banner.
                if !self.pty_spawn_failed {
                    let shell_detail = self.shell_detail.take().unwrap_or("shell".to_owned());
                    self.insert_shell_process_terminated_banner(
                        shell_terminated_banner::TerminationType::Premature { shell_detail },
                        ctx,
                    );
                }
                // Mark the editor as disabled to ensure user interactions with
                // it are ignored.
                self.input.update(ctx, |input, ctx| {
                    input.editor().update(ctx, |editor, ctx| {
                        editor
                            .set_interaction_state(crate::editor::InteractionState::Disabled, ctx);
                    });
                });

                // If we failed to bootstrap by the time we exited, show the
                // bootstrap block so the user might be able to see what went wrong.
                if !self.is_login_shell_bootstrapped {
                    self.show_initialization_block();
                }

                if !self.pty_spawn_failed {
                    ctx.emit(Event::Exited);
                }
            }
            ModelEvent::BlockCompleted(block_completed_event) => {
                record_trace_event!("command_execution:block_completed");
                end_trace_after_next!("window:redraw:end");
                let block_completed_event_clone = block_completed_event.clone();
                self.input.update(ctx, |input, ctx| {
                    input.handle_block_completed_event(block_completed_event_clone, ctx);
                });

                if !matches!(block_completed_event.block_type, BlockType::BootstrapHidden) {
                    if let Some(env_var_block) = self.active_env_var_collection_block(ctx) {
                        let output_truncated =
                            if let BlockType::User(completed) = &block_completed_event.block_type {
                                Some(completed.output_truncated.clone())
                            } else {
                                None
                            };
                        env_var_block.update(ctx, move |block, ctx| {
                            if block.is_running() {
                                match output_truncated {
                                    // If we have a non-empty response we assume it's an error. We are
                                    // relying on this because we don't get a non-zero exit code for the
                                    // `export` function
                                    Some(output) if !output.is_empty() => {
                                        block.on_failed(Some(output), ctx)
                                    }
                                    _ => block.on_succeeded(ctx),
                                }
                            }
                        });
                    }
                }

                // If this block ran a possible subshell command, and it exited before the 1s timer
                // completed, abort showing the banner.
                if let Some(abort_handle) = self.warpify_state.take_subshell_banner_abort_handle() {
                    abort_handle.abort();
                }

                // In-band commands finishing should never trigger a focus change as it could steal
                // focus from the TerminalView.
                if !matches!(block_completed_event.block_type, BlockType::InBandCommand) {
                    let reset_focus = self.redetermine_terminal_focus(ctx);
                    // There are two different cases for redraws here:
                    // 1. If this terminal or its children were focused, redraw immediately after
                    //    this event.
                    // 2. Otherwise, redraw after the next terminal wakeup.
                    //
                    // Additionally, our API for measuring the latency requires installing a
                    // callback for the next redraw. We only want to install this callback in the
                    // first case because otherwise, it could be inaccurate.
                    //
                    // Since our baseline commands are all very small, when the command finishes,
                    // the same terminal almost certainly still has the focus.
                    if reset_focus {
                        if let Some(block_latency_data) = &block_completed_event.block_latency_data
                        {
                            self.install_block_latency_log_callback(
                                block_latency_data.clone(),
                                ctx,
                            );
                        }
                    }
                }

                if let BlockType::User(_) = &block_completed_event.block_type {
                    self.on_user_block_completed(&block_completed_event.block_id, ctx);
                }

                // Clear any stale warpify mode so it doesn't leak into the next command's footer rendering.
                self.use_agent_footer.update(ctx, |footer, ctx| {
                    footer.clear_warpify_mode(ctx);
                });
                self.hide_use_agent_footer_in_blocklist(ctx);
                if matches!(block_completed_event.block_type, BlockType::User(_)) {
                    // Close the rich input editor if it was open (side effects
                    // like input config restore happen reactively).
                    // The auto-toggle flag is irrelevant here because the
                    // session is removed immediately afterwards.
                    self.close_cli_agent_rich_input(ctx);
                    CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions_model, ctx| {
                        sessions_model.remove_session(self.view_id, ctx);
                    });
                }

                let next_block_index = block_completed_event.block_index + BlockIndex::from(1);

                // Don't populate mouse states for In-Band blocks. In-band blocks are hidden to the
                // user and there can be an arbitrarily large number of blocks as the user types
                // and interacts with the session. This in turn can cause performance and memory
                // issues since we clone the mouse states on every render.
                if !matches!(block_completed_event.block_type, BlockType::InBandCommand) {
                    self.block_list_mouse_states
                        .label_mouse_states
                        .entry(next_block_index)
                        .or_default();
                    self.block_list_mouse_states
                        .bookmark_mouse_states
                        .entry(next_block_index)
                        .or_default();
                    self.block_list_mouse_states
                        .filter_mouse_states
                        .entry(next_block_index)
                        .or_default();
                }

                // Refresh the pane title now that
                // is_long_running() has become false. Without this, the title stays at the
                // terminal title until the shell's precmd hook fires its next SetTitle event.
                self.update_pane_configuration(ctx);
            }
            ModelEvent::VisibleBootstrapBlock => {
                // We don't want to focus the input box in the case where
                // the block list isn't bootstrapped and there's a visible
                // bootstrap block oh-my-zsh (update prompt appears). In the
                // case, we want the block to be focused because otherwise,
                // users get stuck as they'd otherwise need to click into the
                // box to respond to whether or not they want to update oh my zsh.
                self.focus_terminal(ctx);
            }
            ModelEvent::AfterBlockStarted {
                command,
                is_for_in_band_command,
                ..
            } => {
                self.any_session_contains_remote_blocks |=
                    self.active_block_is_considered_remote(ctx);

                if *is_for_in_band_command {
                    return;
                }
                self.did_notify_long_running = false;

                // Snapshot the prompt state as of when the command began executing.
                // Commands may themselves affect the prompt (if running `git checkout`), for
                // example, so we want the saved prompt state to match what the user saw when
                // they entered the command.
                let prompt_snapshot = self.current_prompt.as_ref(ctx).snapshot(ctx);
                self.model
                    .lock()
                    .block_list_mut()
                    .active_block_mut()
                    .set_prompt_snapshot(prompt_snapshot);

                // Clear any previously active AM query suggestion banners and hidden blocks.

                // If the first word of the command is a shell alias, expand it
                // for subshell/SSH detection. This enables warpification for
                // aliased SSH commands (e.g. `alias myssh='ssh user@host'`).
                let expanded_command = self
                    .active_block_session_id()
                    .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
                    .and_then(|session| {
                        let (first_word, rest) = command_first_word_and_suffix(command)?;
                        let alias_value = session.alias_value(first_word)?;
                        Some(format!("{alias_value}{rest}"))
                    });
                let warpify_command = expanded_command.as_deref().unwrap_or(command.as_str());

                // Check if the current running command spawns a subshell eligible for Warpification.
                let shell_family = self.shell_family(ctx);
                let warpify_settings = WarpifySettings::as_ref(ctx);
                let is_compatible_subshell_command = warpify_settings
                    .is_compatible_subshell_command(command, shell_family)
                    || warpify_settings
                        .is_compatible_subshell_command(warpify_command, shell_family);
                let command_is_denylisted = warpify_settings
                    .is_denylisted_subshell_command(command)
                    || warpify_settings.is_denylisted_subshell_command(warpify_command);

                if is_compatible_subshell_command {
                    if command_is_denylisted {
                        // Don't auto-warpify or surface warpification for these commands.
                    } else if let Some(shell_type) = self.pending_auto_bootstrap_shell_type.take() {
                        // If there is a subshell we're waiting to bootstrap until we receive
                        // the preexec hook, now we can bootstrap it.
                        let auto_warpify_abort_handle = ctx.spawn_abortable(
                            Timer::after(Duration::from_millis(AUTO_WARPIFY_DELAY)),
                            move |me, _, ctx| {
                                me.trigger_subshell_bootstrap(Some(shell_type), ctx);
                            },
                            |_, _| (),
                        );
                        self.warpify_state
                            .add_auto_warpify_abort_handle(auto_warpify_abort_handle);
                    } else {
                        // Wait 1 second before showing the banner, just to make sure the
                        // command stays running for a bit. If the command fails instantly,
                        // we don't want to flicker the banner away so quickly.
                        let command = command.clone();
                        self.warpify_state
                            .add_subshell_banner_abort_handle(ctx.spawn_abortable(
                                Timer::after(*SUBSHELL_BANNER_DELAY_DURATION),
                                |view, _, ctx| {
                                    if FeatureFlag::WarpifyFooter.is_enabled() {
                                        view.show_warpify_footer(
                                            WarpificationMode::subshell(command),
                                            ctx,
                                        );
                                    } else {
                                        view.handle_action(
                                            &TerminalAction::ShowSubshellBanner(command),
                                            ctx,
                                        );
                                    }
                                },
                                |_, _| {},
                            ));
                    }
                } else {
                    if let Some(ssh_host) =
                        parse_interactive_ssh_command(warpify_command).map(|cmd| cmd.host)
                    {
                        if !self.model.lock().tmux_control_mode_active() {
                            self.warpify_state
                                .set_pending_ssh_host(warpify_command.to_string(), ssh_host);
                            self.model.lock().start_notify_on_end_of_ssh_login();
                        }
                    } else {
                        self.warpify_state.clear_pending_ssh_host();

                        ctx.spawn(
                                Timer::after(Duration::from_millis(
                                    LONG_RUNNING_COMMAND_DURATION_MS,
                                )),
                                move |me, _, ctx| {
                                    // Detect CLI agent and create session before
                                    // showing the footer, so the session drives
                                    // the footer rather than the other way around.
                                    let detection = {
                                        let model = me.model.lock();
                                        me.detect_cli_agent_from_model(&model, ctx)
                                    };
                                    let view_id = me.view_id;
                                    CLIAgentSessionsModel::handle(ctx).update(
                                        ctx,
                                        |sessions_model, ctx| match detection {
                                            Some((agent, ref custom_command_prefix))
                                                if !sessions_model
                                                    .session(view_id)
                                                    .is_some_and(|s| s.agent == agent) =>
                                            {
                                                let remote_host =
                                                    me.active_session_remote_host(ctx);
                                                sessions_model.set_session(
                                                    view_id,
                                                    CLIAgentSession {
                                                        agent,
                                                        status: CLIAgentSessionStatus::InProgress,
                                                        session_context:
                                                            CLIAgentSessionContext::default(),
                                                        input_state: CLIAgentInputState::Closed,
                                                        should_auto_toggle_input: *CLIAgentSettings::as_ref(
                                                            ctx,
                                                        )
                                                        .auto_open_rich_input_on_cli_agent_start,
                                                        listener: None,
                                                        plugin_version: None,
                                                        remote_host,
                                                        draft_text: None,
                                                        custom_command_prefix: custom_command_prefix.clone(),
                                                    },
                                                    ctx,
                                                );
                                            }
                                            _ => {}
                                        },
                                    );

                                    // Codex doesn't use the sentinel-based plugin protocol,
                                    // so create the listener proactively on command detection
                                    // (rather than waiting for a SessionStart event).
                                    if matches!(detection, Some((CLIAgent::Codex, _))) {
                                        me.register_cli_agent_listener_without_session_start_event(
                                            CLIAgent::Codex,
                                            ctx,
                                        );
                                    }

                                    me.maybe_show_use_agent_footer_in_blocklist(ctx);
                                    me.maybe_auto_open_cli_agent_rich_input(ctx);


                                },
                            );
                    }

                    self.set_current_state(TerminalViewState::LongRunning, ctx);
                    ctx.emit(Event::BlockStarted {
                        is_for_in_band_command: *is_for_in_band_command,
                    });
                }
            }
            ModelEvent::AfterBlockCompleted(AfterBlockCompletedEvent { block_type, .. }) => {
                // To automatically warpify a subshell, we run the relevant command to open the
                // subshell and create a future to delay bootstrapping the subshell long enough for
                // the command to complete. We receive AfterBlockCompleted if the subshell command
                // returns an error or the user exits the subshell. Here we abort the future to
                // avoid an attempt to trigger bootstrapping if the subshell command failed. If the
                // future already resolved, abort has no effect. We handle this as early as possible
                // because the abort is time sensitive.
                self.warpify_state.abort_auto_warpify();

                let active_session = self
                    .active_block_session_id()
                    .and_then(|id| self.sessions.as_ref(ctx).get(id));
                if let Some(active_session) = active_session {
                    if !active_session.has_attempted_to_load_external_commands() {
                        ctx.background_executor()
                            .spawn(async move { active_session.load_external_commands().await })
                            .detach();
                    }
                }

                let active_session_id = self.active_block_session_id();
                if let Some(block_id) = self
                    .warpify_state
                    .get_completed_warpify_session_id(active_session_id, ctx)
                {
                    self.remove_ssh_block_by_id(block_id);
                }

                self.dismiss_warpify_banner(
                    &RememberForWarpification::DoNotRememberSubshellCommand,
                    ctx,
                );

                // Emit PendingCommandCompleted when a pending command's block
                // finishes (e.g. tab config setup commands like `git worktree add`).
                if self.awaiting_pending_command_completion {
                    self.awaiting_pending_command_completion = false;
                    ctx.emit(Event::PendingCommandCompleted);

                    // finished, enter it now.
                }

                // For the case when the user uses session configuration with a
                // command list, we execute the command after a delay.
                // The delay is necessary because the shell needs a tiny bit of
                // extra time after the last precmd function is finished.
                // Additionally, it's possible for hooks to install themselves after the warp
                // precmd. For example, `fig_precmd` does this.
                if self.is_login_shell_bootstrapped {
                    let _ = ctx.spawn(
                        async move {
                            warpui::r#async::Timer::after(EXECUTE_PENDING_COMMAND_DELAY).await;
                        },
                        Self::execute_pending_command,
                    );
                }

                // When a block completes, we need to update the prompt for the next
                // active block. We specifically want to avoid doing this for in-band
                // commands because otherwise we'll create a loop if updating the prompt
                // involves running an in-band command. Similarly, we want to ensure
                // that the shell has been bootstrapped. Since we're in the BlockCompleted
                // event, that also implies we would have received the first precmd
                // so we know that the active block has valid metadata.
                if !matches!(block_type, BlockType::InBandCommand)
                    && self
                        .model
                        .lock()
                        .block_list()
                        .is_bootstrapping_precmd_done()
                {
                    self.refresh_warp_prompt(ctx);
                }

                if let BlockType::User(block_completed) = block_type {
                    if let Some(block_duration) =
                        self.block_duration(&block_completed.serialized_block)
                    {
                        self.maybe_send_block_completed_notification(
                            block_completed,
                            block_duration,
                            ctx,
                        );
                    }

                    self.maybe_generate_command_corrections(block_completed, ctx);

                    if self.can_suggest_alias_expansion(ctx) {
                        self.maybe_suggest_alias_expansion(block_completed, ctx);
                    }

                    self.maybe_suggest_open_in_warp(block_completed, ctx);

                    // Check if the user tried to run an AWS login command but AWS CLI wasn't installed.
                    // This runs after other suggestion checks and may add its own banner alongside them.

                    let terminal_view_state = {
                        let model = self.model.lock();
                        match model.block_list().last_non_hidden_block() {
                            Some(block) if block.has_failed() => TerminalViewState::Errored,
                            _ => TerminalViewState::Normal,
                        }
                    };
                    self.did_notify_long_running = false;
                    self.set_current_state(terminal_view_state, ctx);

                    if let (
                        Some(active_session_id),
                        exit_code,
                        Some(start_ts),
                        Some(completed_ts),
                        Some(model_event_sender),
                    ) = (
                        self.active_block_session_id(),
                        block_completed.serialized_block.as_ref().exit_code,
                        block_completed.serialized_block.as_ref().start_ts,
                        block_completed.serialized_block.as_ref().completed_ts,
                        &self.model_event_sender,
                    ) {
                        History::handle(ctx).update(ctx, move |history, _ctx| {
                            history.mark_command_as_finished(
                                active_session_id,
                                start_ts,
                                completed_ts,
                                exit_code,
                            );
                        });

                        let sender_clone = model_event_sender.clone();
                        let update_finished_command_event =
                            persistence::ModelEvent::UpdateFinishedCommand {
                                metadata: FinishedCommandMetadata {
                                    exit_code,
                                    start_ts,
                                    completed_ts,
                                    session_id: active_session_id,
                                },
                            };
                        let _ = ctx.spawn(
                            async move {
                                // Sending over a sync sender can block the current thread, so we do this async.
                                sender_clone.send(update_finished_command_event)
                            },
                            move |_, res, _| {
                                if let Err(err) = res {
                                    log::error!(
                                        "Error sending UpdateFinishedCommand event: {err:?}"
                                    );
                                }
                            },
                        );
                    }

                    // Emit the event to the parent view. This will save the block to sqlite if
                    // session restoration is enabled.
                    ctx.emit(Event::BlockCompleted {
                        block: block_completed.serialized_block.clone(),
                        is_local: !self.is_block_considered_remote(
                            block_completed.serialized_block.session_id,
                            Some(&block_completed.command),
                            ctx,
                        ),
                    });
                } else if let BlockType::Background(serialized_block) = block_type {
                    // Because background output blocks are before the active block, they need to be saved
                    // via a BlockCompleted event but don't affect focus or input.
                    ctx.emit(Event::BlockCompleted {
                        block: serialized_block.clone(),
                        is_local: !self.is_block_considered_remote(
                            serialized_block.session_id,
                            None,
                            ctx,
                        ),
                    });
                } else if let BlockType::BootstrapVisible(serialized_block) = block_type {
                    // Re-compute the focus after the visible bootstrap block has completed.
                    self.redetermine_terminal_focus(ctx);
                    ctx.emit(Event::BlockCompleted {
                        block: serialized_block.clone(),
                        is_local: !self.is_block_considered_remote(
                            serialized_block.session_id,
                            None,
                            ctx,
                        ),
                    });
                }

                self.input.update(ctx, |input, ctx| {
                    input.handle_after_block_completed_event(block_type.clone(), ctx);
                });
            }
            ModelEvent::BackgroundBlockStarted => {
                // For now, this event is only used for telemetry. It may also
                // be useful to request attention if the user's session starts
                //receiving background output, or to auto-scroll it.
            }
            ModelEvent::PreInteractiveSSHSession => {}
            ModelEvent::SSH(remote_shell) => {
                if let Some(shell) = ShellType::from_name(remote_shell) {
                    if shell.is_fully_supported_remotely() {
                        // Start a bootstrap timer for the SSH session, so we can log when the session
                        // takes too long to initialize
                        self.start_bootstrap_timer(BOOTSTRAP_FAILED_DURATION, ctx);
                    }
                }
            }
            ModelEvent::SSHControlMasterError => {
                self.handle_control_master_error(ctx);
            }
            ModelEvent::BlockMetadataReceived(block_metadata_received_event) => {
                let block_metadata = &block_metadata_received_event.block_metadata;

                // In-band commands don't change the CWD, git state, or session
                // metadata. Skip the expensive processing (git repo detection,
                // directory indexing, re-renders) to avoid an infinite loop where
                // a re-render triggers completions which fire another in-band
                // command. See also the complementary guard in
                // Input::set_active_block_metadata.
                if block_metadata_received_event.is_after_in_band_command {
                    self.active_block_metadata = Some(block_metadata.clone());
                    self.input.update(ctx, |view, ctx| {
                        view.set_active_block_metadata(
                            block_metadata.clone(),
                            true, // is_after_in_band_command
                            ctx,
                        );
                    });
                    return;
                }

                if let Some(prev_block_metadata) = self.active_block_metadata.take() {
                    // Only send event to save app state when the block is post bootstrap
                    // and working directory has changed.
                    if prev_block_metadata.current_working_directory()
                        != block_metadata.current_working_directory()
                        && block_metadata_received_event.is_done_bootstrapping
                    {
                        ctx.emit(Event::AppStateChanged);
                    }

                    // Update the shell launch data for the active session.
                    if prev_block_metadata.session_id() != block_metadata.session_id()
                        && block_metadata_received_event.is_done_bootstrapping
                    {
                        let shell_launch_data = self.shell_launch_data_if_local(ctx);
                        self.on_active_shell_launch_data_updated(shell_launch_data, ctx);
                    }

                    // Check if the block is done bootstrapping and the directory is set.
                    #[cfg(feature = "local_fs")]
                    if let Some(active_directory) = block_metadata_received_event
                        .block_metadata
                        .current_working_directory()
                    {
                        // Check if we need to index the directory
                        if block_metadata_received_event.is_done_bootstrapping {
                            #[cfg(feature = "local_fs")]
                            {
                                // Convert the shell-native CWD (e.g. "/c/Users/..." for
                                // Git Bash/MSYS2) to a Windows-native path before passing
                                // it to repo detection, which requires an OS-native path.
                                let native_directory = block_metadata_received_event
                                    .block_metadata
                                    .session_id()
                                    .and_then(|session_id| {
                                        self.sessions.as_ref(ctx).get(session_id)
                                    })
                                    .and_then(|session| {
                                        session.launch_data().and_then(|data| {
                                            data.maybe_convert_absolute_path(active_directory)
                                        })
                                    })
                                    .map(|path| path.to_string_lossy().into_owned());
                                let directory_for_detection =
                                    native_directory.as_deref().unwrap_or(active_directory);

                                let fut = DetectedRepositories::handle(ctx).update(
                                    ctx,
                                    |updater, ctx| {
                                        updater.detect_possible_git_repo(
                                            directory_for_detection,
                                            RepoDetectionSource::TerminalNavigation,
                                            ctx,
                                        )
                                    },
                                );

                                ctx.spawn(fut, move |me, repo_path_opt, ctx| {
                                    let old_repo_path = me.current_repo_path.clone();
                                    // Update the current repo path
                                    me.current_repo_path = repo_path_opt.clone();

                                    // Notify the pane group that the detected repo
                                    // changed so the code review panel can
                                    // (re-)initialize with the correct context.
                                    if old_repo_path != me.current_repo_path {
                                        ctx.emit(Event::Pane(PaneEvent::RepoChanged));
                                    }

                                    let Some(active_directory) =
                                        me.active_session_path_if_local(ctx)
                                    else {
                                        me.clear_git_repo_status(ctx);
                                        return;
                                    };

                                    if let Some(repo_path) = &repo_path_opt {
                                        let Ok(active_directory) =
                                            repo_metadata::CanonicalizedPath::try_from(
                                                active_directory,
                                            )
                                        else {
                                            return;
                                        };

                                        // Make sure the repo path is still an ancestor of the active directory.
                                        let is_ancestor = active_directory
                                            .as_path_buf()
                                            .ancestors()
                                            .any(|ancestor| ancestor == repo_path.as_path());
                                        if !is_ancestor {
                                            return;
                                        }

                                        // Subscribe to GitRepoStatusModel if the repo changed
                                        // and git status updates are needed.
                                        if old_repo_path.as_ref() != Some(repo_path) {
                                            // Drop old handle (unsubscribes automatically).
                                            me.git_repo_status = None;
                                            me.update_git_status_subscription(ctx);
                                        }

                                        // Notify chips of the new repo path.
                                        me.input.update(ctx, |input, ctx| {
                                            input.update_repo_path(Some(repo_path.clone()), ctx);
                                        });
                                    } else {
                                        me.clear_git_repo_status(ctx);
                                        ctx.notify();
                                    }
                                });
                            }
                        }
                    }
                }

                self.active_block_metadata = Some(block_metadata.clone());

                if let Some(session) = block_metadata
                    .session_id()
                    .and_then(|id| self.sessions.as_ref(ctx).get(id))
                {
                    let shell_host = ShellHost::from_session(session.as_ref());
                    self.model
                        .lock()
                        .block_list_mut()
                        .set_active_shell_host(shell_host);
                }

                self.input.update(ctx, |view, ctx| {
                    view.set_active_block_metadata(
                        block_metadata.clone(),
                        block_metadata_received_event.is_after_in_band_command,
                        ctx,
                    );
                    // Now that we've received the metadata for the active block, redraw the
                    // prompt area so it's up to date.
                    ctx.notify();
                });
            }
            ModelEvent::TerminalModeSwapped(mode) => {
                #[cfg(feature = "local_tty")]
                {
                    let active_command = self
                        .model
                        .lock()
                        .block_list()
                        .active_block()
                        .top_level_command(self.sessions.as_ref(ctx));
                    if FeatureFlag::RemoveAltScreenPadding.is_enabled()
                        // If we don't know what the top-level command is,
                        // we should still perform the redundant resize.
                        && active_command.is_none_or(|cmd| {
                            !ALT_SCREEN_APPS_THAT_MUST_MATCH_BLOCKLIST_PADDING
                                .contains(cmd.as_str())
                        })
                    {
                        // Since the alt-screen and blocklist have different sizes,
                        // let's make sure to refresh the winsize when switching
                        // back and forth between these modes.
                        self.refresh_size(ctx);

                        if matches!(mode, TerminalMode::AltScreen)
                            && matches!(
                                *TerminalSettings::as_ref(ctx).alt_screen_padding,
                                AltScreenPaddingMode::Custom { .. }
                            )
                        {
                            // Redundantly send resizes in case the alt-screens
                            // resize handler was not registered in time.
                            self.resize_alt_screen_redundantly(ctx);
                        }
                    }
                }

                let existing_find_options = match mode {
                    TerminalMode::AltScreen => self
                        .find_model
                        .as_ref(ctx)
                        .block_list_find_run()
                        .map(|run| run.options()),
                    TerminalMode::BlockList => self
                        .find_model
                        .as_ref(ctx)
                        .alt_screen_find_run()
                        .map(|run| run.options()),
                }
                .cloned();
                if let Some(FindOptions {
                    query: Some(query),
                    is_regex_enabled,
                    is_case_sensitive,
                    ..
                }) = existing_find_options
                {
                    // If there was an active find in the old mode, preserve and re-run the same
                    // query in the new mode.
                    self.find_model.update(ctx, |find_model, ctx| {
                        find_model.run_find(
                            FindOptions {
                                query: Some(query),
                                is_regex_enabled,
                                is_case_sensitive,
                                ..Default::default()
                            },
                            ctx,
                        );
                    });
                }

                // Close the find bar across the screen transition.
                // We don't want to change focus unnecessarily, e.g. when
                // using synced inputs and exiting `vim`.
                if self.find_model.as_ref(ctx).is_find_bar_open() {
                    self.close_find_bar(ctx);
                    self.redetermine_global_focus(ctx);
                }
            }
            ModelEvent::TmuxControlModeReady { .. } => {
                self.trigger_subshell_bootstrap(None, ctx);
            }
            ModelEvent::DetectedEndOfSshLogin(check_type) => {
                self.handle_detected_end_of_ssh_login(check_type, ctx);
            }
            ModelEvent::RemoteWarpificationIsUnavailable(reason) => {
                self.handle_remote_warpification_is_unavailable(reason.clone(), ctx);
            }
            ModelEvent::SshTmuxInstaller(tmux_installation) => {
                self.warpify_state
                    .set_tmux_installation_state(*tmux_installation);
            }
            ModelEvent::TmuxInstallFailed { line, command } => {
                let system_details = self
                    .warpify_state
                    .ssh_block_state()
                    .and_then(|s| s.get_system_details(ctx));
                self.warpify_state.abort_ssh_warpify_timeout();
                self.add_ssh_error_block(
                    WarpificationUnavailableReason::TmuxInstallFailed {
                        system_details,
                        line: Some(line.to_string()),
                        command: Some(command.to_string()),
                    },
                    ctx,
                );
            }
            ModelEvent::ExecutedInBandCommand(event) => {
                // TODO(vorporeal): Figure out a way to not need the terminal view involved
                // in this flow.
                let active_session_id = self.active_block_session_id();
                if let Some(active_session_id) = active_session_id {
                    self.sessions.update(ctx, |sessions, _ctx| {
                        sessions.handle_executed_command_event(active_session_id, event.clone());
                    });
                }
            }
            ModelEvent::InitSubshell(event) => {
                let shell_type = event.shell_type;
                self.trigger_subshell_bootstrap(Some(shell_type), ctx);
            }
            ModelEvent::InitSsh(event) => {
                let shell_type = event.shell_type;
                let uname = event.uname.as_ref().unwrap_or(&String::default()).clone();
                self.continue_warpify_ssh_session(&uname, shell_type, ctx);
            }
            ModelEvent::SourcedRcFileInSubshell(event) => {
                let shell_type = event.shell_type;
                let uname = event.uname.clone();
                let disable_tmux = event.tmux == Some(false);

                ctx.spawn(
                    async {
                        warpui::r#async::Timer::after(*TRIGGER_RC_FILE_SUBSHELL_BOOTSTRAP_DELAY)
                            .await
                    },
                    move |me, _, ctx| {
                        let uname = uname.to_owned().unwrap_or_default();
                        let (is_ssh, is_tmux_control_mode_active) = {
                            let model = me.model.lock();
                            (model.is_ssh_block(), model.tmux_control_mode_active())
                        };
                        // To simplify the implementation, we do not support warpifying while SSH-warpified.
                        if is_tmux_control_mode_active {
                            return;
                        }
                        if is_ssh && !disable_tmux {
                            me.continue_warpify_ssh_session(&uname, shell_type, ctx);
                        } else {
                            me.trigger_subshell_bootstrap(Some(shell_type), ctx);
                        }
                    },
                );
            }
            ModelEvent::PromptUpdated => {
                self.input.update(ctx, |input, ctx| {
                    input.notify_and_notify_children(ctx);
                });
            }
            ModelEvent::HonorPS1OutOfSync => {}
            ModelEvent::Typeahead => {
                self.handle_typeahead_event(ctx);
            }
            ModelEvent::Handler(AnsiHandlerEvent::InitShell {
                pending_session_info,
            }) => {
                // The remote confirmed a subshell bootstrap is starting. Hide the
                // original long-running block now so the user doesn't see the
                // bootstrap payload echoed into it.
                if pending_session_info.subshell_info.is_some() {
                    let show_debug_block = BlockVisibilitySettings::as_ref(ctx)
                        .should_show_ssh_block
                        .value();
                    if !show_debug_block {
                        self.update_long_running_ssh_block_with_lock(|block| block.hide());
                    }
                }
            }
            ModelEvent::Handler(_) => {}
            ModelEvent::SelectedTextChanged => {
                ctx.emit(Event::SelectedTextChanged);
            }
            ModelEvent::ShellSpawned(shell_type) => {
                ctx.emit(Event::ShellSpawned(*shell_type));
                ctx.notify();
            }
            ModelEvent::CompletionsFinished(_data) => {}
            ModelEvent::SendCompletionsPrompt => {}
            ModelEvent::ImageReceived {
                image_id,
                image_data,
                ..
            } => {
                AssetCache::handle(ctx).update(ctx, |asset_cache, ctx| {
                    asset_cache.insert_raw_asset_bytes::<ImageType>(
                        image_id.to_string(),
                        &image_data[..],
                        ctx,
                    );
                });
                ctx.notify();
            }
            ModelEvent::BootstrapPrecmdDone => {
                self.execute_pending_command((), ctx);
            }

            ModelEvent::PluggableNotification { title, body } => {
                // Intercept structured CLI agent notifications (e.g. from Claude Code plugin).
                // The listener's own subscription handles subsequent events; we just
                // suppress the raw JSON from becoming a toast/desktop notification.
                if title.as_deref() == Some(CLI_AGENT_NOTIFICATION_SENTINEL) {
                    self.handle_cli_agent_notification(title.as_deref(), body, ctx);
                    return;
                }

                // Suppress OSC 9 notifications when a Codex listener is active.
                // The listener's subscription handles these via CodexSessionHandler.
                if title.is_none() {
                    let has_codex_listener = CLIAgentSessionsModel::as_ref(ctx)
                        .session(self.view_id)
                        .is_some_and(|s| s.agent == CLIAgent::Codex && s.listener.is_some());
                    if has_codex_listener {
                        return;
                    }
                }

                if self.is_navigated_away_from_window(ctx) {
                    let notification_title =
                        title.clone().unwrap_or_else(|| "Notification".to_string());
                    let notification = BlockNotification {
                        title: notification_title,
                        body: body.clone(),
                    };
                    ctx.emit(Event::SendNotification(notification));
                } else {
                    ctx.emit(Event::PluggableNotification {
                        title: title.clone(),
                        body: body.clone(),
                    });
                }
            }
            ModelEvent::ExitShell { .. } => {}
        }
    }

    /// Handles an OSC 777 event with the `warp://cli-agent` sentinel title.
    /// On `session_start`, creates a `CLIAgentSessionListener` that subscribes
    /// to subsequent events from this terminal's PTY.
    fn handle_cli_agent_notification(
        &mut self,
        title: Option<&str>,
        body: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(notification) = parse_event(title, body) else {
            return;
        };

        if !is_agent_supported(&notification.agent) {
            return;
        }
        if !self.register_cli_agent_listener_from_event(&notification, ctx) {
            return;
        }

        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions_model, ctx| {
            sessions_model.update_from_event(self.view_id, &notification, ctx);
        });

        if notification.event == CLIAgentEventType::SessionStart {
            self.maybe_auto_open_cli_agent_rich_input(ctx);
        }
    }

    fn register_cli_agent_listener_from_event(
        &mut self,
        notification: &CLIAgentEvent,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        if !is_agent_supported(&notification.agent) {
            return false;
        }
        let has_listener = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .is_some_and(|s| s.listener.is_some());
        if has_listener {
            return false;
        }

        let model_events_handle = self.model_events_handle.clone();
        let view_id = self.view_id;
        let agent = notification.agent;
        let listener = ctx.add_model(|ctx| {
            CLIAgentSessionListener::new(view_id, agent, &model_events_handle, ctx)
        });
        let remote_host = self.active_session_remote_host(ctx);
        let should_auto_toggle_input =
            *CLIAgentSettings::as_ref(ctx).auto_open_rich_input_on_cli_agent_start;
        // Seed context from the event that caused registration before the
        // listener subscribes to future events.
        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions_model, ctx| {
            sessions_model.register_listener(
                view_id,
                agent,
                notification.cwd.clone(),
                notification.project.clone(),
                notification.session_id.clone(),
                notification.payload.plugin_version.clone(),
                remote_host,
                should_auto_toggle_input,
                listener,
                ctx,
            );
        });
        true
    }

    /// Creates and registers a listener for flows without a `SessionStart` event.
    fn register_cli_agent_listener_without_session_start_event(
        &mut self,
        agent: CLIAgent,
        ctx: &mut ViewContext<Self>,
    ) {
        // No SessionStart event in this path (mid-session install/update).
        // Assume the just-installed plugin meets the minimum version for this agent
        // so the update chip doesn't flash before the user runs /reload-plugins.
        let plugin_version =
            plugin_manager_for(agent).map(|m| m.minimum_plugin_version().to_owned());
        let notification = CLIAgentEvent {
            v: 1,
            agent,
            event: CLIAgentEventType::SessionStart,
            session_id: None,
            cwd: None,
            project: None,
            payload: CLIAgentEventPayload {
                plugin_version,
                ..Default::default()
            },
        };
        if self.register_cli_agent_listener_from_event(&notification, ctx) {
            self.maybe_auto_open_cli_agent_rich_input(ctx);
        }
    }

    /// If the startup auto-open setting is enabled, auto-opens rich input for a
    /// CLI agent session. Called after creating a command-detected session or
    /// registering a listener so rich input is shown immediately.
    fn maybe_auto_open_cli_agent_rich_input(&mut self, ctx: &mut ViewContext<Self>) {
        let cli_settings = CLIAgentSettings::as_ref(ctx);
        if !*cli_settings.auto_open_rich_input_on_cli_agent_start
            || !*cli_settings.should_render_cli_agent_footer
            || !is_rich_input_chip_in_cli_toolbar(ctx)
        {
            return;
        }
        let should_open = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .is_some_and(|s| s.should_auto_toggle_input);
        if should_open && !self.has_active_cli_agent_input_session(ctx) {
            self.open_cli_agent_rich_input(CLIAgentInputEntrypoint::AutoShow, ctx);
        }
    }

    /// Handles CLI agent session status changes from the singleton model.
    /// Sends a desktop notification when a CLI agent reaches a completed state
    /// (blocked or succeeded) and the user is in a different window.
    /// Also handles auto-show/hide of CLI agent rich input based on the
    /// `auto_toggle_rich_input` setting: closes rich input when blocked
    /// (agent requires keyboard interaction) and opens it when the agent resumes.
    fn handle_cli_agent_sessions_event(
        &mut self,
        event: &CLIAgentSessionsModelEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            CLIAgentSessionsModelEvent::Started {
                terminal_view_id, ..
            } if *terminal_view_id == self.view_id => {
                if FeatureFlag::TrimTrailingBlankLines.is_enabled() {
                    self.model
                        .lock()
                        .block_list_mut()
                        .active_block_mut()
                        .set_trim_trailing_blank_rows(true);
                }
            }
            CLIAgentSessionsModelEvent::Ended {
                terminal_view_id, ..
            } if *terminal_view_id == self.view_id => {
                if FeatureFlag::TrimTrailingBlankLines.is_enabled() {
                    self.model
                        .lock()
                        .block_list_mut()
                        .active_block_mut()
                        .set_trim_trailing_blank_rows(false);
                }
            }
            _ => {}
        }
        if event.terminal_view_id() == self.view_id
            && matches!(
                event,
                CLIAgentSessionsModelEvent::Started { .. }
                    | CLIAgentSessionsModelEvent::StatusChanged { .. }
                    | CLIAgentSessionsModelEvent::SessionUpdated { .. }
                    | CLIAgentSessionsModelEvent::Ended { .. }
            )
        {
            self.update_pane_configuration(ctx);
            ctx.notify();
        }

        let CLIAgentSessionsModelEvent::StatusChanged {
            terminal_view_id,
            agent,
            status,
            session_context,
        } = event
        else {
            return;
        };

        if *terminal_view_id != self.view_id {
            return;
        }

        let cli_settings = CLIAgentSettings::as_ref(ctx);
        if *cli_settings.auto_toggle_rich_input
            && *cli_settings.should_render_cli_agent_footer
            && is_rich_input_chip_in_cli_toolbar(ctx)
        {
            let should_auto_toggle_input = CLIAgentSessionsModel::as_ref(ctx)
                .session(self.view_id)
                .is_some_and(|s| s.listener.is_some() && s.should_auto_toggle_input);
            if should_auto_toggle_input {
                match status {
                    CLIAgentSessionStatus::Blocked { .. } => {
                        // Auto-close rich input when the agent is blocked
                        // (it requires direct keyboard interaction in the terminal).
                        self.close_cli_agent_rich_input(ctx);
                    }
                    CLIAgentSessionStatus::InProgress | CLIAgentSessionStatus::Success => {
                        // Auto-open rich input when the agent resumes or completes.
                        if !self.has_active_cli_agent_input_session(ctx) {
                            self.open_cli_agent_rich_input(CLIAgentInputEntrypoint::AutoShow, ctx);
                        }
                    }
                }
            }
        }

        // Desktop notifications — only when navigated away and not in-progress.
        if !self.is_navigated_away_from_window(ctx)
            || matches!(status, CLIAgentSessionStatus::InProgress)
        {
            return;
        }

        let title = session_context
            .query
            .as_deref()
            .filter(|q| !q.is_empty())
            .or(session_context.summary.as_deref().filter(|s| !s.is_empty()))
            .unwrap_or(agent.command_prefix())
            .to_owned();
        let description = if let CLIAgentSessionStatus::Blocked { message } = status {
            message.clone().unwrap_or_default()
        } else {
            session_context.response.clone().unwrap_or_default()
        };

        let trigger = if matches!(status, CLIAgentSessionStatus::Blocked { .. }) {
            NotificationsTrigger::NeedsAttention
        } else {
            NotificationsTrigger::AgentTaskCompleted(true)
        };
        self.send_cli_agent_desktop_notification_or_show_banner(trigger, title, description, ctx);
    }

    /// Handles the initialization of a session within this terminal pane.
    ///
    /// This does not indicate that the session has bootstrapped, but only
    /// that we're aware of the beginning of a session that we will attempt
    /// to bootstrap.
    fn handle_session_initialized(&self, ctx: &mut ViewContext<Self>) {
        // Make sure we re-render the input so we're displaying an appropriate
        // prompt.
        self.input.update(ctx, |_, ctx| {
            ctx.notify();
        });
    }

    /// Handles a session in this terminal pane completing the bootstrapping
    /// process.
    fn handle_session_bootstrapped(
        &mut self,
        bootstrap_event: SessionBootstrappedEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        let session_id = bootstrap_event.session_id;
        let Some(session) = self.sessions.as_ref(ctx).get(session_id) else {
            log::error!(
                "Could not find session {session_id:?} in sessions model after \
                         being notified that the session had bootstrapped!"
            );
            return;
        };

        // Ensure that the new session's working directory and environment are persisted.
        ctx.dispatch_global_action("workspace:save_app", ());

        self.update_incompatible_configuration_banner(session.shell().plugins(), ctx);

        if let Some(subshell_info) = session.subshell_info() {
            self.warpify_state
                .add_subshell_separator(subshell_info, self.model.clone(), ctx);
        }

        self.is_login_shell_bootstrapped = true;
        self.hide_slow_bootstrap_banner(ctx);

        if self.should_display_vim_banner(&session, ctx) {
            self.insert_vim_mode_banner(ctx);
        }

        if let Some(env_var_collection) = self.pending_env_var_collection.take() {
            self.invoke_environment_variables(env_var_collection, false, ctx);
        }

        // Make sure we decorate any text that is already in the input.  We
        // need to make sure external commands have finished loading before
        // doing the decoration to ensure we don't erroneously apply error
        // underlines to valid commands.
        let input = self.input().clone();
        ctx.spawn(
            async move { session.load_external_commands().await },
            move |me, _, ctx| {
                input.update(ctx, |input, ctx| {
                    input.run_input_background_jobs(
                        InputBackgroundJobOptions::default().with_command_decoration(),
                        ctx,
                    );
                });
                me.refresh_warp_prompt(ctx);
            },
        );

        // If we were waiting for a successful warpification, it's come. Stop the timeout.
        self.warpify_state.abort_ssh_warpify_timeout();

        if bootstrap_event.subshell_info.is_some() {
            self.add_bootstrap_success_block(bootstrap_event, ctx);
        }
        self.any_session_contains_restored_remote_blocks = self.contains_restored_remote_blocks();
        self.any_session_contains_remote_blocks |= self.active_block_is_considered_remote(ctx);

        self.update_pane_configuration(ctx);
        self.ignore_next_set_title_event = true;

        self.refresh_warp_prompt(ctx);
        ctx.emit(Event::SessionBootstrapped);
    }

    fn should_display_vim_banner(
        &self,
        session: &Arc<Session>,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        // Is this the active session?
        // We should only show the vim keybindings banner in one place at a time.
        if !self.is_active_session(ctx) {
            return false;
        }

        // Is the vim keybindings banner already open or dismissed?
        let vim_banner_displayed = self.inline_banners_state.vim_banner_state.is_some()
            || VimBannerSettings::handle(ctx).read(ctx, |banner_settings, _| {
                *banner_settings.vim_keybindings_banner_state == BannerState::Dismissed
            });

        // Have we already enabled vim keybindings?
        let vim_keybindings_enabled = AppEditorSettings::handle(ctx)
            .read(ctx, |editor_settings, _| editor_settings.vim_mode_enabled());

        if vim_banner_displayed || vim_keybindings_enabled {
            return false;
        }

        // Have we detected that vim keybindings may be wanted?
        let vi_mode_in_plugins = session.shell().plugins().contains("vi");
        let vi_mode_in_opts = session
            .shell()
            .options()
            .to_owned()
            .unwrap_or_default()
            .contains("vi_mode");

        vi_mode_in_plugins || vi_mode_in_opts
    }

    #[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
    fn add_settings_import_block(&mut self, ctx: &mut ViewContext<Self>) {
        let current_block_view_handle = ctx.add_typed_action_view(SettingsImportView::new);
        self.settings_import_block = Some(current_block_view_handle.clone());

        ctx.subscribe_to_view(
            &current_block_view_handle,
            move |terminal_view, settings_import_view_handle, event, _ctx| match event {
                SettingsImportEvent::Completed(true) => {
                    terminal_view.settings_import_block = None;
                }
                SettingsImportEvent::NoConfigsFound => {
                    // In the case where no settings were found to import, we want to remove the settings import block.
                    terminal_view
                        .model
                        .lock()
                        .block_list_mut()
                        .remove_rich_content(settings_import_view_handle.id());

                    terminal_view.settings_import_block = None;
                }
                _ => {
                    terminal_view.settings_import_block = None;
                }
            },
        );

        self.insert_rich_content(
            None,
            current_block_view_handle,
            None,
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: false,
            },
            ctx,
        );
    }

    pub fn interrupt_settings_import_block(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(settings_import_block_handle) = &self.settings_import_block {
            settings_import_block_handle.update(ctx, |settings_import_view, ctx| {
                settings_import_view.interrupt_block(ctx);
            })
        }

        self.settings_import_block = None;
    }

    /// Opens a folder that the user may or may not have opened in the past
    pub fn open_repo_folder(&mut self, path: String, ctx: &mut ViewContext<Self>) {
        let escaped = self.shell_family(ctx).shell_escape(&path);
        self.input.update(ctx, |input, ctx| {
            input.try_execute_command(&format!("cd {escaped}"), ctx);
        });

        self.toggle_left_panel_file_tree(true, ctx);
    }

    /// Gets the selected text from the terminal, if any.
    pub fn selected_text(&self, ctx: &AppContext) -> Option<String> {
        let semantic_selection = SemanticSelection::handle(ctx).as_ref(ctx);
        let input_mode = *InputModeSettings::handle(ctx)
            .as_ref(ctx)
            .input_mode
            .value();
        let inverted = input_mode.is_inverted_blocklist();
        self.model
            .lock()
            .selection_to_string(semantic_selection, inverted, ctx)
    }

    /// Gets the selected text from the terminal input editor, if any.
    pub fn selected_text_from_input(&self, ctx: &AppContext) -> Option<String> {
        let text = self
            .input
            .as_ref(ctx)
            .editor()
            .as_ref(ctx)
            .selected_text(ctx);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

impl TerminalView {
    // Redundantly issues resize changes to increase the chances that the alt-screen program
    // gets the latest winsize when it has a resize handler setup.
    //
    // When the alt-screen is activated, the terminal size changes because the blocklist has
    // padding that the alt-screen doesn't. So we do this to avoid a race condition between
    // (1) resizing right after swapping terminal modes, and
    // (2) the alt-screen app registering its resize handler
    #[cfg(feature = "local_tty")]
    fn resize_alt_screen_redundantly(&mut self, ctx: &mut ViewContext<Self>) {
        use futures_lite::StreamExt;

        // Resize twice, half a second apart.
        ctx.spawn_stream_local(
            async_io::Timer::interval(Duration::from_millis(500)).take(2),
            |view, _, ctx| {
                let model = view.model.lock();

                // If the alt-screen was exited since the timer expired,
                // there's nothing to do.
                if !model.is_alt_screen_active() {
                    return;
                }

                let correct_size_info = *view.size_info;
                let active_command = model
                    .block_list()
                    .active_block()
                    .top_level_command(view.sessions.as_ref(ctx));

                // Drop the lock since the resize methods will take an explicit lock.
                drop(model);

                // This is a workaround for alt-screen programs that _cache_ resizes during init
                // but don't actually redraw the contents. For example, we've seen this happen with
                // certain emacs setups. So we fake a winsize before immediately correcting it to
                // invalidate that cache.
                if active_command
                    .is_some_and(|cmd| ALT_SCREEN_APPS_WITH_RESIZE_PROBLEMS.contains(cmd.as_str()))
                {
                    let mut wrong_size_info = *view.size_info;
                    wrong_size_info.pane_width_px += 1.;
                    view.resize_internal(
                        SizeUpdateBuilder::for_refresh(wrong_size_info).build(view, ctx),
                        ctx,
                    );
                }

                // Send the resize as a refresh to force a size update.
                view.resize_internal(
                    SizeUpdateBuilder::for_refresh(correct_size_info).build(view, ctx),
                    ctx,
                );
            },
            |_, _| {},
        );
    }

    async fn fetch_command_corrections(
        block: UserBlockCompleted,
        session: Option<Arc<Session>>,
        history_commands: Vec<HistoryEntry>,
    ) -> Vec<Correction> {
        // Create the command
        let (input, output, exit_code, working_dir) = (
            block.command.as_str(),
            block.output_truncated.as_str(),
            block.serialized_block.exit_code,
            block.serialized_block.pwd.as_ref(),
        );

        let mut command = Command::new(input, output, exit_code.into());
        if let Some(working_dir) = working_dir {
            command = command.set_working_dir(working_dir);
        }

        // Create the session metadata
        // TODO: we need to figure out how to avoid re-creating this
        // for every single invocation of correct_command.
        let mut session_metadata = SessionMetadata::new();

        session_metadata.set_history(history_commands.iter().filter_map(|s| {
            Some(HistoryItem::new(
                s.command.as_str(),
                s.exit_code?,
                s.pwd.as_ref()?.as_str(),
            ))
        }));

        let mut git_branches = None;
        if let Some(session) = &session {
            session_metadata.set_session_type(session.session_type().clone().into());
            let shell = session.shell();
            session_metadata.set_shell(shell.shell_type().into(), shell.version().as_deref());
            session_metadata.set_aliases(session.alias_names());
            session_metadata.set_executables(session.executable_names());
            session_metadata.set_functions(session.function_names());
            session_metadata.set_builtins(session.builtin_names());
            session_metadata.set_platform_type(session.host_info().platform_type());
            if let Some(working_dir) = working_dir {
                git_branches = Some(
                    session
                        .git_branches_for_command_corrections(working_dir)
                        .await,
                );
            }
        }
        session_metadata.set_git_branches(git_branches.iter().flatten().map(|s| s.as_str()));

        // https://github.com/warpdotdev/command-corrections/blob/df7848d4fb3da7883623e959889a296a07d88053/src/rules/cd/mod.rs#L31-L36
        // We don't currently support dynamic rules over SSH, so we should not attempt to correct commands if
        // inside ssh session.
        let is_ssh_command = SshWarpifyCommand::matches(input).is_some();
        if is_ssh_command {
            return vec![];
        }
        if FeatureFlag::CommandCorrectionsHistoryRule.is_enabled() {
            correct_command(command, &session_metadata, std::iter::empty())
        } else {
            correct_command(
                command,
                &session_metadata,
                DEFAULT_IGNORED_RULES_FOR_COMMAND_CORRECTIONS.into_iter(),
            )
        }
    }

    fn write_init_subshell_bytes_to_pty(
        &mut self,
        shell_type: Option<ShellType>,
        ctx: &mut ViewContext<Self>,
    ) {
        let session_id = crate::terminal::bootstrap::generate_session_id();
        self.model.lock().register_session_id(session_id);
        self.clear_line_editor_and_write_to_pty(
            init_subshell_command(shell_type, &self.env_vars, session_id, ctx).into_bytes(),
            ctx,
        );
        self.write_to_pty(vec![escape_sequences::C0::CR], ctx);
    }

    /// If a command correction exists, generate the command correction banner.
    fn after_command_correction_generation(
        &mut self,
        corrections: Vec<Correction>,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        if let Some(correction) = corrections.into_iter().next() {
            let _rule = correction.rule_applied;

            // Set the autosuggestion only if the input is still empty
            self.input.update(ctx, |input, ctx| {
                if input.buffer_text(ctx).is_empty() {
                    input.set_autosuggestion(correction.command.as_str(), ctx);
                }
            });

            let a11y_content = AccessibilityContent::new(
                format!("Suggested corrected command: {}", correction.command),
                "Press right arrow to insert or keep editing to ignore",
                WarpA11yRole::HelpRole,
            );
            ctx.emit_a11y_content(a11y_content);

            self.most_recent_command_correction = Some(correction);

            ctx.notify();
        }
    }

    pub(crate) fn remove_plugin_instructions_block(
        &mut self,
        block_handle: ViewHandle<plugin_instructions_block::PluginInstructionsBlock>,
        ctx: &mut ViewContext<Self>,
    ) {
        let block_id = block_handle.id();
        self.rich_content_views
            .retain(|rich_content| rich_content.view_id() != block_id);
        self.model
            .lock()
            .block_list_mut()
            .remove_rich_content(block_id);
        ctx.notify();
    }

    /// Generates command corrections, if applicable.
    fn maybe_generate_command_corrections(
        &mut self,
        block_completed: &UserBlockCompleted,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        let block_completed = block_completed.to_owned();

        if *InputSettings::as_ref(ctx).command_corrections.value() {
            let session_id = self.active_block_session_id();

            let session = session_id.and_then(|id| self.sessions.as_ref(ctx).get(id));
            let history_entries = session_id
                .and_then(|id| History::as_ref(ctx).commands(id))
                .into_iter()
                .flatten()
                .cloned()
                .collect();

            let _ = ctx.spawn(
                Self::fetch_command_corrections(block_completed.clone(), session, history_entries),
                Self::after_command_correction_generation,
            );
        }
    }

    fn can_suggest_alias_expansion(&mut self, ctx: &mut ViewContext<TerminalView>) -> bool {
        let has_user_seen_banner: bool = ctx
            .private_user_preferences()
            .read_value(ALIAS_EXPANSION_BANNER_SEEN_KEY)
            .unwrap_or_default()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(false);
        let alias_expansion_settings = AliasExpansionSettings::as_ref(ctx);
        let supported_on_current_platform = alias_expansion_settings
            .alias_expansion_enabled
            .is_supported_on_current_platform();
        let is_fish_shell = self
            .active_block_session_id()
            .and_then(|id| self.sessions.as_ref(ctx).get(id))
            .is_some_and(|s| s.shell().shell_type() == ShellType::Fish);

        supported_on_current_platform
            && !*alias_expansion_settings.alias_expansion_enabled
            && !has_user_seen_banner
            // We don't suggest alias expansions for fish since we already expand
            // abbreviations by default.
            && !is_fish_shell
    }

    fn maybe_suggest_alias_expansion(
        &mut self,
        block_completed: &UserBlockCompleted,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        if let Some(session) = self
            .active_block_session_id()
            .and_then(|id| self.sessions.as_ref(ctx).get(id))
        {
            let command = block_completed.command.clone();
            ctx.spawn(
                async move { check_for_alias_async(&command, session).await },
                move |view, aliased_command, ctx| {
                    view.suggest_alias_expansion(aliased_command, ctx);
                },
            );
        }
    }

    fn suggest_alias_expansion(
        &mut self,
        aliased_command: Option<AliasedCommand>,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        if let Some(aliased_command) = aliased_command {
            let _ = ctx
                .private_user_preferences()
                .write_value(ALIAS_EXPANSION_BANNER_SEEN_KEY, "true".to_owned());
            self.insert_alias_expansion_banner(aliased_command, ctx);
        }
    }

    fn maybe_send_block_completed_notification(
        &mut self,
        block: &UserBlockCompleted,
        block_duration: Duration,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        let session_settings_handle = SessionSettings::as_ref(ctx);

        // If notifications are not enabled on this platform, we don't want to
        // send notifications or show any of the notification-related banners.
        if !session_settings_handle
            .notifications
            .is_supported_on_current_platform()
        {
            return;
        }

        let notification_settings = session_settings_handle.notifications.value().clone();
        let long_running_trigger = NotificationsTrigger::LongRunningCommand(
            !block.serialized_block.has_failed(),
            block_duration,
        );
        match notification_settings.mode {
            NotificationsMode::Unset => {
                if let NotificationsDiscoveryBanner::Triggered(trigger) =
                    self.inline_banners_state.notifications_discovery_banner
                {
                    // if the banner is not yet open, but there is some trigger,
                    // we were likely waiting on the block to finish so insert it now
                    self.insert_notifications_discovery_banner(trigger, ctx);
                } else if self.is_navigated_away_from_window(ctx)
                    && block_duration >= *DEFAULT_THRESHOLD_FOR_LONG_RUNNING_NOTIFICATION
                {
                    // otherwise, if the user is navigated away when the block completes
                    // and the block ran longer than the default for long-running notifications,
                    // insert a banner for the long running trigger
                    self.insert_notifications_discovery_banner(long_running_trigger, ctx);
                }
            }
            NotificationsMode::Enabled => {
                // If the notifications error is not open but was triggered,
                // we should insert a banner to surface the error
                if let NotificationsErrorBannerType::Triggered = self
                    .inline_banners_state
                    .notifications_error_banner
                    .banner_type
                {
                    self.insert_notifications_error_banner(ctx);
                } else if self.is_navigated_away_from_window(ctx)
                    && notification_settings.is_long_running_enabled
                    && block_duration >= notification_settings.long_running_threshold
                {
                    // Otherwise, since the block completed, check if we
                    // should send a notification for long-running command
                    let notification_content = long_running_trigger.create_notification_content(
                        block.command.clone(),
                        // Only include the last line when displaying a notification.
                        block
                            .output_truncated
                            .lines()
                            .last()
                            .map_or_else(String::new, ToOwned::to_owned),
                    );
                    ctx.emit(Event::SendNotification(notification_content));
                }
            }
            _ => {}
        }
    }

    /// Executes a command that was submitted by the user and not yet sent to the shell.
    fn send_cli_agent_desktop_notification_or_show_banner(
        &mut self,
        trigger: NotificationsTrigger,
        title: String,
        description: String,
        ctx: &mut ViewContext<Self>,
    ) {
        let notification_settings = SessionSettings::as_ref(ctx).notifications.value().clone();

        match notification_settings.mode {
            NotificationsMode::Unset => {
                if let NotificationsDiscoveryBanner::Triggered(trigger) =
                    self.inline_banners_state.notifications_discovery_banner
                {
                    // if the banner is not yet open, but there is some trigger,
                    // we were likely waiting on the block to finish so insert it now
                    self.insert_notifications_discovery_banner(trigger, ctx);
                } else {
                    // otherwise, insert a discovery banner for the current trigger
                    self.insert_notifications_discovery_banner(trigger, ctx);
                }
            }
            NotificationsMode::Enabled => {
                let success = matches!(trigger, NotificationsTrigger::AgentTaskCompleted(true));
                if success {
                    if !notification_settings.is_agent_task_completed_enabled {
                        return;
                    }
                } else if !notification_settings.is_needs_attention_enabled {
                    return;
                }
                let notification_content = trigger.create_notification_content(title, description);
                ctx.emit(Event::SendNotification(notification_content));
            }
            _ => {}
        }
    }

    pub fn execute_pending_command(&mut self, _: (), ctx: &mut ViewContext<Self>) {
        let had_pending = self.input.read(ctx, |input, _| input.has_pending_command());
        self.input.update(ctx, |input, ctx| {
            input.execute_pending_command(ctx);
        });
        // If the pending command was just consumed, track that we're waiting
        // for the resulting block to complete.
        if had_pending && !self.input.read(ctx, |input, _| input.has_pending_command()) {
            self.awaiting_pending_command_completion = true;
        }
    }

    // Try to execute the provided command. If we cannot execute it now, set it as the pending
    // command.
    //
    // If we set it as pending, the command will execute when we trigger another call to
    // `execute_pending_command` (either from a `BlockCompleted` or `BootstrapPrecmdDone` event)
    pub fn execute_command_or_set_pending(&mut self, command: &str, ctx: &mut ViewContext<Self>) {
        self.set_pending_command(command, ctx);
        self.execute_pending_command((), ctx);
    }

    fn hide_slow_bootstrap_banner(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_slow_bootstrap_banner_open {
            self.is_slow_bootstrap_banner_open = false;
            ctx.notify();
        }
    }
    pub(super) fn on_shell_determined(&self, ctx: &mut ViewContext<Self>) {
        self.start_bootstrap_timer(BOOTSTRAP_FAILED_DURATION, ctx);
    }

    pub fn is_login_shell_bootstrapped(&self) -> bool {
        self.is_login_shell_bootstrapped
    }
    pub fn has_pending_command_or_awaiting_completion(&self, ctx: &AppContext) -> bool {
        self.awaiting_pending_command_completion || self.input.as_ref(ctx).has_pending_command()
    }

    pub(super) fn on_pty_spawn_failed(
        &mut self,
        pty_spawn_error: anyhow::Error,
        ctx: &mut ViewContext<Self>,
    ) {
        self.pty_spawn_failed = true;
        self.insert_shell_process_terminated_banner(
            shell_terminated_banner::TerminationType::PtySpawnFailure { pty_spawn_error },
            ctx,
        );
        ctx.notify();
    }

    /// Start a timer so that we can detect when a session does not bootstrap in a timely manner
    fn start_bootstrap_timer(&self, duration: Duration, ctx: &mut ViewContext<Self>) {
        let _ = ctx.spawn(
            async move {
                warpui::r#async::Timer::after(duration).await;
            },
            Self::on_bootstrap_failed_timer_complete,
        );
    }

    /// Called once the bootstrap timer completes
    ///
    /// Will send telemetry if the current session is not bootstrapped and will show a banner to
    /// the user if this is the first bootstrap in the session.
    fn on_bootstrap_failed_timer_complete(&mut self, _: (), ctx: &mut ViewContext<Self>) {
        let (is_ssh, shell) = {
            let model = self.model.lock();

            // If we did actually bootstrap, or if the session is no longer usable
            // (e.g.: the shell process terminated), don't show a banner.
            if model.is_read_only() || model.is_active_block_bootstrapped() {
                return;
            }

            let is_ssh = model.has_pending_ssh_session();
            let shell = model
                .pending_shell_type()
                .map_or("unknown", |shell| shell.name());

            (is_ssh, shell)
        };

        log::warn!("Bootstrapping failed for shell {shell:?} on ssh {is_ssh}");

        // Unhide the long-running block that was hidden at the start of
        // subshell bootstrap so the user can see the session output again.
        self.update_long_running_ssh_block_with_lock(|block| {
            block.unhide();
        });

        // Send the bootstrapping slow event synchronously to ensure that we don't drop
        // the event if the user quits the app before the event queue is flushed and then
        // never reopens the app.

        if !self.is_login_shell_bootstrapped {
            log::warn!("Showing bootstrap slow toast");
            self.is_slow_bootstrap_banner_open = true;
            ctx.notify();
        }

        ctx.emit(Event::SlowBootstrap);
    }

    pub fn size_info(&self) -> &SizeInfo {
        &self.size_info
    }

    pub fn colors(&self) -> &color::List {
        &self.colors
    }

    pub fn override_colors(&self) -> color::OverrideList {
        let override_colors = self.model.lock().override_colors();
        override_colors
    }

    fn appearance<'a>(&self, ctx: &'a ViewContext<Self>) -> &'a Appearance {
        Appearance::as_ref(ctx)
    }

    fn refresh_size(&mut self, ctx: &mut ViewContext<Self>) {
        self.resize_internal(
            SizeUpdateBuilder::for_refresh(*self.size_info).build(self, ctx),
            ctx,
        )
    }

    fn resize_internal(&mut self, size_update: SizeUpdate, ctx: &mut ViewContext<Self>) {
        // If this isn't an actionable resize, there's nothing to do.
        if !(size_update.anything_changed() || size_update.is_refresh()) {
            return;
        }

        let new_size = size_update.new_size.pane_size_px();
        if new_size.x() == 0. || new_size.y() == 0. {
            log::info!("Tried to resize with size {new_size:?}. Skipping resize");
            return;
        }

        // Update model with new size info.
        self.model.lock().resize(size_update);
        self.find_model.update(ctx, |find_model, ctx| {
            find_model.rerun_find_on_active_grid(ctx);
        });
        // Resizing the model already clears selected text, but
        if size_update.rows_or_columns_changed() {
            self.clear_selected_text(ctx);
        }

        // Update view data with new size info.
        self.input.update(ctx, |view, ctx| {
            view.set_size_info(size_update.new_size, ctx);
            view.notify_and_notify_children(ctx);
        });

        *self.size_info = size_update.new_size;
        self.update_scroll_position_locking(ScrollPositionUpdate::AfterResize, ctx);

        // Notify subscribers.
        ctx.emit(Event::Resize { size_update });
    }

    /// This handler is called after *every* terminal view layout with the
    /// size of the entire terminal (block_list + input OR alt-grid) as its
    /// argument.
    fn after_terminal_view_layout(&mut self, size: Vector2F, ctx: &mut ViewContext<Self>) {
        let size_update = SizeUpdateBuilder::after_layout(*self.size_info, size).build(self, ctx);
        self.resize_internal(size_update, ctx);

        // Update the height of the "gap" - the space we would need to clear
        // in the terminal to accommodate a clear or ctrl-L.
        let mut model = self.model.lock();
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let gap_height_in_lines = match (
            model.is_alt_screen_active(),
            input_mode,
            model.block_list().active_gap(),
        ) {
            (false, InputMode::Waterfall, Some(_)) => {
                let last_frame_input_height = ctx
                    .element_position_by_id(self.input.as_ref(ctx).save_position_id())
                    .map_or(Pixels::zero(), |r| r.height().into_pixels());

                // If there is already a gap in waterfall mode, we can't use the block list element's size
                // to figure out the next gap and so we have to do some math to figure out
                // the space available for the clear.
                self.size_info.pane_height_px() - last_frame_input_height
            }
            (_, _, _) => {
                // If there is no gap, then the height of the next gap is just the
                // entire height of the block list or alt grid.
                Pixels::new(self.content_element_height_px(ctx))
            }
        }
        .to_lines(self.size_info.cell_height_px);
        model
            .block_list_mut()
            .set_next_gap_height_in_lines(gap_height_in_lines);
    }

    fn is_block_visible_locking(
        &self,
        block_index: BlockIndex,
        block_visibility: BlockVisibilityMode,
        input_mode: InputMode,
        app: &AppContext,
    ) -> bool {
        let model = self.model.lock();
        self.is_block_visible(
            block_index,
            model.block_list(),
            block_visibility,
            input_mode,
            app,
        )
    }

    // Whether a block is visible. We define a block to be visible if its command is in the
    // viewport.
    fn is_block_visible(
        &self,
        block_index: BlockIndex,
        block_list: &BlockList,
        block_visibility: BlockVisibilityMode,
        input_mode: InputMode,
        app: &AppContext,
    ) -> bool {
        self.viewport_state(block_list, input_mode, app)
            .is_block_in_view(block_index, block_visibility)
    }

    pub fn mark_as_visible(&mut self) {
        self.was_ever_visible = true;
    }

    pub fn set_active_session_state(
        &mut self,
        _state: ActiveSessionState,
        ctx: &mut ViewContext<Self>,
    ) {
        self.on_pane_state_change(ctx);
    }

    fn toggle_left_panel_file_tree(&self, force_open: bool, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::ToggleLeftPanel {
            target_view: LeftPanelTargetView::FileTree,
            force_open,
        });
    }

    /// Adds persistent toast to toast stack.
    pub fn show_persistent_toast(
        &mut self,
        text: String,
        flavor: ToastFlavor,
        ctx: &mut ViewContext<Self>,
    ) {
        let window_id = ctx.window_id();
        ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            let toast = DismissibleToast::new(text, flavor);
            toast_stack.add_persistent_toast(toast, window_id, ctx);
        });
    }

    /// Currently, we show the notification error in the form of a banner,
    /// similar to how we help the user discover notifications via a banner.
    pub fn show_notification_error(
        &mut self,
        error: NotificationSendError,
        ctx: &mut ViewContext<Self>,
    ) {
        // If notifications are not enabled on this platform, we don't want to
        // show the notification error banner.
        if !SessionSettings::as_ref(ctx)
            .notifications
            .is_supported_on_current_platform()
        {
            return;
        }

        self.inline_banners_state.notifications_error_banner.error = Some(error);

        // Only show a banner if it is currently closed
        if matches!(
            self.inline_banners_state
                .notifications_error_banner
                .banner_type,
            NotificationsErrorBannerType::Closed
        ) {
            if self
                .model
                .lock()
                .block_list()
                .active_block()
                .is_active_and_long_running()
            {
                // If the current block is still running, mark the banner as
                // triggered so we can surface it once this block completes
                self.inline_banners_state
                    .notifications_error_banner
                    .banner_type = NotificationsErrorBannerType::Triggered;
            } else {
                // If the current block is not running, open the banner up right away
                self.insert_notifications_error_banner(ctx);
            }
        }
    }

    pub fn scroll_position(&self) -> ScrollPosition {
        self.scroll_position.position()
    }

    pub fn shell_family(&self, ctx: &mut ViewContext<Self>) -> ShellFamily {
        self.active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
            .map(|session| session.shell().shell_type().into())
            .unwrap_or_else(|| {
                SessionSettings::handle(ctx).read(ctx, |settings, _| {
                    settings
                        .new_session_shell_override
                        .value()
                        .clone()
                        .unwrap_or_default()
                        .shell_family()
                })
            })
    }

    fn paste(&mut self, middle_click: bool, ctx: &mut ViewContext<Self>) {
        let (should_paste_in_input, needs_bracketed_paste) = {
            let mut model = self.model.lock();
            (
                // If the block list isn't bootstrapped yet, there could be something in the .rc file waiting for input,
                // and we want the paste to go there if the editor isn't focused.
                self.is_input_box_visible(&model, ctx)
                    && (self.input.as_ref(ctx).editor().is_focused(ctx)
                        || model.block_list().is_bootstrapped()),
                model.needs_bracketed_paste(),
            )
        };

        let is_cli_agent_paste =
            !should_paste_in_input && !middle_click && self.has_active_cli_agent_session(ctx);

        // If we're pasting into a CLI coding agent (e.g. Claude Code) that has its own native
        // handling for pasted file paths and images, skip shell-escaping and let the agent
        // see https://github.com/anthropics/claude-code/issues/18590.
        let shell_family = if is_cli_agent_paste {
            None
        } else {
            Some(self.shell_family(ctx))
        };
        let mut copied = if middle_click {
            TerminalView::middle_click_paste_content(shell_family, ctx)
        } else {
            let clipboard_content = ctx.clipboard().read();

            if is_cli_agent_paste && clipboard_content.has_image_data() {
                // Windows Claude Code paste is `Alt+V` (ESC + 'v'); macOS/Linux is `Ctrl+V` (SYN).
                let paste_bytes: Vec<u8> = if cfg!(windows) {
                    vec![0x1b, b'v']
                } else {
                    vec![0x16]
                };
                self.write_user_bytes_to_pty(paste_bytes, ctx);
                return;
            }

            clipboard_content_with_escaped_paths(clipboard_content, shell_family, false)
        };

        if should_paste_in_input {
            // We put everything from the clipboard into the input box, even
            // if it includes non-printable characters.
            self.input.update(ctx, |input, ctx| {
                input.system_insert(&copied, ctx);
            });
        } else {
            // We need to replace newlines (either \n or \r\n) with \r, as
            // otherwise programs that don't support bracketed paste might
            // misinterpret newlines as a ^J sequence.
            // See: https://github.com/vercel/hyper/issues/1448#issuecomment-367890105
            copied = LINEFEED_REGEX
                .replace_all(copied.as_str(), "\r")
                .to_string();

            if needs_bracketed_paste {
                // If bracketed paste is enabled in the current grid, then we should surround any
                // paste operation with `\x1b[200~` and `\x1b[201~` so that the application knows
                // the text came from paste, rather than from direct user input.
                // See https://cirw.in/blog/bracketed-paste for more info on bracketed paste
                copied = format!("{BRACKETED_PASTE_PREFIX}{copied}{BRACKETED_PASTE_SUFFIX}");
            }
            self.write_user_bytes_to_pty(copied.into_bytes(), ctx);
        }
    }

    fn has_active_cli_agent_session(&self, ctx: &AppContext) -> bool {
        CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .is_some()
    }

    fn is_inverted_blocklist(&self, ctx: &ViewContext<Self>) -> bool {
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        input_mode.is_inverted_blocklist()
    }

    fn copy(&mut self, ctx: &mut ViewContext<Self>) {
        let semantic_selection = SemanticSelection::as_ref(ctx);
        if let Some(selected) = self.model.lock().selection_to_string(
            semantic_selection,
            self.is_inverted_blocklist(ctx),
            ctx,
        ) {
            if !selected.is_empty() {
                ctx.clipboard()
                    .write(ClipboardContent::plain_text(selected));
            }
            return;
        }
        if !self.selected_blocks.is_empty() {
            self.copy_blocks(BlockEntity::CommandAndOutput, ctx);
        }
    }

    fn copy_commands(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.selected_blocks.is_empty() {
            self.copy_blocks(BlockEntity::Command, ctx);
        }
    }

    fn copy_outputs(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.selected_blocks.is_empty() {
            self.copy_blocks(BlockEntity::FilteredOutput, ctx);
        }
    }

    fn context_menu_items(
        &self,
        menu_source: &BlockListMenuSource,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let model = self.model.lock();

        let items = match (
            menu_source,
            self.highlighted_link.as_ref(),
            self.selected_blocks.is_empty(),
        ) {
            (
                BlockListMenuSource::RegularBlockRightClick { .. }
                | BlockListMenuSource::RichContentBlockRightClick { .. },
                Some(highlighted_link),
                _,
            ) => {
                match highlighted_link {
                    GridHighlightedLink::Url(url) => {
                        let url_content =
                            Some(model.link_at_range(url, RespectObfuscatedSecrets::Yes));
                        url_content
                            .map(|url_content| {
                                vec![MenuItemFields::new("Copy URL")
                                    .with_on_select_action(TerminalAction::ContextMenu(
                                        ContextMenuAction::CopyUrl { url_content },
                                    ))
                                    .into_item()]
                            })
                            .unwrap_or_default()
                    }
                    #[cfg(feature = "local_fs")]
                    GridHighlightedLink::File(file_link) => {
                        let path = file_link.get_inner().absolute_path();
                        let show_in_file_explorer_menu_item_label = if cfg!(target_os = "macos") {
                            "Show in Finder"
                        } else {
                            "Show containing folder"
                        };
                        path.map(|path| {
                            let mut items = vec![
                                MenuItemFields::new("Copy path")
                                    .with_on_select_action(TerminalAction::ContextMenu(
                                        ContextMenuAction::CopyUrl {
                                            url_content: path.to_string_lossy().into(),
                                        },
                                    ))
                                    .into_item(),
                                MenuItemFields::new(show_in_file_explorer_menu_item_label)
                                    .with_on_select_action(TerminalAction::ShowInFileExplorer(
                                        path.clone(),
                                    ))
                                    .into_item(),
                            ];

                            if is_markdown_file(&path) {
                                items.push(
                                    MenuItemFields::new("Open in Warper")
                                        .with_on_select_action(TerminalAction::OpenFileInWarp(path))
                                        .into_item(),
                                );
                                // Because the default for cmd-click is to open in Warp, we also
                                // have an open-in-editor option.
                                items.push(
                                    MenuItemFields::new("Open in editor")
                                        .with_on_select_action(TerminalAction::OpenGridLink(
                                            highlighted_link.clone(),
                                        ))
                                        .into_item(),
                                );
                            }

                            items
                        })
                        .unwrap_or_default()
                    }
                }
            }
            (
                BlockListMenuSource::RegularTextRightClick { .. }
                | BlockListMenuSource::RichContentTextRightClick { .. },
                None,
                true,
            ) => {
                let fields = vec![
                    MenuItemFields::new("Copy")
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::CopySelectedText,
                        ))
                        .with_key_shortcut_label(keybinding_name_to_display_string(
                            "terminal:copy",
                            ctx,
                        ))
                        .into_item(),
                    MenuItemFields::new("Insert into input")
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::InsertSelectedText,
                        ))
                        .into_item(),
                ];

                fields
            }
            (
                BlockListMenuSource::BlockOverflowButton { .. }
                | BlockListMenuSource::BlockKeybinding { .. }
                | BlockListMenuSource::RegularBlockRightClick { .. }
                | BlockListMenuSource::RichContentBlockRightClick { .. }
                | BlockListMenuSource::OutsideBlockRightClick { .. },
                None,
                false,
            ) => {
                let tail_block_index = self
                    .selected_blocks
                    .tail()
                    .expect("Expected at least one block to be selected.");

                let tail_block = match model.block_list().block_at(tail_block_index) {
                    None => return vec![],
                    Some(block) => block,
                };

                let is_single_selection = self.selected_blocks.is_singleton();
                let copy_commands_str = if is_single_selection {
                    "Copy command"
                } else {
                    "Copy commands"
                };
                let copy_str = "Copy";
                let find_str = if is_single_selection {
                    "Find within block"
                } else {
                    "Find within blocks"
                };
                let scroll_to_top_str = if is_single_selection {
                    "Scroll to top of block"
                } else {
                    "Scroll to top of blocks"
                };
                let scroll_to_bottom_str = if is_single_selection {
                    "Scroll to bottom of block"
                } else {
                    "Scroll to bottom of blocks"
                };

                let is_copy_commands_disabled =
                    is_single_selection && tail_block.command_to_string().trim().is_empty();
                let is_copy_both_disabled =
                    is_copy_commands_disabled && tail_block.output_to_string().trim().is_empty();

                let mut items = vec![
                    MenuItemFields::new(copy_str)
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::CopyBlocks,
                        ))
                        .with_key_shortcut_label(keybinding_name_to_display_string(
                            "terminal:copy",
                            ctx,
                        ))
                        .with_disabled(is_copy_both_disabled)
                        .into_item(),
                    MenuItemFields::new(copy_commands_str)
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::CopyBlockCommands,
                        ))
                        .with_key_shortcut_label(keybinding_name_to_display_string(
                            "terminal:copy_commands",
                            ctx,
                        ))
                        .with_disabled(is_copy_commands_disabled)
                        .into_item(),
                ];

                if false {
                    items.push(MenuItem::Separator);
                    items.push(
                        MenuItemFields::new("Save as workflow")
                            .with_on_select_action(TerminalAction::ContextMenu(
                                ContextMenuAction::OpenWorkflowModal,
                            ))
                            .with_key_shortcut_label(keybinding_name_to_display_string(
                                "terminal:toggle_teams_modal",
                                ctx,
                            ))
                            .into_item(),
                    );
                }

                if is_single_selection {
                    let mut copy_output_menu_item = MenuItemFields::new("Copy output")
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::CopyBlockOutputs,
                        ))
                        .with_disabled(tail_block.output_grid().is_empty());

                    // If there is an active filter on a block, then we want to display a
                    // Copy filtered output option and assign the "terminal:copy_outputs" keybinding to it.
                    if tail_block.has_active_filter() {
                        items.insert(
                            1,
                            MenuItemFields::new("Copy filtered output")
                                .with_on_select_action(TerminalAction::ContextMenu(
                                    ContextMenuAction::CopyBlockFilteredOutputs,
                                ))
                                .with_key_shortcut_label(keybinding_name_to_display_string(
                                    "terminal:copy_outputs",
                                    ctx,
                                ))
                                .into_item(),
                        );
                        items.insert(2, copy_output_menu_item.into_item());
                    } else {
                        copy_output_menu_item = copy_output_menu_item.with_key_shortcut_label(
                            keybinding_name_to_display_string("terminal:copy_outputs", ctx),
                        );
                        items.insert(2, copy_output_menu_item.into_item());
                    }

                    let mut prompt_items = self.copy_prompt_menu_items(
                        self.input_is_on_git_branch(&model),
                        self.is_rprompt_shown(&model),
                        PromptPosition::Block(tail_block_index),
                    );
                    items.push(MenuItem::Separator);
                    items.append(&mut prompt_items);
                }

                items.append(&mut vec![
                    MenuItem::Separator,
                    MenuItemFields::new(find_str)
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::FindWithinBlock,
                        ))
                        .with_key_shortcut_label(keybinding_name_to_display_string(
                            "terminal:find",
                            ctx,
                        ))
                        .into_item(),
                ]);
                items.append(&mut vec![MenuItemFields::new("Toggle block filter")
                    .with_on_select_action(TerminalAction::ToggleBlockFilterOnSelectedOrLastBlock(
                        ToggleBlockFilterSource::ContextMenu,
                    ))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        TOGGLE_BLOCK_FILTER_KEYBINDING,
                        ctx,
                    ))
                    .into_item()]);
                items.append(&mut vec![MenuItemFields::new("Toggle bookmark")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::ToggleBookmark,
                    ))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "terminal:bookmark_selected_block",
                        ctx,
                    ))
                    .into_item()]);

                items.append(&mut vec![
                    MenuItem::Separator,
                    MenuItemFields::new(scroll_to_top_str)
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::ScrollToTopOfBlock,
                        ))
                        .with_key_shortcut_label(keybinding_name_to_display_string(
                            "terminal:scroll_to_top_of_selected_block",
                            ctx,
                        ))
                        .into_item(),
                ]);
                items.append(&mut vec![MenuItemFields::new(scroll_to_bottom_str)
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::ScrollToBottomOfBlock,
                    ))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "terminal:scroll_to_bottom_of_selected_block",
                        ctx,
                    ))
                    .into_item()]);

                items
            }
            (
                BlockListMenuSource::RichContentBlockRightClick { .. }
                | BlockListMenuSource::OutsideBlockRightClick { .. },
                None,
                true,
            ) => Vec::new(),
            _ => vec![],
        };

        items
    }

    fn copy_prompt_menu_items(
        &self,
        is_on_git_branch: bool,
        is_rprompt_shown: bool,
        position: PromptPosition,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut items = vec![MenuItemFields::new("Copy prompt")
            .with_on_select_action(TerminalAction::ContextMenu(ContextMenuAction::CopyPrompt {
                position,
                part: PromptPart::EntirePrompt,
            }))
            .into_item()];

        if is_rprompt_shown {
            items.push(
                MenuItemFields::new("Copy right prompt")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::CopyRprompt,
                    ))
                    .into_item(),
            );
        }

        items.push(
            MenuItemFields::new("Copy working directory")
                .with_on_select_action(TerminalAction::ContextMenu(ContextMenuAction::CopyPrompt {
                    position,
                    part: PromptPart::Pwd,
                }))
                .into_item(),
        );

        if is_on_git_branch {
            items.push(
                MenuItemFields::new("Copy git branch")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::CopyPrompt {
                            position,
                            part: PromptPart::GitBranch,
                        },
                    ))
                    .into_item(),
            )
        }
        items
    }

    fn pane_context_menu_items(
        &self,
        shell: Option<AvailableShell>,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut items = vec![];

        if ContextFlag::CreateNewSession.is_enabled() {
            items.extend(vec![
                MenuItemFields::new("Split pane right")
                    .with_on_select_action(TerminalAction::SplitRight(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_right",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane left")
                    .with_on_select_action(TerminalAction::SplitLeft(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_left",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane down")
                    .with_on_select_action(TerminalAction::SplitDown(shell.clone()))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_down",
                        ctx,
                    ))
                    .into_item(),
                MenuItemFields::new("Split pane up")
                    .with_on_select_action(TerminalAction::SplitUp(shell))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:add_up",
                        ctx,
                    ))
                    .into_item(),
            ]);
        }

        let pane_state = self.split_pane_state(ctx);
        if pane_state.is_in_split_pane() {
            let is_maximized = pane_state.is_maximized();
            items.push(
                MenuItemFields::toggle_pane_action(is_maximized)
                    .with_on_select_action(TerminalAction::ToggleMaximizePane)
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "pane_group:toggle_maximize_pane",
                        ctx,
                    ))
                    .into_item(),
            );

            items.push(
                MenuItemFields::new("Close pane")
                    .with_on_select_action(TerminalAction::Close)
                    .with_key_shortcut_label(
                        custom_tag_to_keystroke(CustomAction::CloseCurrentSession.into())
                            .map(|keystroke| keystroke.displayed()),
                    )
                    .into_item(),
            );
        }

        items
    }

    fn input_is_on_git_branch(&self, model: &TerminalModel) -> bool {
        PromptPosition::Input
            .block(model)
            .and_then(Block::git_branch)
            .is_some()
    }

    fn is_rprompt_shown(&self, model: &TerminalModel) -> bool {
        model
            .block_list()
            .active_block()
            .should_display_rprompt(&self.size_info)
    }

    /// Closes all overlays managed by the terminal view and its input. Does not change what
    /// element is focused.
    pub fn close_overlays(&mut self, ctx: &mut ViewContext<Self>) {
        self.close_context_menu(ctx, false);
        self.close_block_filter_editor(ctx);
        self.close_find_bar(ctx);

        self.input.update(ctx, |input, ctx| {
            input.close_overlays(true, ctx);
        });
    }

    fn prompt_context_menu_items(&self, ctx: &AppContext) -> Vec<MenuItem<TerminalAction>> {
        let copy_prompt = MenuItemFields::new("Copy prompt")
            .with_on_select_action(TerminalAction::ContextMenu(ContextMenuAction::CopyPrompt {
                position: PromptPosition::Input,
                part: PromptPart::EntirePrompt,
            }))
            .into_item();

        let has_cli_agent_session = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .is_some();
        let edit_menu_item = if has_cli_agent_session {
            FeatureFlag::AgentToolbarEditor.is_enabled().then(|| {
                MenuItemFields::new("Edit CLI agent toolbelt")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::EditCLIAgentToolbar,
                    ))
                    .into_item()
            })
        } else {
            Some(
                MenuItemFields::new("Edit prompt")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::EditPrompt,
                    ))
                    .with_disabled(false)
                    .into_item(),
            )
        };

        if *SessionSettings::as_ref(ctx).honor_ps1 {
            let mut items = vec![copy_prompt];
            if self.is_rprompt_shown(&self.model.lock()) {
                items.push(
                    MenuItemFields::new("Copy right prompt")
                        .with_on_select_action(TerminalAction::ContextMenu(
                            ContextMenuAction::CopyRprompt,
                        ))
                        .into_item(),
                );
            }
            if let Some(edit_menu_item) = edit_menu_item {
                items.extend([MenuItem::Separator, edit_menu_item]);
            }
            items
        } else {
            let mut items = vec![copy_prompt];
            let current_prompt_menu_items = self
                .current_prompt
                .as_ref(ctx)
                .copy_menu_items(PromptPosition::Input, ctx);
            if !current_prompt_menu_items.is_empty() {
                items.push(MenuItem::Separator);
                items.extend(current_prompt_menu_items);
            }
            if let Some(edit_menu_item) = edit_menu_item {
                items.extend([MenuItem::Separator, edit_menu_item]);
            }
            items
        }
    }

    fn show_prompt_context_menu(&mut self, position: Vector2F, ctx: &mut ViewContext<Self>) {
        let items = self.prompt_context_menu_items(ctx);
        self.show_context_menu(
            ContextMenuState {
                menu_type: ContextMenuType::Prompt { position },
            },
            items,
            ctx,
        );
    }

    fn input_context_menu_items(
        &self,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let model = self.model.lock();
        let mut items = Vec::new();

        let is_editor_disabled = false;

        // Section 1: Cut, Copy, Copy All, Paste, Share Session
        let (all_current_input_text, selected_input_text) = self.input.read(ctx, |input, ctx| {
            input.editor().read(ctx, |editor, ctx| {
                (editor.buffer_text(ctx), editor.selected_text(ctx))
            })
        });

        if !selected_input_text.is_empty() {
            items.extend([
                MenuItemFields::new("Cut")
                    .with_on_select_action(TerminalAction::InputContextMenuItem(
                        InputContextMenuAction::CutSelectedText,
                    ))
                    .into_item(),
                MenuItemFields::new("Copy")
                    .with_on_select_action(TerminalAction::InputContextMenuItem(
                        InputContextMenuAction::CopySelectedText,
                    ))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "terminal:copy",
                        ctx,
                    ))
                    .into_item(),
            ]);
        }

        if !all_current_input_text.is_empty() & selected_input_text.is_empty() {
            items.push(
                MenuItemFields::new("Select all")
                    .with_on_select_action(TerminalAction::InputContextMenuItem(
                        InputContextMenuAction::SelectAll,
                    ))
                    .with_key_shortcut_label(keybinding_name_to_display_string(
                        "editor_view:select_all",
                        ctx,
                    ))
                    .with_disabled(is_editor_disabled)
                    .into_item(),
            );
        }

        items.push(
            MenuItemFields::new("Paste")
                .with_on_select_action(TerminalAction::InputContextMenuItem(
                    InputContextMenuAction::Paste,
                ))
                .with_key_shortcut_label(keybinding_name_to_display_string("terminal:paste", ctx))
                .with_disabled(is_editor_disabled)
                .into_item(),
        );

        // Section 2: command search
        items.extend([
            MenuItem::Separator,
            MenuItemFields::new("Command search")
                .with_on_select_action(TerminalAction::InputContextMenuItem(
                    InputContextMenuAction::ShowCommandSearch,
                ))
                .with_key_shortcut_label(keybinding_name_to_display_string(
                    "workspace:show_command_search",
                    ctx,
                ))
                .with_disabled(is_editor_disabled)
                .into_item(),
        ]);

        // Section 3: Teams related
        if false {
            items.extend([
                MenuItem::Separator,
                MenuItemFields::new("Save as workflow")
                    .with_on_select_action(TerminalAction::InputContextMenuItem(
                        InputContextMenuAction::SaveAsWorkflow,
                    ))
                    .into_item(),
            ]);
        }

        // Section 4: input hint text toggle
        if !is_editor_disabled {
            let input_settings = InputSettings::as_ref(ctx);
            let inverse_action = if *input_settings.show_hint_text {
                "Hide"
            } else {
                "Show"
            };
            items.push(MenuItem::Separator);
            items.push(
                MenuItemFields::new(format!("{inverse_action} input hint text"))
                    .with_on_select_action(TerminalAction::InputContextMenuItem(
                        InputContextMenuAction::ToggleInputHintText,
                    ))
                    .into_item(),
            );
        }
        // Section 5: All Pane related
        let current_shell = model.shell_launch_state().available_shell();
        let pane_context_menu_items = self.pane_context_menu_items(current_shell, ctx);
        if !pane_context_menu_items.is_empty() {
            items.push(MenuItem::Separator);
            items.extend(pane_context_menu_items);
        }

        items
    }

    fn show_input_context_menu(&mut self, position: Vector2F, ctx: &mut ViewContext<Self>) {
        let items = self.input_context_menu_items(ctx);

        self.show_context_menu(
            ContextMenuState {
                menu_type: ContextMenuType::Input { position },
            },
            items,
            ctx,
        );
    }

    fn open_workflow_modal(&mut self, ctx: &mut ViewContext<Self>) {
        let selected_block_contents =
            self.selected_block_contents_as_string(BlockEntity::Command, " &&\n", ctx);

        self.open_workflow_modal_with_command(selected_block_contents, ctx);
    }

    fn open_block_filter_editor(
        &mut self,
        block_index: BlockIndex,
        opened_from_click: OpenedFromClick,
        ctx: &mut ViewContext<Self>,
    ) {
        self.active_filter_editor_block_index = Some(block_index);
        {
            let model = self.model.lock();
            let active_filter_query = model
                .block_list()
                .block_at(block_index)
                .and_then(|block| block.current_filter())
                .filter(|query| query.is_active)
                .cloned();
            let num_matched_lines = model
                .block_list()
                .num_matched_lines_in_filter_for_block(block_index);
            self.block_filter_editor
                .update(ctx, |block_filter_editor, ctx| {
                    block_filter_editor.open_and_set_filter(
                        active_filter_query,
                        num_matched_lines,
                        ctx,
                    );
                });
        }
        self.focus_block_filter_editor(ctx);
        if matches!(opened_from_click, OpenedFromClick::Yes) {}
    }

    fn close_block_filter_editor(&mut self, ctx: &mut ViewContext<Self>) {
        self.active_filter_editor_block_index = None;
        self.block_filter_editor.update(ctx, |block_filter, ctx| {
            block_filter.reset(ctx);
        });
        ctx.notify();
    }

    fn open_workflow_modal_from_block(
        &mut self,
        block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        // Make the block for which we're showing the modal the only selected block.
        self.reset_selection_to_single_block(block_index, ctx);
        self.scroll_to_if_not_visible(block_index, ctx);

        // Set the command in the modal to the command of the block.
        if let Some(block) = self.model.lock().block_list().block_at(block_index) {
            ctx.emit(Event::OpenWorkflowModalWithCommand(
                block.command_to_string(),
            ))
        }
    }

    pub fn open_workflow_modal_with_existing(
        &mut self,
        workflow_id: SyncId,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.emit(Event::OpenWorkflowModalWithLocalWorkflow(workflow_id));
        ctx.notify();
    }

    /// Helper method to build alt screen context menu items.
    /// Used both when opening the menu and when rebuilding it (e.g., on pane state changes).
    fn rebuild_alt_screen_context_menu_items(
        &self,
        ctx: &mut ViewContext<Self>,
    ) -> Vec<MenuItem<TerminalAction>> {
        let mut menu_items = Vec::new();
        let model = self.model.lock();

        let semantic_selection = SemanticSelection::as_ref(ctx);
        let selection_string =
            model.selection_to_string(semantic_selection, self.is_inverted_blocklist(ctx), ctx);
        if selection_string.is_some() {
            menu_items.push(
                MenuItemFields::new("Copy")
                    .with_on_select_action(TerminalAction::ContextMenu(
                        ContextMenuAction::CopySelectedText,
                    ))
                    .with_key_shortcut_label(Some("⌘-C"))
                    .into_item(),
            );
        }

        let current_shell = model.shell_launch_state().available_shell();
        let mut pane_context_menu_items = self.pane_context_menu_items(current_shell, ctx);
        if !menu_items.is_empty() && !pane_context_menu_items.is_empty() {
            menu_items.push(MenuItem::Separator);
        }
        if !pane_context_menu_items.is_empty() {
            menu_items.append(&mut pane_context_menu_items);
        }
        menu_items
    }

    fn alt_screen_context_menu(&mut self, position: Vector2F, ctx: &mut ViewContext<Self>) {
        let menu_items = self.rebuild_alt_screen_context_menu_items(ctx);
        self.show_context_menu(
            ContextMenuState {
                menu_type: ContextMenuType::AltScreen { position },
            },
            menu_items,
            ctx,
        );
    }

    fn block_list_context_menu(
        &mut self,
        menu_source: &BlockListMenuSource,
        ctx: &mut ViewContext<Self>,
    ) {
        match menu_source {
            BlockListMenuSource::BlockOverflowButton { block_index }
            | BlockListMenuSource::RegularBlockRightClick { block_index, .. } => {
                if !self.selected_blocks.is_selected(*block_index) {
                    // If the context menu is already open, we just want to close
                    // the context menu for the existing selections instead of changing
                    // the selections
                    // TODO(INT-922): It doesn't look like this code is actually being reached. Is this behavior intended?
                    if self.is_context_menu_open() {
                        self.close_context_menu(ctx, true);
                        return;
                    }
                    self.reset_selection_to_single_block(*block_index, ctx);
                }
            }

            BlockListMenuSource::BlockKeybinding { .. } => {
                // If the context menu is already open, we just want to close
                // the context menu for the existing selections instead of changing
                // the selections
                if self.is_context_menu_open() {
                    self.close_context_menu(ctx, true);
                    return;
                }
            }

            BlockListMenuSource::RichContentBlockRightClick { .. }
            | BlockListMenuSource::OutsideBlockRightClick { .. } => {
                // Existing text selections should be deselected when opening a context menu
                // elsewhere. This is already done automatically for RegularBlockRightClick,
                // since block selections clear selected text.
                self.clear_selected_text(ctx);
            }

            BlockListMenuSource::RegularTextRightClick { .. }
            | BlockListMenuSource::RichContentTextRightClick { .. } => {}
        }

        let items = self.context_menu_items(menu_source, ctx);
        if !items.is_empty() {
            self.show_context_menu(
                ContextMenuState {
                    menu_type: ContextMenuType::BlockList {
                        menu_source: *menu_source,
                    },
                },
                items,
                ctx,
            );
        }
    }

    fn show_context_menu(
        &mut self,
        menu_state: ContextMenuState,
        items: Vec<MenuItem<TerminalAction>>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.update_view(&self.context_menu, |context_menu, view_ctx| {
            context_menu.set_origin(menu_state.menu_type.origin());
            context_menu.set_width(CONTEXT_MENU_WIDTH);
            // This will also reset the selection.
            context_menu.set_items(items, view_ctx);
        });

        self.context_menu_state = Some(menu_state);
        ctx.focus(&self.context_menu);
        ctx.notify();

        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });
    }

    fn alt_mouse_action(&mut self, mouse_state: &MouseState, ctx: &mut ViewContext<Self>) {
        let escape_sequences = mouse_state
            .to_escape_sequence(self.model.lock().deref())
            .unwrap();
        self.write_user_bytes_to_pty(escape_sequences, ctx);
    }

    fn alt_select(&mut self, arg: &SelectAction<Point>, ctx: &mut ViewContext<Self>) {
        match arg {
            SelectAction::Begin {
                point,
                side,
                selection_type,
                ..
            } => {
                self.begin_alt_selection(*point, *side, *selection_type, ctx);
            }
            SelectAction::Update {
                point, side, delta, ..
            } => self.update_alt_selection(*point, *side, delta, ctx),
            SelectAction::End => {
                self.end_alt_selection(ctx);
            }
        }
    }

    fn begin_alt_selection(
        &mut self,
        point: Point,
        side: Side,
        selection_type: SelectionType,
        ctx: &mut ViewContext<Self>,
    ) {
        // is starting on the alt screen (which can be visible simultaneously).

        self.model.lock().alt_screen_mut().clear_selection();
        self.model
            .lock()
            .alt_screen_mut()
            .start_selection(point, selection_type, side);
        self.is_selecting = true;

        ctx.notify();
    }

    fn update_alt_selection(
        &mut self,
        point: Point,
        side: Side,
        _delta: &Lines,
        ctx: &mut ViewContext<Self>,
    ) {
        self.model
            .lock()
            .alt_screen_mut()
            .update_selection(point, side);
        ctx.notify();
    }

    fn end_alt_selection(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_selecting {
            self.is_selecting = false;
            self.maybe_copy_selection_to_clipboard(ctx);
            ctx.notify();
        } else {
            log::error!("end_selection dispatched with no pending selection");
        }
    }

    fn end_text_selection(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_selecting {
            self.is_selecting = false;
            self.block_text_selection_start_position = None;

            let selected_text = {
                let semantic_selection = SemanticSelection::as_ref(ctx);
                self.model
                    .lock()
                    .selection_to_string(semantic_selection, false, ctx)
                    .filter(|text| !text.is_empty())
            };

            // The text selection changed, so clear any previously attached context text.

            // A text selection might be a byproduct of a block selection.
            // If there's no renderable text selection, we should clear the text selection.
            if selected_text.is_none() {
                self.clear_selected_text(ctx);
            } else {
                self.maybe_copy_selection_to_clipboard(ctx);
                // Text and block selections are mutually exclusive context sources.
                // When the user makes a non-empty text selection, clear any block selections.
                self.clear_selected_blocks(ctx);
            }

            ctx.notify();
        } else {
            log::error!("end_selection dispatched with no pending selection");
        }
    }

    // Additionally handles side effects of changing block selections (i.e. CMD + F results,
    // a `change_block_selections` invocation.
    fn change_block_selections<F>(&mut self, change_selection: F, ctx: &mut ViewContext<Self>)
    where
        F: FnOnce(&mut SelectedBlocks),
    {
        change_selection(&mut self.selected_blocks);
        self.update_find_selection(ctx);

        // selections, we must also update the context

        ctx.emit(Event::SelectedBlocksChanged);
    }

    pub fn integration_test_change_block_selection_to_single(
        &mut self,
        block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        self.reset_selection_to_single_block(block_index, ctx);
    }

    fn block_select(
        &mut self,
        block_action: &BlockSelectAction,
        should_redetermine_focus: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        // If the input suggestions are showing, remove them and don't update block selection.
        // The mouse up event is excluded here as scrollbar and text selection in input suggestion
        // could cause it to misfire (see WAR-274 and WAR-407).
        if self.selected_blocks.is_empty()
            && self
                .input
                .as_ref(ctx)
                .suggestions_mode_model()
                .as_ref(ctx)
                .mode()
                .is_visible()
            && !matches!(block_action, BlockSelectAction::MouseUp { .. })
        {
            self.input
                .update(ctx, |input, ctx| input.close_input_suggestions(true, ctx));
            return;
        }

        // If the context menu is open, clicking somewhere on the block list
        // should only close the context menu, and NOT update block selections.
        if self.is_context_menu_open() {
            self.close_context_menu(ctx, true);
            return;
        }

        match block_action {
            BlockSelectAction::ClearAllBlocks => {
                self.clear_selected_blocks(ctx);
            }
            BlockSelectAction::MouseDown(maybe_block_index) => {
                if let Some(block_index) = maybe_block_index {
                    self.mouse_down_block_index = Some(*block_index);

                    self.tips_completed.update(ctx, |tips, ctx| {
                        mark_feature_used_and_write_to_user_defaults(
                            Tip::Hint(TipHint::BlockSelect),
                            tips,
                            ctx,
                        );
                        ctx.notify();
                    });
                } else {
                    // Clear the current block selection upon clicking on a rich content block
                    self.clear_selected_blocks(ctx);

                    // Since rich content blocks cannot be selected, `redetermine_focus` has no way
                    // of knowing whether the user just clicked on a rich content block. To allow
                    // users to attach blocks as context and submit queries quickly, we only divert

                    self.focus_terminal(ctx);

                    // As part of Code Mode V2, we're introducing left and right panels which might be focused
                    // but we want to allow users to click to refocus to a terminal session
                    // so if the terminal isn't focused and a user clicks into the terminal, we want to force focusing the input
                }
            }
            BlockSelectAction::MouseUp {
                block_index,
                is_ctrl_down,
                is_cmd_down,
                is_shift_down,
            } => {
                if let Some(mouse_down_block_index) = self.mouse_down_block_index.take() {
                    // There is a highlighted url and cmd key is held -- don't process this as a block selection.
                    if self.highlighted_link.is_some() && *is_cmd_down {
                        return;
                    }

                    let semantic_selection = SemanticSelection::as_ref(ctx);
                    // Only allow a block to be selected if it's the same block as the mouse down event
                    // and if there's currently no block text selection
                    if mouse_down_block_index == *block_index
                        && self
                            .model
                            .lock()
                            .block_list()
                            .renderable_selection(
                                semantic_selection,
                                self.is_inverted_blocklist(ctx),
                            )
                            .is_none()
                    {
                        let should_toggle_block_selected = if cfg!(target_os = "macos") {
                            *is_cmd_down
                        } else {
                            *is_ctrl_down
                        };

                        if should_toggle_block_selected {
                            // We need to use the next and prev non-hidden indices to
                            // ensure that the tail/pivot of a range selection will never
                            // be a hidden index.
                            let next = self
                                .model
                                .lock()
                                .block_list()
                                .next_non_hidden_block_from_index(*block_index);
                            let prior = self
                                .model
                                .lock()
                                .block_list()
                                .prev_non_hidden_block_from_index(*block_index);

                            // This block's selection needs to be toggled.
                            // If it was already selected, then it will be unselected.
                            // If it wasn't already selected, it will be a new, disjoint selection.
                            self.change_block_selections(
                                |selected_blocks| {
                                    selected_blocks.toggle(*block_index, next, prior);
                                },
                                ctx,
                            );
                        } else if *is_shift_down && !self.selected_blocks.is_empty() {
                            self.change_block_selections(
                                |selected_blocks| {
                                    selected_blocks.range_select(*block_index);
                                },
                                ctx,
                            );
                        } else {
                            self.reset_selection_to_single_block(*block_index, ctx);
                        }

                        self.tips_completed.update(ctx, |tips, ctx| {
                            mark_feature_used_and_write_to_user_defaults(
                                Tip::Hint(TipHint::BlockSelect),
                                tips,
                                ctx,
                            );
                            ctx.notify();
                        });
                    }
                }
            }
        }

        if should_redetermine_focus {
            self.redetermine_global_focus(ctx);
        }
    }

    fn block_text_select(&mut self, arg: &BlockTextSelectAction, ctx: &mut ViewContext<Self>) {
        match arg {
            BlockTextSelectAction::Begin {
                point,
                side,
                selection_type,
                position,
            } => self.begin_block_text_selection(*point, *side, *selection_type, *position, ctx),
            BlockTextSelectAction::Update {
                point,
                side,
                delta,
                position,
            } => self.update_block_text_selection(*point, *side, *delta, *position, ctx),
            BlockTextSelectAction::End => {
                self.end_text_selection(ctx);
            }
        }
    }

    fn maybe_copy_selection_to_clipboard(&mut self, ctx: &mut ViewContext<Self>) {
        let selection_settings = SelectionSettings::handle(ctx);
        let semantic_selection = SemanticSelection::as_ref(ctx);
        let model = self.model.lock();
        if let Some(selected) =
            model.selection_to_string(semantic_selection, self.is_inverted_blocklist(ctx), ctx)
        {
            selection_settings.update(ctx, |selection_settings, ctx| {
                selection_settings
                    .maybe_copy_on_select(ClipboardContent::plain_text(selected), ctx);
            });
        }
    }

    fn terminal_is_selecting(&self, model: &TerminalModel, ctx: &mut ViewContext<Self>) -> bool {
        let semantic_selection = SemanticSelection::as_ref(ctx);
        (!model.is_alt_screen_active()
            && model
                .block_list()
                .renderable_selection(semantic_selection, self.is_inverted_blocklist(ctx))
                .is_some())
            || model
                .alt_screen()
                .selection_range(semantic_selection)
                .is_some()
    }

    fn click_on_grid(
        &mut self,
        position: &WithinModel<Point>,
        modifiers: &ModifiersState,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.terminal_is_selecting(&self.model.lock(), ctx) {
            return;
        }

        let handle = {
            let model = self.model.lock();
            model.secret_at_point(position).map(|(handle, _)| handle)
        };
        if let Some(handle) = handle {
            self.open_secret_tool_tip = Some(SecretTooltip::Grid {
                tooltip: position.replace_inner(handle),
            });
            self.focus_terminal(ctx);
        }

        let should_directly_open_link = should_directly_open_link(modifiers);
        if *GeneralSettings::as_ref(ctx).link_tooltip
            && !should_directly_open_link
            && self.highlighted_link.is_some()
        {
            self.open_grid_link_tool_tip = self.highlighted_link.clone_inner();
            self.focus_terminal(ctx);
        } else {
            self.open_grid_link_tool_tip = None;
        }

        if should_directly_open_link {
            self.maybe_open_link(position, ctx);
        }
    }

    #[cfg(feature = "local_fs")]
    fn open_file_path(
        &mut self,
        path: PathBuf,
        line_and_column_num: Option<LineAndColumnArg>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.notify();

        let settings = EditorSettings::as_ref(ctx);
        let target = resolve_file_target(&path, settings, None);

        ctx.emit(Event::OpenFileWithTarget {
            path,
            target,
            line_col: line_and_column_num,
        });
    }

    #[cfg(feature = "local_fs")]
    fn open_file_path_with_target(
        &mut self,
        path: PathBuf,
        target: FileTarget,
        line_and_column_num: Option<LineAndColumnArg>,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.notify();
        ctx.emit(Event::OpenFileWithTarget {
            path,
            target,
            line_col: line_and_column_num,
        });
    }

    fn maybe_open_link(&mut self, position: &WithinModel<Point>, ctx: &mut ViewContext<Self>) {
        let Some(link) = self.highlighted_link.as_ref() else {
            return;
        };

        match link {
            #[cfg(feature = "local_fs")]
            GridHighlightedLink::File(link) if link.contains(position) => {
                let link = link.get_inner();
                if let Some(path) = link.absolute_path() {
                    self.open_file_path(path, link.line_and_column_num, ctx);
                }
            }
            GridHighlightedLink::Url(url) if url.contains(position) => {
                let model = self.model.lock();
                ctx.notify();
                ctx.open_url(&model.link_at_range(url, RespectObfuscatedSecrets::No));
            }
            _ => (),
        }

        if self.highlighted_link.take(&mut self.model.lock()).is_some() {
            ctx.reset_cursor();
            ctx.notify();
        }
    }

    fn middle_click_on_grid(
        &mut self,
        position: &Option<WithinModel<Point>>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.highlighted_link.is_some() {
            // Middle click should open a highlighted link if there is one.
            if let Some(position) = position {
                self.maybe_open_link(position, ctx);
            }
        } else {
            // Otherwise, assume that the user wants to middle-click paste.
            self.paste(true, ctx);
        }
    }

    fn middle_click_on_input(&mut self, ctx: &mut ViewContext<Self>) {
        self.focus_input_and_clear_selections(ctx);
        self.paste(true, ctx);
    }

    /// Tell the pane group to open a file within Warp.
    fn open_file_in_warp(&mut self, path: PathBuf, ctx: &mut ViewContext<Self>) {
        if let Some(session) = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
        {
            ctx.emit(Event::OpenFileInWarp { path, session })
        }
    }

    #[cfg(feature = "local_fs")]
    fn open_code_in_warp(
        &mut self,
        source: CodeSource,
        layout: EditorLayout,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.emit(Event::OpenCodeInWarp { source, layout })
    }

    fn toggle_grid_secret(
        &mut self,
        secret_handle: &WithinModel<SecretHandle>,
        show_secret: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if show_secret && self.model.lock().unobfuscate_secret(secret_handle).is_err() {
            log::warn!(
                "Failed to reveal secret with id {}",
                secret_handle.get_inner().id()
            );
        } else if !show_secret && self.model.lock().obfuscate_secret(secret_handle).is_err() {
            log::warn!(
                "Failed to obfuscate secret with id {}",
                secret_handle.get_inner().id()
            );
        }
        self.dismiss_tooltips(ctx);
        ctx.notify();
    }

    fn copy_grid_secret(
        &mut self,
        secret_handle: &WithinModel<SecretHandle>,
        ctx: &mut ViewContext<Self>,
    ) {
        {
            let model = self.model.lock();
            if let Some(secret) = model.secret_from_handle(secret_handle) {
                let secret_in_model = secret_handle.replace_inner(secret);
                let text = model.string_at_range(&secret_in_model, RespectObfuscatedSecrets::No);
                ctx.clipboard().write(ClipboardContent::plain_text(text));
            }
        }
        self.dismiss_tooltips(ctx);
        ctx.notify();
    }

    fn maybe_hover_secret(
        &mut self,
        secret_handle: Option<SecretHandle>,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.hovered_secret != secret_handle {
            self.hovered_secret = secret_handle;
            if secret_handle.is_some() {
                ctx.set_cursor_shape(Cursor::PointingHand);
            } else {
                ctx.reset_cursor();
            }
            ctx.notify();
        }
    }

    fn block_hover(&mut self, arg: &BlockHoverAction, ctx: &mut ViewContext<Self>) {
        if self.context_menu_state.is_none() {
            match arg {
                BlockHoverAction::Begin { block_index, .. } => {
                    if let Some(hovered_index) = self.hovered_block_index {
                        if *block_index != hovered_index {
                            self.hovered_block_index = Some(*block_index);
                            ctx.notify();
                        }
                    } else {
                        self.hovered_block_index = Some(*block_index);
                        ctx.notify();
                    }
                }
                BlockHoverAction::Clear => {
                    if self.hovered_block_index.is_some()
                        // Don't clear if the user has moved the mouse over the jump to bottom of block button.
                        // This button needs special handling because it's rendered on top of the block list,
                        // not as part of it.
                        && !self.is_jump_to_bottom_of_block_element_hovered()
                    {
                        self.hovered_block_index = None;
                        ctx.notify();
                    }
                }
            }
        }
    }

    fn block_snackbar_hover(&mut self, _is_hovered: bool, ctx: &mut ViewContext<Self>) {
        ctx.notify()
    }

    fn block_near_snackbar_hover(&mut self, is_hovered: bool, ctx: &mut ViewContext<Self>) {
        self.hover_near_snackbar_area = is_hovered;
        ctx.notify()
    }

    pub fn toggle_snackbar_in_active_pane(&mut self, ctx: &mut ViewContext<Self>) {
        self.show_snackbar = !self.show_snackbar;

        ctx.notify()
    }

    fn begin_block_text_selection(
        &mut self,
        point: BlockListPoint,
        side: Side,
        selection_type: SelectionType,
        position: Vector2F,
        ctx: &mut ViewContext<Self>,
    ) {
        self.block_text_selection_start_position = Some(position);
        self.model
            .lock()
            .block_list_mut()
            .start_selection(point, selection_type, side);
        self.is_selecting = true;
        ctx.notify();
    }

    fn update_block_text_selection(
        &mut self,
        point: BlockListPoint,
        side: Side,
        delta: Lines,
        position: Vector2F,
        ctx: &mut ViewContext<Self>,
    ) {
        // When selecting blocks, there is too much noise with the mouse_dragged event,
        // causing a block selection to be mis-interpreted as a text selection. Hence,
        // we check if the move is non-trivial before resetting the block selections.
        if let Some(start_position) = self.block_text_selection_start_position {
            let (start_col, start_row) = (start_position.x(), start_position.y());
            let (curr_col, curr_row) = (position.x(), position.y());
            if (start_col - curr_col).abs() <= MIN_DELTA_FOR_TEXT_SELECTION
                && (start_row - curr_row).abs() <= MIN_DELTA_FOR_TEXT_SELECTION
            {
                return;
            } else {
                self.block_text_selection_start_position = None;
            }
        }

        self.scroll(delta, ctx);

        // Clear the selected block index on mouse drag.
        self.clear_selected_blocks(ctx);
        self.model
            .lock()
            .block_list_mut()
            .update_selection(point, side);

        ctx.notify();
    }

    pub fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    #[cfg(test)]
    pub fn clear_buffer_for_testing(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_buffer(ctx);
    }

    fn clear_buffer(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_blocks(ctx);

        // Focus the appropriate part of the terminal view (possibly a
        // long-running block, possibly the input field) depending on its
        // current state.
        self.redetermine_global_focus(ctx);

        self.model.lock().clear_screen(ClearMode::ResetAndClear);
        self.find_model.update(ctx, |find_model, ctx| {
            find_model.clear_matches(ctx);
        });

        self.block_list_mouse_states.label_mouse_states.clear();
        self.block_list_mouse_states.bookmark_mouse_states.clear();
        self.block_list_mouse_states.filter_mouse_states.clear();
        self.bookmarked_blocks.clear();

        // clearing the rich content views.

        self.rich_content_views.clear();

        // Clear screen will remove all blocks except the started block so insert
        // the label mouse state here to make sure this is handled.
        self.block_list_mouse_states
            .label_mouse_states
            .insert(BlockIndex::zero(), Default::default());
        self.block_list_mouse_states
            .bookmark_mouse_states
            .insert(BlockIndex::zero(), Default::default());
        self.block_list_mouse_states
            .filter_mouse_states
            .insert(BlockIndex::zero(), Default::default());

        self.update_find_selection(ctx);

        // don't consider the terminal view to be in an error state if we cmd+k
        // the failing block away
        if matches!(self.current_state.state, TerminalViewState::Errored) {
            self.set_current_state(TerminalViewState::Normal, ctx);
        }

        self.input.update(ctx, |input, ctx| {
            input
                .editor()
                .update(ctx, |editor, ctx| editor.clear_autosuggestion(ctx))
        });

        // Note: we set this here since clear_screen at the TerminalModel and BlockList levels is
        // called much more often (on every new session/block it seems), and we only want to track explicit
        // clear screen actions e.g. Cmd-k.
        self.model.lock().blocklist_has_been_cleared = true;
        ctx.emit(Event::BlockListCleared);

        // If we're currently in a subshell, add another flag to indicate that because we just
        // cleared the existing one.
        if let Some(session) = self
            .active_block_session_id()
            .and_then(|id| self.sessions.as_ref(ctx).get(id))
        {
            if let Some(info) = session.subshell_info() {
                self.warpify_state
                    .add_subshell_separator(info, self.model.clone(), ctx);
            }
        }

        // appear active for the terminal view anymore.
        // No more restored blocks, since we just cleared the buffer
        log::info!("Clearing buffer.  resetting any_session_contains_restored_remote_blocks");
        self.any_session_contains_restored_remote_blocks = false;

        // Since we just cleared blocks, we can just look at the state of the active block
        self.any_session_contains_remote_blocks = self.active_block_is_considered_remote(ctx);

        ctx.notify();
    }

    fn find_within_block(&mut self, ctx: &mut ViewContext<Self>) {
        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });
        self.update_find_selection(ctx);
        self.show_find_bar(ctx);
    }

    fn scroll_to_top_of_topmost_selected_block(&mut self, ctx: &mut ViewContext<Self>) {
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let block_sort_direction = input_mode.block_sort_direction();
        let sorted_ranges = self.selected_blocks.sorted_ranges(block_sort_direction);
        if let Some(block_index) = sorted_ranges
            .first()
            .and_then(|r| r.range(Some(block_sort_direction)).next())
        {
            self.update_scroll_position_locking(
                ScrollPositionUpdate::ScrollToTopOfBlock { block_index },
                ctx,
            );
        }
    }

    fn scroll_to_bottom_of_overhanging_block(
        &mut self,
        overhanging_block: &OverhangingBlock,
        ctx: &mut ViewContext<Self>,
    ) {
        self.update_scroll_position_locking(
            ScrollPositionUpdate::ScrollToBottomOfBlock {
                block_index: overhanging_block.block_index(),
            },
            ctx,
        );
    }

    fn scroll_to_bottom_of_bottommost_selected_block(&mut self, ctx: &mut ViewContext<Self>) {
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let block_sort_direction = input_mode.block_sort_direction();
        let sorted_ranges = self.selected_blocks.sorted_ranges(block_sort_direction);
        if let Some(block_index) = sorted_ranges
            .last()
            .and_then(|r| r.range(Some(block_sort_direction)).last())
        {
            self.update_scroll_position_locking(
                ScrollPositionUpdate::ScrollToBottomOfBlock { block_index },
                ctx,
            );
        }
    }

    pub fn full_prompt(&self, app: &AppContext) -> String {
        self.input.as_ref(app).prompt_and_rprompt_text(app).0
    }

    pub fn prompt_elements(&self, app: &AppContext) -> SessionNavigationPromptElements {
        self.input.as_ref(app).create_prompt_elements(app)
    }

    pub fn session_command_context(&self, _app: &AppContext) -> CommandContext {
        let model = self.model.lock();
        let block_list = model.block_list();

        let active_block = block_list.active_block();
        let last_block = block_list.last_non_hidden_block();

        match (active_block.is_active_and_long_running(), last_block) {
            // There is an active block running, so we should return the running command.
            (true, _) => CommandContext::RunningCommand {
                running_command: active_block.command_to_string(),
            },
            // There is not active block, so we try to retrieve the last non-hidden block and get its command and timestamp.
            (false, Some(last_block)) => {
                let last_run_command = last_block.command_to_string();

                let mins_since_completion = last_block.completed_ts().map(|completed_ts| {
                    let now = chrono::Local::now();
                    let diff = now.signed_duration_since(*completed_ts);
                    diff.num_minutes()
                });
                CommandContext::LastRunCommand {
                    last_run_command,
                    mins_since_completion,
                }
            }
            // There is no active block and no last non-hidden block, so it is an empty session with no CommandContext.
            (false, None) => CommandContext::None,
        }
    }

    fn cut_selected_text_from_input(&mut self, ctx: &mut ViewContext<Self>) {
        let selected_input_text = self.input.read(ctx, |input, ctx| {
            input
                .editor()
                .read(ctx, |editor, ctx| editor.selected_text(ctx))
        });

        self.input.update(ctx, |input, ctx| {
            input.editor().update(ctx, |editor, ctx| {
                editor.backspace(ctx);
            })
        });

        if !selected_input_text.is_empty() {
            ctx.clipboard()
                .write(ClipboardContent::plain_text(selected_input_text));
        }
    }

    fn copy_selected_text_from_input(&mut self, ctx: &mut ViewContext<Self>) {
        let selected_input_text = self.input.read(ctx, |input, ctx| {
            input
                .editor()
                .read(ctx, |editor, ctx| editor.selected_text(ctx))
        });

        if !selected_input_text.is_empty() {
            ctx.clipboard()
                .write(ClipboardContent::plain_text(selected_input_text));
        }
    }

    fn select_all_text_from_input(&mut self, ctx: &mut ViewContext<Self>) {
        self.input.update(ctx, |input, ctx| {
            input.editor().update(ctx, |editor, ctx| {
                editor.handle_action(&EditorAction::SelectAll, ctx)
            })
        });
    }

    fn paste_in_input(&mut self, ctx: &mut ViewContext<Self>) {
        let clipboard_content = ctx.clipboard().read();

        self.input.update(ctx, |input, ctx| {
            input.system_insert(clipboard_content.plain_text.as_str(), ctx);
            ctx.focus_self();
        });
    }

    fn command_search_from_input(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::ShowCommandSearch(Default::default()))
    }

    fn save_as_workflow_from_input(&mut self, ctx: &mut ViewContext<Self>) {
        let (all_current_input_text, selected_input_text) = self.input.read(ctx, |input, ctx| {
            input.editor().read(ctx, |editor, ctx| {
                (editor.buffer_text(ctx), editor.selected_text(ctx))
            })
        });

        let command = if selected_input_text.is_empty() {
            all_current_input_text
        } else {
            selected_input_text
        };

        self.open_workflow_modal_with_command(command, ctx);
    }

    fn toggle_input_hint_text(&mut self, ctx: &mut ViewContext<Self>) {
        InputSettings::handle(ctx).update(ctx, |input_settings, ctx| {
            report_if_error!(input_settings.show_hint_text.toggle_and_save_value(ctx));
            *input_settings.show_hint_text
        });
    }

    fn open_workflow_modal_with_command(&mut self, command: String, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::OpenWorkflowModalWithCommand(command));
    }

    fn copy_prompt(
        &mut self,
        position: &PromptPosition,
        part: &PromptPart,
        ctx: &mut ViewContext<Self>,
    ) {
        let to_copy = match part {
            PromptPart::EntirePrompt => match position {
                PromptPosition::Block(block_index) => {
                    Self::block_prompt(&self.model.lock(), self.sessions.as_ref(ctx), *block_index)
                }
                PromptPosition::Input => self.input.as_ref(ctx).prompt_and_rprompt_text(ctx).0,
            },
            PromptPart::GitBranch => position
                .block(&self.model.lock())
                .and_then(Block::git_branch)
                .cloned()
                .unwrap_or_default(),
            PromptPart::CondaContext => position
                .block(&self.model.lock())
                .and_then(Block::conda_env)
                .cloned()
                .unwrap_or_default(),
            PromptPart::Pwd => position
                .block(&self.model.lock())
                .and_then(Block::pwd)
                .cloned()
                .unwrap_or_default(),
            PromptPart::VirtualEnv => position
                .block(&self.model.lock())
                .and_then(Block::virtual_env_short_name)
                .unwrap_or_default(),
            PromptPart::ContextChip(kind) => match position {
                PromptPosition::Input => self
                    .current_prompt
                    .as_ref(ctx)
                    .latest_chip_value(kind, ctx)
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                PromptPosition::Block(_) => position
                    .block(&self.model.lock())
                    .and_then(Block::prompt_snapshot)
                    .and_then(|snapshot| snapshot.chip_value(kind))
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            },
        };
        ctx.clipboard().write(ClipboardContent::plain_text(to_copy));

        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });
        self.close_context_menu(ctx, true);
    }

    fn copy_rprompt(&mut self, ctx: &mut ViewContext<Self>) {
        let rprompt_text_option = self.input.as_ref(ctx).prompt_and_rprompt_text(ctx).1;

        if let Some(rprompt_text) = rprompt_text_option {
            ctx.clipboard()
                .write(ClipboardContent::plain_text(rprompt_text));
        }
    }

    fn edit_prompt(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::OpenPromptEditor);
    }

    fn show_find_bar(&mut self, ctx: &mut ViewContext<Self>) {
        let model = self.model.lock();
        let inverted_blocklist = self.is_inverted_blocklist(ctx);
        self.find_bar.update(ctx, |view, ctx| {
            let semantic_selection = SemanticSelection::as_ref(ctx);
            if let Some(selected) =
                model.selection_to_string(semantic_selection, inverted_blocklist, ctx)
            {
                if !selected.is_empty() {
                    view.set_query_text(selected.as_str(), ctx);
                }
            }

            // If the alt screen is not active and there are selected blocks, enable the find_within_block.
            view.display_find_within_block = match (
                model.is_alt_screen_active(),
                self.selected_blocks.is_empty(),
            ) {
                (false, false) => FindWithinBlockState::Enabled,
                (false, true) => FindWithinBlockState::Disabled,
                (true, _) => FindWithinBlockState::Hidden,
            };

            ctx.notify();
        });
        drop(model);

        self.find_model.update(ctx, |find_model, _ctx| {
            find_model.set_is_find_bar_open(true);
        });

        let options = self
            .find_model
            .as_ref(ctx)
            .active_find_options()
            .cloned()
            .unwrap_or_default();
        // Start find using the previous query.
        self.run_find(options, ctx);
        self.focus_find_bar(ctx);
    }

    fn close_find_bar(&mut self, ctx: &mut ViewContext<Self>) {
        self.find_model.update(ctx, |find_model, _ctx| {
            find_model.set_is_find_bar_open(false);
        });
        ctx.notify();
    }

    fn update_find_selection(&mut self, ctx: &mut ViewContext<Self>) {
        if self.find_model.as_ref(ctx).is_find_bar_open()
            && !self.model.lock().is_alt_screen_active()
        {
            let mut find_options = self
                .find_model
                .as_ref(ctx)
                .active_find_options()
                .cloned()
                .unwrap_or_default();

            let new_blocks_to_include_in_results = matches!(
                self.find_bar.as_ref(ctx).display_find_within_block,
                FindWithinBlockState::Enabled
            )
            .then(|| self.selected_blocks.block_indices().collect_vec());

            if find_options.blocks_to_include_in_results.as_ref()
                != new_blocks_to_include_in_results.as_ref()
            {
                self.find_bar.update(ctx, |view, ctx| {
                    if new_blocks_to_include_in_results.is_none() {
                        // If there aren't any selected blocks, turn off find in block
                        view.display_find_within_block = FindWithinBlockState::Disabled;
                    }

                    find_options = find_options
                        .with_blocks_to_include_in_results(new_blocks_to_include_in_results);

                    self.find_model.update(ctx, |find_model, ctx| {
                        find_model.run_find(find_options, ctx)
                    });
                    ctx.notify();
                });
            }
        }
    }

    fn toggle_find_within_block(
        &mut self,
        ctx: &mut ViewContext<Self>,
        enable_find_in_block: bool,
    ) {
        if enable_find_in_block && self.selected_blocks.is_empty() {
            // If a block isn't selected, auto select the most recent block
            self.select_most_recent_blocks(1, ctx);
        } else {
            self.update_find_selection(ctx);
        }
    }

    /// Starts finding the matches for the given query string from the most recent block.
    /// Sets the focused match to the first match in the terminal or doesn't update it.
    /// Note that the meaning of "first" varies depending on whether the block list is inverted
    /// or not.
    fn run_find(&mut self, mut options: FindOptions, ctx: &mut ViewContext<Self>) {
        let blocks_to_include_in_results = matches!(
            self.find_bar.as_ref(ctx).display_find_within_block,
            FindWithinBlockState::Enabled
        )
        .then(|| self.selected_blocks.block_indices());
        options = options.with_blocks_to_include_in_results(blocks_to_include_in_results);

        self.find_model
            .update(ctx, |find_model, ctx| find_model.run_find(options, ctx));

        // Scroll terminal view to the focused match, if any.
        self.scroll_to_match(ctx);

        ctx.notify();
    }

    fn goto_next_find_match(&mut self, direction: &FindDirection, ctx: &mut ViewContext<Self>) {
        self.find_model.update(ctx, |find_model, ctx| {
            find_model.focus_next_find_match(*direction, ctx);
        });
        self.scroll_to_match(ctx);
        ctx.notify();
    }

    fn select_most_recent_blocks(&mut self, count: usize, ctx: &mut ViewContext<Self>) {
        if count == 0 {
            self.clear_selected_blocks(ctx);
            return;
        }

        let indices = {
            let terminal_model = self.model.lock();
            (
                terminal_model
                    .block_list()
                    .first_non_hidden_block_by_index(),
                terminal_model.block_list().last_non_hidden_block_by_index(),
            )
        };
        let (Some(first_block_index), Some(last_block_index)) = indices else {
            return;
        };

        let start_index = usize::from(last_block_index)
            .saturating_sub(count - 1)
            .max(usize::from(first_block_index));
        let last_index = usize::from(last_block_index);
        self.change_block_selections(
            |selected_blocks| {
                selected_blocks
                    .reset_to_block_indices((start_index..=last_index).map(BlockIndex::from));
            },
            ctx,
        );

        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockSelect),
                tips,
                ctx,
            );
            ctx.notify();
        });

        // submit queries quickly, we don't want to divert the focus away from the input box. With

        self.scroll_to_if_not_visible(last_block_index, ctx);

        if let Some(accessibility_contents) =
            self.selected_block_accessibility_content(last_block_index)
        {
            ctx.emit_a11y_content(accessibility_contents);
        }
        ctx.notify();
    }

    fn select_less_recent_block(&mut self, is_shift_down: bool, ctx: &mut ViewContext<Self>) {
        if self.is_context_menu_open() {
            self.close_context_menu(ctx, true);
        }

        if let Some(selected_block_index) = self.selected_blocks.tail() {
            let new_block_index = self
                .model
                .lock()
                .block_list()
                .prev_non_hidden_block_from_index(selected_block_index /* from_index */)
                .unwrap_or(selected_block_index);

            if is_shift_down {
                self.change_block_selections(
                    |selected_blocks| {
                        selected_blocks.range_select(new_block_index);
                    },
                    ctx,
                );
            } else {
                self.reset_selection_to_single_block(new_block_index, ctx);
            }

            self.scroll_to_if_not_visible(new_block_index, ctx);
            ctx.notify();

            self.tips_completed.update(ctx, |tips, ctx| {
                mark_feature_used_and_write_to_user_defaults(
                    Tip::Hint(TipHint::BlockSelect),
                    tips,
                    ctx,
                );
                ctx.notify();
            });
        } else {
            self.select_most_recent_blocks(1, ctx);
        }
    }

    fn select_more_recent_block(
        &mut self,
        is_cmd_down: bool,
        is_shift_down: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.is_context_menu_open() {
            self.close_context_menu(ctx, true);
        }
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let is_inverted_blocklist = input_mode.is_inverted_blocklist();
        let is_most_recent_block_visible = {
            let model = self.model.lock();
            let block_list = model.block_list();
            let viewport = self.viewport_state(block_list, input_mode, ctx);
            if is_inverted_blocklist {
                viewport.is_most_recent_block_in_view(BlockVisibilityMode::TopOfBlockVisible)
            } else {
                viewport.is_most_recent_block_in_view(BlockVisibilityMode::BottomOfBlockVisible)
            }
        };
        let is_long_running_command = {
            self.model
                .lock()
                .block_list()
                .active_block()
                .is_active_and_long_running()
        };
        if let Some(selected_block_index) = self.selected_blocks.tail() {
            let new_block_index = {
                self.model
                    .lock()
                    .block_list()
                    .next_non_hidden_block_from_index(selected_block_index /* from_index */)
                    .unwrap_or(selected_block_index)
            };

            if new_block_index != selected_block_index {
                if is_shift_down {
                    self.change_block_selections(
                        |selected_blocks| {
                            selected_blocks.range_select(new_block_index);
                        },
                        ctx,
                    );
                } else {
                    self.reset_selection_to_single_block(new_block_index, ctx);
                }
                self.scroll_to_if_not_visible(new_block_index, ctx);
                self.tips_completed.update(ctx, |tips, ctx| {
                    mark_feature_used_and_write_to_user_defaults(
                        Tip::Hint(TipHint::BlockSelect),
                        tips,
                        ctx,
                    );
                    ctx.notify();
                });
            } else if !is_most_recent_block_visible {
                // Scroll to the bottom if the index hasn't changed.
                // This happens when there is a second arrow down when the bottom
                // block is selected.
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::ScrollMostRecentBlockIntoView,
                    ctx,
                );
            } else if is_cmd_down && !is_long_running_command {
                // Focus the input box if the keystroke is cmd-down and we are already at the
                // most recent block (unless it's a long running command, in which case we leave
                // the selection as is.
                self.clear_selected_blocks(ctx);
                ctx.focus(&self.input);
            }
            ctx.notify();
        }
    }

    fn select_all_blocks(&mut self, ctx: &mut ViewContext<Self>) {
        let first_block_index = self
            .model
            .lock()
            .block_list()
            .first_non_hidden_block_by_index();
        let last_block_index = self
            .model
            .lock()
            .block_list()
            .last_non_hidden_block_by_index();

        if let Some(start_index) = first_block_index {
            if let Some(end_index) = last_block_index {
                self.change_block_selections(
                    |selected_blocks| {
                        selected_blocks.reset_to_single(start_index);
                        selected_blocks.range_select(end_index);
                    },
                    ctx,
                );
            }
        }
        ctx.focus_self();
        ctx.notify();
    }

    fn focus_terminal(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
        ctx.notify();
    }

    fn reset_selection_to_single_block(
        &mut self,
        block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        self.change_block_selections(
            |selected_blocks| {
                selected_blocks.reset_to_single(block_index);
            },
            ctx,
        );
        ctx.notify();
    }

    fn clear_selected_blocks(&mut self, ctx: &mut ViewContext<Self>) {
        self.change_block_selections(
            |selected_blocks| {
                selected_blocks.reset();
            },
            ctx,
        );
        ctx.notify();
    }

    /// context, etc.). Never invoke `block_list_mut().clear_selection()` elsewhere on its own.
    fn clear_selected_text(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_text_except(None, ctx);
    }

    /// context, etc.). Never invoke `block_list_mut().clear_selection()` elsewhere on its own.
    ///
    /// Provides the option of keeping the existing text selection for one rich content view, whose
    /// view ID must be passed in via `exempt_rich_content_view_id`. This is helpful for ensuring that
    /// text selections don't simultaneously exist on unrelated views (i.e. a regular command block
    /// and a suggested plan).
    fn clear_selected_text_except(
        &mut self,
        exempt_rich_content_view_id: Option<EntityId>,
        ctx: &mut ViewContext<Self>,
    ) {
        // The below function clears all text selections within the underlying `TerminalModel`:
        // - Text selection rendering on regular blocks is tied to the underlying model.
        // - Text selection rendering on rich content blocks is **not** tied to the underlying model.
        //
        // Thus, not only is invoking `clear_selection()` on its own insufficient for clearing all
        // on-screen text selections, but the invocation must also be followed by supplementary logic
        // to clear visual text selections on rich content views.
        //
        // This also explains why we don't invoke this function unless we're attempting to clear
        // **all** selected text; because rich content text copying will stop working otherwise.
        if exempt_rich_content_view_id.is_none() {
            self.model.lock().block_list_mut().clear_selection();
        }

        // except for the view with a matching view ID.

        // Clear all selected text within rich content block view sub-hierarchies,
        // except for the rich content block with a matching view ID.
        for rich_content in self.rich_content_views.iter() {
            match rich_content.metadata() {
                Some(RichContentMetadata::EnvVarCollectionBlock {
                    env_var_collection_block_handle,
                    ..
                }) => {
                    if exempt_rich_content_view_id
                        .is_some_and(|view_id| env_var_collection_block_handle.id() == view_id)
                    {
                        continue;
                    }
                    env_var_collection_block_handle.update(ctx, |env_var_collection_block, ctx| {
                        env_var_collection_block.clear_selection(ctx);
                    });
                }
                Some(RichContentMetadata::WarpifySuccessBlock { .. }) => {
                    // TODO(Simon): We should be checking for WarpifySuccessBlocks here as well.
                    // The `WarpifySuccessBlock` implements a `SelectableArea`.
                }
                _ => {}
            }
        }

        // When this function is invoked because of an ongoing text selection within a nested
        // rich content view component (i.e. `CodeEditorView`), setting `is_selecting` to false
        // will prevent the selection from "spilling" into neighbouring blocks.
        self.is_selecting = false;

        // TODO(Simon): This doesn't work as intended for nested inline SelectableAreas.
        // This includes inline action headers, requested commands, and env var collection blocks.
        // The reasoning behind this is that `SelectableArea`s don't produce selected text until
        // the selection is **complete**, but `clear_selected_text_except` is only invoked while
        // nested selections are **ongoing**.
        self.maybe_copy_selection_to_clipboard(ctx);
    }

    fn clear_selections(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_blocks(ctx);
        self.clear_selected_text(ctx);
        self.focus_input_box(ctx);
        ctx.notify();
    }

    fn focus_input_and_clear_selections(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_text(ctx);
        self.focus_input_box(ctx);
        ctx.notify();
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    fn clear_selections_without_focusing_input(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_blocks(ctx);
        self.clear_selected_text(ctx);
        ctx.notify();
    }

    fn focus_input_box(&mut self, ctx: &mut ViewContext<Self>) {
        self.clear_selected_blocks(ctx);
        self.clear_selected_text(ctx);
        self.update_find_selection(ctx);
        ctx.focus(&self.input);
        ctx.notify();
    }

    fn focus_find_bar(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.find_bar);
        ctx.notify();
    }

    fn focus_block_filter_editor(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus(&self.block_filter_editor);
        ctx.notify();
    }

    /// Returns the last block's `EnvVarCollectionBlock` if it is uncompleted, scoped to the
    /// terminal session.
    fn active_env_var_collection_block(
        &self,
        ctx: &AppContext,
    ) -> Option<&ViewHandle<EnvVarCollectionBlock>> {
        self.rich_content_views.iter().find_map(|rich_content| {
            if let Some(RichContentMetadata::EnvVarCollectionBlock {
                env_var_collection_block_handle,
            }) = rich_content.metadata()
            {
                return (!env_var_collection_block_handle
                    .as_ref(ctx)
                    .is_block_completed())
                .then_some(env_var_collection_block_handle);
            }
            None
        })
    }

    /// Examines the local state of the [`TerminalView`] and chooses where best to assign focus.
    ///
    /// WARNING: this can steal focus even when the user is working in a separate terminal view!
    /// Consider using [`Self::redetermine_terminal_focus`] instead.
    ///
    /// WARNING: this method takes a lock on the TerminalModel.
    /// Caller must ensure the model is not already locked!
    ///
    /// TODO: https://linear.app/warpdotdev/issue/CORE-277
    pub fn redetermine_global_focus(&mut self, ctx: &mut ViewContext<Self>) {
        if self.context_menu_state.is_some() {
            return;
        }
        self.last_focus_ts = Some(Local::now().naive_local());
        let should_focus_terminal = {
            let model = self.model.lock();
            let has_input = self.is_input_box_visible(&model, ctx);
            (!has_input && model.is_alt_screen_active())
                || (model
                    .block_list()
                    .active_block()
                    .is_active_and_long_running()
                    && (!has_input || !model.block_list().is_bootstrapping_precmd_done()))
                || !self.selected_blocks.is_empty()
                || model
                    .selection_to_string(SemanticSelection::as_ref(ctx), false, ctx)
                    .is_some_and(|text| !text.is_empty())
        };
        if should_focus_terminal {
            self.focus_terminal(ctx);
        } else if let Some(block) = self.active_env_var_collection_block(ctx) {
            ctx.focus(block);
        } else {
            self.focus_input_box(ctx);
        }
    }

    fn close_context_menu(&mut self, ctx: &mut ViewContext<Self>, should_redetermine_focus: bool) {
        if self.context_menu_state.is_some() {
            self.context_menu_state = None;
            ctx.notify();
            if should_redetermine_focus {
                self.redetermine_global_focus(ctx);
            }
        }
    }

    fn context_menu_insert_selected_text(&mut self, ctx: &mut ViewContext<Self>) {
        {
            let semantic_selection = SemanticSelection::as_ref(ctx);
            // Note: we purposely separate this expression here, to avoid locking the TerminalModel for the duration of the `if let`
            // block, since downstream functions may need the lock (`Input::insert_internal`).
            let selected_text = self.model.lock().selection_to_string(
                semantic_selection,
                self.is_inverted_blocklist(ctx),
                ctx,
            );
            if let Some(selected_text) = selected_text {
                // We put everything from the selection into the input box, even
                // if it includes non-printable characters. Note that this is
                // important to handle new lines appropriately.
                self.input.update(ctx, |input, ctx| {
                    input.system_insert(&selected_text, ctx);
                    ctx.focus_self();
                })
            }
        }
        self.close_context_menu(ctx, true);
    }

    fn input_command(&mut self, ctx: &mut ViewContext<Self>, command: String) {
        self.input.update(ctx, |input, ctx| {
            input.replace_buffer_content((command).trim(), ctx);
            ctx.focus_self();
        });
    }

    fn reinput_commands(&mut self, as_root: bool, ctx: &mut ViewContext<Self>) {
        if !self.selected_blocks.is_empty() {
            let mut commands = vec![];
            self.with_non_hidden_selected_blocks(
                |block| {
                    let command_str = block.command_to_string();
                    if !command_str.trim().is_empty() {
                        if as_root {
                            commands.push(format!("sudo {command_str}"));
                        } else {
                            commands.push(command_str);
                        }
                    }
                },
                ctx,
            );
            self.input_command(ctx, commands.join("\n"));
            self.focus_input_box(ctx);
        }
    }

    fn context_menu_copy_blocks(&mut self, ctx: &mut ViewContext<Self>) {
        self.copy_blocks(BlockEntity::CommandAndOutput, ctx);
    }

    fn context_menu_copy_block_commands(&mut self, ctx: &mut ViewContext<Self>) {
        self.copy_blocks(BlockEntity::Command, ctx);
    }

    fn context_menu_copy_block_outputs(&mut self, ctx: &mut ViewContext<Self>) {
        self.copy_blocks(BlockEntity::Output, ctx);
    }

    fn context_menu_copy_filtered_block_outputs(&mut self, ctx: &mut ViewContext<Self>) {
        self.copy_blocks(BlockEntity::FilteredOutput, ctx);
    }

    fn context_menu_copy_url(&mut self, url_content: &str, ctx: &mut ViewContext<Self>) {
        ctx.clipboard()
            .write(ClipboardContent::plain_text(url_content.to_string()));
        self.close_context_menu(ctx, true);
    }

    fn num_non_hidden_selected_blocks(&self) -> usize {
        let model = self.model.lock();
        self.selected_blocks
            .ranges()
            .iter()
            .flat_map(|range| range.range(None))
            .filter(|block_index| {
                model
                    .block_list()
                    .block_at(*block_index)
                    .is_some_and(|block| !block.is_empty())
            })
            .count()
    }

    fn with_non_hidden_selected_blocks<T>(&mut self, mut action: T, ctx: &mut ViewContext<Self>)
    where
        T: FnMut(&Block),
    {
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        let sort_direction = input_mode.block_sort_direction();
        let model = self.model.lock();
        let sorted_ranges = self.selected_blocks.sorted_ranges(sort_direction);
        for selection_range in sorted_ranges {
            for block_index in selection_range.range(Some(sort_direction)) {
                if let Some(block) = model
                    .block_list()
                    .block_at(block_index)
                    .filter(|block| !block.is_empty())
                {
                    action(block);
                }
            }
        }
    }

    fn copy_blocks(&mut self, entity: BlockEntity, ctx: &mut ViewContext<Self>) {
        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });

        let selected_block_contents = self.selected_block_contents_as_string(entity, "\n", ctx);
        ctx.clipboard()
            .write(ClipboardContent::plain_text(selected_block_contents));
        self.close_context_menu(ctx, true);
    }

    fn context_menu_copy_selected_text(&mut self, ctx: &mut ViewContext<Self>) {
        {
            let semantic_selection = SemanticSelection::as_ref(ctx);
            let model = self.model.lock();
            if let Some(selected_text) =
                model.selection_to_string(semantic_selection, self.is_inverted_blocklist(ctx), ctx)
            {
                ctx.clipboard()
                    .write(ClipboardContent::plain_text(selected_text));
            }
        }
        self.close_context_menu(ctx, true);
    }

    fn selected_block_contents_as_string(
        &mut self,
        entity: BlockEntity,
        separator: &str,
        ctx: &mut ViewContext<Self>,
    ) -> String {
        let mut block_strs = vec![];
        self.with_non_hidden_selected_blocks(
            |block| {
                let block_str = match entity {
                    BlockEntity::Command => block.command_to_string(),
                    BlockEntity::Output => block.output_to_string_force_full_grid_contents(),
                    BlockEntity::CommandAndOutput => format!(
                        "{}\n{}",
                        block.command_to_string(),
                        block.output_to_string(),
                    ),
                    BlockEntity::FilteredOutput => block.output_to_string(),
                };

                if !block_str.trim().is_empty() {
                    block_strs.push(block_str);
                }
            },
            ctx,
        );

        block_strs.join(separator)
    }

    fn handle_menu_event(&mut self, event: &MenuEvent, ctx: &mut ViewContext<Self>) {
        if let MenuEvent::Close { via_select_item } = event {
            self.close_context_menu(ctx, !*via_select_item);
        }
    }

    fn bookmark_selected_block(&mut self, ctx: &mut ViewContext<Self>) {
        self.tips_completed.update(ctx, |tips, ctx| {
            mark_feature_used_and_write_to_user_defaults(
                Tip::Hint(TipHint::BlockAction),
                tips,
                ctx,
            );
            ctx.notify();
        });
        if let Some(selected_block_index) = self.selected_blocks.tail() {
            self.bookmark_block(&selected_block_index, ctx);
            ctx.notify();
        }
    }

    fn bookmark_block(&mut self, index: &BlockIndex, ctx: &mut ViewContext<Self>) {
        match self.bookmarked_blocks.entry(*index) {
            Entry::Occupied(occupied) => {
                occupied.remove();
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Default::default());
            }
        };

        ctx.notify();
    }

    fn is_navigated_away_from_window(&self, ctx: &mut ViewContext<Self>) -> bool {
        let active_window = ctx.windows().active_window();
        Some(ctx.window_id()) != active_window
    }

    fn is_block_active_and_running(&self, model: &TerminalModel, block_index: BlockIndex) -> bool {
        let active_block = model.block_list().active_block();
        active_block.index() == block_index && active_block.is_active_and_long_running()
    }

    /// If password notification settings enabled, send a notification.
    /// Otherwise, set the banner trigger so that we show the banner the next
    /// time a block completes.
    pub fn maybe_send_password_notification(
        &mut self,
        block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        let model = self.model.lock();
        let active_block = model.block_list().active_block();
        let notification_settings = SessionSettings::as_ref(ctx).notifications.value().clone();

        // The active block could have changed before we send the notification
        // so double check before sending
        if self.is_block_active_and_running(&model, block_index) {
            match notification_settings.mode {
                NotificationsMode::Enabled if notification_settings.is_needs_attention_enabled => {
                    let password_trigger = NotificationsTrigger::NeedsAttention;
                    let notification_content = password_trigger.create_notification_content(
                        active_block.command_to_string(),
                        "Command is waiting for a password".to_string(),
                    );
                    ctx.emit(Event::SendNotification(notification_content));
                }
                NotificationsMode::Unset
                    if matches!(
                        self.inline_banners_state.notifications_discovery_banner,
                        NotificationsDiscoveryBanner::Unset
                    ) =>
                {
                    // if the user hasn't configured notifications before and there isn't already
                    // a banner, we should add the banner once the block completes
                    self.inline_banners_state.notifications_discovery_banner =
                        NotificationsDiscoveryBanner::Triggered(
                            NotificationsTrigger::NeedsAttention,
                        );
                }
                _ => {}
            }
        }
    }

    fn handle_input_event(&mut self, event: &InputEvent, ctx: &mut ViewContext<Self>) {
        match event {
            InputEvent::ExecuteCommand(event) => {
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterCommandExecutionStarted,
                    ctx,
                );
                if let Some(active_session) = self
                    .active_block_session_id()
                    .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
                {
                    active_session.cancel_active_commands();
                }

                // Don't steal focus from other parts of the app.
                if ctx.is_self_or_child_focused() {
                    self.focus_terminal(ctx);
                }

                ctx.emit(Event::ExecuteCommand(event.as_ref().clone()));
            }

            InputEvent::ClearSelectedBlock => self.clear_selected_blocks(ctx),
            InputEvent::SelectRecentBlocks { count } => self.select_most_recent_blocks(*count, ctx),
            InputEvent::Copy => self.copy(ctx),
            InputEvent::UnhandledModifierKeyOnEditor(_) => {}
            InputEvent::ClearSelections => self.clear_selections(ctx),
            InputEvent::AutosuggestionAccepted => {
                // blocks we need to render the border for.
                ctx.notify()
            }

            InputEvent::Escape => {
                if self.has_active_cli_agent_input_session(ctx) {
                    self.close_cli_agent_rich_input_and_disable_auto_toggle(ctx);
                    return;
                }

                // Ignore any passive blocks on escape.

                ctx.emit(Event::Escape)
            }

            InputEvent::SyncInput(input) => {
                if !SyncedInputState::as_ref(ctx).is_syncing_any_inputs(ctx.window_id()) {
                    return;
                }

                match input {
                    SyncInputType::InputEditorContentsChanged { contents, .. } => {
                        ctx.emit(Event::SyncInput(SyncEvent {
                            source_view_id: self.view_id,
                            data: SyncInputType::InputEditorContentsChanged {
                                contents: contents.clone(),
                            },
                        }));
                    }
                    SyncInputType::RanCommand => {
                        ctx.emit(Event::SyncInput(SyncEvent {
                            source_view_id: self.view_id,
                            data: SyncInputType::RanCommand,
                        }));
                    }
                    // Terminal Inputs should only be sending
                    // InputEditorContentsChanged and RanCommand events.
                    _ => (),
                }
            }
            InputEvent::ShowCommandSearch(options) => {
                ctx.emit(Event::ShowCommandSearch(options.clone()));
            }
            InputEvent::CtrlD => {
                ctx.emit(Event::CtrlD);
            }
            InputEvent::CtrlC { cleared_buffer_len } => {
                self.handle_ctrl_c_input_event(*cleared_buffer_len, ctx);
            }
            InputEvent::EmacsBindingUsed => {
                if OperatingSystem::get().is_linux() && self.should_show_emacs_bindings_banner(ctx)
                {
                    self.show_emacs_bindings_banner(ctx);
                }
            }
            InputEvent::EditorUpdated {
                block_id,
                operations,
            } => {
                ctx.emit(Event::InputEditorUpdated {
                    block_id: block_id.clone(),
                    operations: operations.clone(),
                });
            }
            InputEvent::InputFocusedFromMiddleClick => {
                self.focus_input_box(ctx);
            }
            InputEvent::EditorFocused => {
                ctx.dispatch_typed_action(&PaneGroupAction::HandleFocusChange);
                ctx.notify();
            }
            #[cfg(feature = "local_fs")]
            InputEvent::OpenCodeInWarp { source, layout } => {
                ctx.emit(Event::OpenCodeInWarp {
                    source: source.clone(),
                    layout: *layout,
                });
            }
            InputEvent::OpenCodeReviewPane => {
                ctx.emit(Event::OpenCodeReviewPane(CodeReviewPanelArg {
                    repo_path: self.current_repo_path.clone(),
                    terminal_view: self.view_handle.clone(),
                    entrypoint: CodeReviewPaneEntrypoint::GitDiffChip,
                    focus_new_pane: true,
                    cli_agent: None,
                }));
            }

            InputEvent::OpenFilesPalette { source } => {
                ctx.emit(Event::OpenFilesPalette { source: *source })
            }

            InputEvent::SubmitCLIAgentInput { text } => {
                self.submit_cli_agent_rich_input(text.clone(), ctx);
            }

            InputEvent::ShowToast { message, flavor } => {
                ctx.emit(Event::ShowToast {
                    message: message.clone(),
                    flavor: *flavor,
                });
            }

            InputEvent::RegisterPluginListener(agent) => {
                self.register_cli_agent_listener_without_session_start_event(*agent, ctx);
            }
            InputEvent::OpenPluginInstructionsPane(agent, kind) => {
                ctx.emit(Event::OpenPluginInstructionsPane(*agent, *kind));
            }
        }
    }

    fn handle_find_event(&mut self, event: &FindEvent, ctx: &mut ViewContext<Self>) {
        match event {
            FindEvent::CloseFindBar => {
                self.close_find_bar(ctx);
                self.redetermine_global_focus(ctx);
            }
            FindEvent::Update { query } => {
                let options = self
                    .find_model
                    .as_ref(ctx)
                    .active_find_options()
                    .cloned()
                    .unwrap_or_default()
                    .with_query(query.clone());
                self.run_find(options, ctx)
            }
            FindEvent::NextMatch { direction } => self.goto_next_find_match(direction, ctx),
            FindEvent::ToggleFindInBlock { value } => self.toggle_find_within_block(ctx, *value),
            FindEvent::ToggleCaseSensitivity { is_case_sensitive } => {
                let options = self
                    .find_model
                    .as_ref(ctx)
                    .active_find_options()
                    .cloned()
                    .unwrap_or_default()
                    .with_is_case_sensitive(*is_case_sensitive);
                self.run_find(options, ctx)
            }
            FindEvent::ToggleRegexSearch { is_regex_enabled } => {
                let options = self
                    .find_model
                    .as_ref(ctx)
                    .active_find_options()
                    .cloned()
                    .unwrap_or_default()
                    .with_is_regex_enabled(*is_regex_enabled);
                self.run_find(options, ctx)
            }
        }
    }

    fn update_block_filter_for_block_with_active_editor(
        &mut self,
        block_filter_query: &BlockFilterQuery,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(active_filter_editor_block_index) = self.active_filter_editor_block_index else {
            log::warn!(
                "Tried to update block filter query without active_filter_editor_block_index set"
            );
            return;
        };

        self.update_block_filter_for_block(
            active_filter_editor_block_index,
            block_filter_query,
            ctx,
        );
    }

    /// Caches the scroll position before a filter is applied, if the filter is
    /// being applied from a zero-state. This cached scroll position is used to
    /// return users to their original scroll position when the filter is removed.
    fn maybe_cache_scroll_position_before_filter(
        &self,
        block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        let mut model = self.model.lock();

        let prev_filter_query = model.get_filter_on_block(block_index);
        // Only cache the scroll position when applying a filter from a zero state.
        if !prev_filter_query.is_some_and(|query| query.is_active_and_nonempty()) {
            let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
            let viewport = self.viewport_state(model.block_list(), input_mode, ctx);
            let top_of_viewport = viewport.scroll_top_in_lines();
            let top_of_block = viewport.top_of_block_in_lines(block_index);
            let bottom_of_block = viewport.bottom_of_block_in_lines(block_index);
            // Only cache the position if the block is in the viewport.
            if height_in_range_approx(top_of_viewport, top_of_block, bottom_of_block) {
                let offset_from_block_top = top_of_viewport - top_of_block;
                model
                    .block_list_mut()
                    .set_scroll_position_before_filter(block_index, offset_from_block_top);
            }
        }
    }

    /// Set the scroll position after a filter is applied/updated. If the block
    /// is returning to a non-filtered state, we try to return the user to their
    /// original scroll position. Otherwise, we make a best effort to show the
    /// users the same lines they were seeing before a filter.
    fn update_scroll_position_after_filter(
        &mut self,
        block_index: BlockIndex,
        block_filter_query: &BlockFilterQuery,
        prev_top_of_viewport: Lines,
        prev_bottom_of_block: Lines,
        prev_first_visible_original_row: Option<usize>,
        ctx: &mut ViewContext<Self>,
    ) {
        if !block_filter_query.is_active_and_nonempty() {
            let cached_scroll_position = self
                .model
                .lock()
                .block_list()
                .scroll_position_before_filter();
            if let Some(scroll_position) = cached_scroll_position {
                self.update_scroll_position_locking(
                    ScrollPositionUpdate::AfterFilterClear {
                        block_index: scroll_position.block_index,
                        offset_from_block_top: scroll_position.offset_from_block_top,
                    },
                    ctx,
                );
                self.model
                    .lock()
                    .block_list_mut()
                    .clear_scroll_position_before_filter();
                return;
            }
        }

        self.update_scroll_position_locking(
            ScrollPositionUpdate::AfterFilter {
                block_index,
                prev_top_of_viewport,
                prev_bottom_of_block,
                prev_first_visible_original_row,
            },
            ctx,
        );
    }

    fn update_block_filter_for_block(
        &mut self,
        block_index: BlockIndex,
        block_filter_query: &BlockFilterQuery,
        ctx: &mut ViewContext<Self>,
    ) {
        self.maybe_cache_scroll_position_before_filter(block_index, ctx);

        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        // Fetch some state of the current viewport before the filter is applied.
        let (prev_top_of_viewport, prev_bottom_of_block, prev_first_visible_original_row) = {
            let model = self.model.lock();
            let viewport = self.viewport_state(model.block_list(), input_mode, ctx);
            (
                viewport.scroll_top_in_lines(),
                viewport.bottom_of_block_in_lines(block_index),
                viewport.get_first_visible_output_row(block_index),
            )
        };

        if block_filter_query.query.is_empty() {
            self.model.lock().clear_filter_on_block(block_index);
        } else {
            self.model
                .lock()
                .update_filter_on_block(block_index, block_filter_query.clone());
        };
        self.find_model.update(ctx, |find_model, ctx| {
            log::info!("Updating matches for filtered block.");
            find_model.update_matches_for_filtered_block(block_index, ctx);
        });

        let num_matched_lines = self
            .model
            .lock()
            .block_list()
            .num_matched_lines_in_filter_for_block(block_index);

        self.block_filter_editor.update(ctx, |filter_editor, ctx| {
            filter_editor.set_num_matched_lines(num_matched_lines);
            ctx.notify();
        });

        self.update_scroll_position_after_filter(
            block_index,
            block_filter_query,
            prev_top_of_viewport,
            prev_bottom_of_block,
            prev_first_visible_original_row,
            ctx,
        );

        ctx.notify();
    }

    fn handle_block_filter_event(
        &mut self,
        event: &BlockFilterEditorEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            BlockFilterEditorEvent::UpdateFilter(block_filter_state) => {
                self.update_block_filter_for_block_with_active_editor(block_filter_state, ctx);
            }
            BlockFilterEditorEvent::Close => {
                self.close_block_filter_editor(ctx);
                self.redetermine_global_focus(ctx);
            }
        }
    }

    fn handle_slow_bootstrap_banner_event(
        &mut self,
        event: &BannerEvent<TerminalAction>,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            BannerEvent::Dismiss { .. } => self.hide_slow_bootstrap_banner(ctx),
            BannerEvent::Action(terminal_action) => {
                self.handle_action(terminal_action, ctx);
            }
        }
    }

    fn show_osc52_clipboard_blocked_banner(
        &mut self,
        blocked_type: Osc52ClipboardBlockedType,
        ctx: &mut ViewContext<Self>,
    ) {
        if self.osc52_clipboard_banner_suppressed {
            return;
        }
        // Dedup: if already showing the same type, do not re-show.
        if self.osc52_clipboard_blocked_type == Some(blocked_type) {
            return;
        }
        let text = match blocked_type {
            Osc52ClipboardBlockedType::Write => {
                "A terminal program tried to write to your clipboard. This is disabled by default for security reasons, to protect against malicious software."
            }
            Osc52ClipboardBlockedType::Read => {
                "A terminal program tried to read your clipboard. This is disabled by default for security reasons, to protect against malicious software."
            }
        };
        let button_label = match blocked_type {
            Osc52ClipboardBlockedType::Write => "Allow clipboard writes",
            Osc52ClipboardBlockedType::Read => "Allow clipboard reads and writes",
        };
        self.osc52_clipboard_blocked_banner
            .update(ctx, |banner, ctx| {
                banner.set_content(BannerTextContent::plain_text(text), ctx);
                banner.set_action_button_label(0, button_label, ctx);
            });
        self.osc52_clipboard_blocked_type = Some(blocked_type);
        ctx.notify();
    }

    fn handle_osc52_clipboard_banner_event(
        &mut self,
        event: &BannerEvent<TerminalAction>,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            BannerEvent::Dismiss(DismissalType::Temporary) => {
                self.osc52_clipboard_blocked_type = None;
                ctx.notify();
            }
            BannerEvent::Dismiss(DismissalType::Permanent) => {
                self.osc52_clipboard_blocked_type = None;
                self.osc52_clipboard_banner_suppressed = true;
                let _ = ctx
                    .private_user_preferences()
                    .write_value(OSC52_CLIPBOARD_BANNER_SUPPRESSED_KEY, "true".to_owned());
                ctx.notify();
            }
            BannerEvent::Action(terminal_action) => {
                self.handle_action(terminal_action, ctx);
            }
        }
    }

    fn handle_incompatible_configuration_banner_event(
        &mut self,
        event: &BannerEvent<TerminalAction>,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            BannerEvent::Dismiss { .. } => {
                self.is_incompatible_configuration_banner_open = false;
                ctx.notify();
            }
            BannerEvent::Action(_) => {
                #[cfg(debug_assertions)]
                log::warn!("Incomptabile configuration banner does not support handling actions");
            }
        }
    }

    /// Whether the incompatible shell configuration banner is open.
    pub fn is_incompatible_configuration_banner_open(&self) -> bool {
        self.is_incompatible_configuration_banner_open
    }

    fn handle_emacs_bindings_banner_clicked(
        &mut self,
        event: &BannerEvent<TerminalAction>,
        ctx: &mut ViewContext<Self>,
    ) {
        if matches!(event, BannerEvent::Dismiss(DismissalType::Temporary)) {
            set_custom_keybinding(SELECT_ALL_BINDING_NAME, &CTRL_SHIFT_A_KEYSTROKE, ctx);
            set_custom_keybinding(MOVE_LINE_START_BINDING_NAME, &CTRL_A_KEYSTROKE, ctx);
            set_custom_keybinding(MOVE_LINE_END_BINDING_NAME, &CTRL_E_KEYSTROKE, ctx);
        }
        EmacsBindingsSettings::handle(ctx).update(ctx, |settings_model, settings_ctx| {
            report_if_error!(settings_model
                .emacs_bindings_banner_state
                .set_value(BannerState::Dismissed, settings_ctx));
        });
        self.is_emacs_bindings_banner_open = false;
        ctx.notify();
    }

    fn should_show_emacs_bindings_banner(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        // Is this the active session?
        // We should only show the banner in one place at a time.
        if !self.is_active_session(ctx) {
            return false;
        }

        // Was the banner already open or dismissed?
        let emacs_bindings_banner_displayed = self.is_emacs_bindings_banner_open
            || EmacsBindingsSettings::handle(ctx).read(ctx, |banner_settings, _| {
                *banner_settings.emacs_bindings_banner_state.value() == BannerState::Dismissed
            });

        !emacs_bindings_banner_displayed
    }

    fn show_emacs_bindings_banner(&mut self, ctx: &mut ViewContext<Self>) {
        self.is_emacs_bindings_banner_open = true;
        ctx.notify();
    }

    /// Updates the state of the "incompatible shell configuration" banner with
    /// a new set of shell plugins. This should be called when either a new session
    /// is bootstrapped or the `honor_ps1` setting changes.
    fn update_incompatible_configuration_banner(
        &mut self,
        shell_plugins: &HashSet<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        let honor_ps1 = *SessionSettings::as_ref(ctx).honor_ps1;

        let show_banner = if honor_ps1 {
            let banner_content = if shell_plugins.contains("p10k_unsupported") {
                Some(BannerTextContent::formatted_text(vec![
                    FormattedTextFragment::bold("Powerlevel10k now supports Warper!  "),
                    FormattedTextFragment::plain_text(
                        "You seem to be running an older (unsupported) version, please follow ",
                    ),
                    FormattedTextFragment::hyperlink(
                        "these instructions",
                        P10K_UPDATE_INSTRUCTIONS_URL,
                    ),
                    FormattedTextFragment::plain_text(" to update to the latest version."),
                ]))
            } else if shell_plugins.contains("pure") {
                Some(BannerTextContent::formatted_text(vec![
                    FormattedTextFragment::plain_text(
                        "Pure is not yet supported in Warp. You might consider one of the \
                        supported prompts as an alternative.  ",
                    ),
                    FormattedTextFragment::hyperlink("Learn more", PROMPT_COMPATIBILITY_URL),
                ]))
            } else {
                None
            };

            if let Some(banner_content) = banner_content {
                self.incompatible_configuration_banner
                    .update(ctx, |banner, ctx| {
                        banner.set_content(banner_content, ctx);
                    });
                true
            } else {
                false
            }
        } else {
            false
        };

        if show_banner != self.is_incompatible_configuration_banner_open {
            self.is_incompatible_configuration_banner_open = show_banner;
            ctx.notify();
        }
    }

    fn handle_controlmaster_error_banner_event(
        &mut self,
        event: &BannerEvent<TerminalAction>,
        ctx: &mut ViewContext<Self>,
    ) {
        match event {
            BannerEvent::Dismiss { .. } => {
                self.control_master_error_banner_state.is_open = false;
                ctx.notify();
            }
            BannerEvent::Action(_) => {
                #[cfg(debug_assertions)]
                unimplemented!(
                    "Control master error banner does not yet support handling terminal actions"
                );
            }
        }
    }

    fn open_block_list_context_menu_via_keybinding(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(block_index) = self.selected_blocks.tail() {
            // We are manually putting the selected block in the hover state
            // before we open the context menu since
            // 1. The buttons need to be visible when the context menu is open
            // 2. We need to use the saved position of the overflow button
            // to know where to open up the context menu, which is only saved
            // using the position ID that includes the block index when a block
            // is hovered. Otherwise, we will have a panic.
            self.hovered_block_index = Some(block_index);
            self.scroll_to_if_not_visible(block_index, ctx);
            self.block_list_context_menu(
                &BlockListMenuSource::BlockKeybinding { block_index },
                ctx,
            );
            ctx.notify();
        }
    }

    fn terminal_up(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.selected_blocks.is_empty() {
            let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
            match input_mode {
                InputMode::PinnedToBottom | InputMode::Waterfall => {
                    self.select_less_recent_block(false /* is_shift_down */, ctx);
                }
                InputMode::PinnedToTop => {
                    self.select_more_recent_block(
                        false, /* is_cmd_down */
                        false, /* is_shift_down */
                        ctx,
                    );
                }
            }
        } else if self.is_long_running() {
            self.on_ssh_warpification_key_event(None, ctx);
            let sequence =
                EscCodes::build_escape_sequence(self.model.lock().deref(), &[EscCodes::ARROW_UP]);
            self.write_user_bytes_to_pty(sequence, ctx);
        }
    }

    fn bookmark_up(&mut self, ctx: &mut ViewContext<Self>) {
        let next_index = self
            .selected_blocks
            .tail()
            .and_then(|selected_block_index| {
                let mut maximum_index_above_bookmark = None;
                for index in self.bookmarked_blocks.keys() {
                    if *index < selected_block_index {
                        if let Some(max_ind) = maximum_index_above_bookmark {
                            if *index > max_ind {
                                maximum_index_above_bookmark = Some(*index);
                            }
                        } else {
                            maximum_index_above_bookmark = Some(*index);
                        }
                    }
                }
                maximum_index_above_bookmark
            })
            .or_else(|| self.bookmarked_blocks.keys().max().copied());

        if let Some(index) = next_index {
            self.reset_selection_to_single_block(index, ctx);
            self.jump_to_previous_command(index, ctx);
            ctx.notify();
        }
    }

    fn bookmark_down(&mut self, ctx: &mut ViewContext<Self>) {
        let next_index = self
            .selected_blocks
            .tail()
            .and_then(|selected_block_index| {
                let mut minimum_index_below_bookmark = None;
                for index in self.bookmarked_blocks.keys() {
                    if *index > selected_block_index {
                        if let Some(min_ind) = minimum_index_below_bookmark {
                            if *index < min_ind {
                                minimum_index_below_bookmark = Some(*index);
                            }
                        } else {
                            minimum_index_below_bookmark = Some(*index);
                        }
                    }
                }
                minimum_index_below_bookmark
            })
            .or_else(|| self.bookmarked_blocks.keys().min().copied());

        if let Some(index) = next_index {
            self.reset_selection_to_single_block(index, ctx);
            self.jump_to_previous_command(index, ctx);
            ctx.notify();
        }
    }

    fn terminal_down(&mut self, ctx: &mut ViewContext<Self>) {
        if !self.selected_blocks.is_empty() {
            let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
            match input_mode {
                InputMode::PinnedToBottom | InputMode::Waterfall => {
                    self.select_more_recent_block(
                        false, /* is_cmd_down */
                        false, /* is_shift_down */
                        ctx,
                    );
                }
                InputMode::PinnedToTop => {
                    self.select_less_recent_block(false /* is_cmd_down */, ctx);
                }
            }
        } else if self.is_long_running() {
            let sequence =
                EscCodes::build_escape_sequence(self.model.lock().deref(), &[EscCodes::ARROW_DOWN]);
            self.write_user_bytes_to_pty(sequence, ctx);
        }
    }

    fn page_up(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            // Note: We explicitly use the CSI prefix, as the terminal we are impersonating
            // (`xterm-256color`) has the escape sequence for page up defined with that prefix
            let sequence = EscCodes::build_escape_sequence_with_c1(C1::CSI, EscCodes::PAGE_UP);
            self.write_user_bytes_to_pty(sequence, ctx);
        } else {
            self.update_scroll_position_locking(ScrollPositionUpdate::AfterPageUp, ctx);
        }
    }

    fn page_down(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            // Note: We explicitly use the CSI prefix, as the terminal we are impersonating
            // (`xterm-256color`) has the escape sequence for page down defined with that prefix
            let sequence = EscCodes::build_escape_sequence_with_c1(C1::CSI, EscCodes::PAGE_DOWN);
            self.write_user_bytes_to_pty(sequence, ctx);
        } else {
            self.update_scroll_position_locking(ScrollPositionUpdate::AfterPageDown, ctx);
        }
    }

    fn move_home(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            let sequence = EscCodes::build_escape_sequence(self.model.lock().deref(), b"H");
            self.write_user_bytes_to_pty(sequence, ctx);
        } else {
            self.update_scroll_position_locking(ScrollPositionUpdate::AfterHome, ctx);
        }
    }

    fn move_end(&mut self, ctx: &mut ViewContext<Self>) {
        if self.is_long_running() {
            let sequence = EscCodes::build_escape_sequence(self.model.lock().deref(), b"F");
            self.write_user_bytes_to_pty(sequence, ctx);
        } else {
            self.update_scroll_position_locking(ScrollPositionUpdate::AfterEnd, ctx);
        }
    }

    fn keyboard_select_text(
        &mut self,
        ctx: &mut ViewContext<Self>,
        direction: &SelectionDirection,
    ) {
        let semantic_selection = SemanticSelection::as_ref(ctx);
        let selection_result = self.model.lock().block_list_mut().move_selection_tail(
            direction,
            semantic_selection,
            self.is_inverted_blocklist(ctx),
        );

        if let Some(new_tail) = selection_result {
            // Because standardized endpoints fall in the vertical center of their row,
            // subtracting 0.5 positions us at the top of the row, where we'd like to scroll to.
            let row = new_tail.row - 0.5.into_lines();
            self.scroll_to_row_if_not_visible(row.into_lines(), ctx);
        }

        self.maybe_copy_selection_to_clipboard(ctx);

        // The text selection changed, so clear any previously attached context text.

        ctx.notify();
    }

    /// Takes a row in the blocklist coordinate space.
    fn scroll_to_row_if_not_visible(&mut self, row: Lines, ctx: &mut ViewContext<Self>) {
        self.update_scroll_position_locking(
            ScrollPositionUpdate::ScrollToBlocklistRowIfNotVisible { row },
            ctx,
        );
    }

    fn scroll_to_if_not_visible(&mut self, block_index: BlockIndex, ctx: &mut ViewContext<Self>) {
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();
        if !self.is_block_visible_locking(
            block_index,
            BlockVisibilityMode::TopOfBlockVisible,
            input_mode,
            ctx,
        ) {
            self.scroll_to(block_index, ctx);
        }
    }

    fn jump_to_previous_command(
        &mut self,
        topmost_block_index: BlockIndex,
        ctx: &mut ViewContext<Self>,
    ) {
        self.scroll_to_if_not_visible(topmost_block_index, ctx);
    }

    fn jump_to_bookmark(&mut self, index: BlockIndex, ctx: &mut ViewContext<Self>) {
        self.reset_selection_to_single_block(index, ctx);
        self.jump_to_previous_command(index, ctx);

        ctx.notify();
    }

    /// Scrolls to the focused match
    fn scroll_to_match(&mut self, ctx: &mut ViewContext<Self>) {
        // Scrolling to matches is not done for the alt screen.
        if self.model.lock().is_alt_screen_active() {
            return;
        }

        let Some(focused_match) = self
            .find_model
            .as_ref(ctx)
            .block_list_find_run()
            .and_then(|run| run.focused_match())
        else {
            return;
        };

        let find_match_location = match focused_match {
            BlockListMatch::RichContent { index, .. } => {
                FindMatchScrollLocation::RichContent { index: *index }
            }
            BlockListMatch::CommandBlock(BlockGridMatch {
                block_index,
                range,
                grid_type,
                ..
            }) => {
                let focused_match_row = range.start().row;

                let block_section = match grid_type {
                    GridType::PromptAndCommand => {
                        BlockSection::PromptAndCommandGrid(focused_match_row.into_lines())
                    }
                    GridType::Output => BlockSection::OutputGrid(focused_match_row.into_lines()),
                    _ => {
                        // Find matches never occur in other grid types.
                        return;
                    }
                };
                FindMatchScrollLocation::Block {
                    block_index: *block_index,
                    section: block_section,
                }
            }
        };

        self.update_scroll_position_locking(
            ScrollPositionUpdate::ScrollToFindMatchIfNotVisible(find_match_location),
            ctx,
        );
    }

    /// Scrolls the view to the top of the block at `block_index`.
    fn scroll_to(&mut self, block_index: BlockIndex, ctx: &mut ViewContext<Self>) {
        self.update_scroll_position_locking(
            ScrollPositionUpdate::ScrollToTopOfBlock { block_index },
            ctx,
        );
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn selected_blocks_tail_index(&self) -> Option<BlockIndex> {
        self.selected_blocks.tail()
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn selected_blocks_pivot_index(&self) -> Option<BlockIndex> {
        self.selected_blocks
            .ranges()
            .last()
            .map(|range| range.pivot())
    }

    /// Returns the CLI agent currently active in this terminal, if any.
    pub fn active_cli_agent(&self, ctx: &AppContext) -> Option<super::CLIAgent> {
        if !FeatureFlag::HoaCodeReview.is_enabled() {
            return None;
        }

        CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .map(|s| s.agent)
    }

    /// Returns `true` if CLI agent rich input is currently open.
    pub fn is_cli_agent_rich_input_open(&self, ctx: &AppContext) -> bool {
        CLIAgentSessionsModel::as_ref(ctx).is_input_open(self.view_id)
    }

    /// Appends `text` to CLI agent rich input and focuses it.
    fn append_to_rich_input(&mut self, text: &str, ctx: &mut ViewContext<Self>) {
        self.input.update(ctx, |input, ctx| {
            input.append_to_buffer(text, ctx);
        });
        self.focus_input_box(ctx);
    }

    /// Sends `text` to the active CLI agent, routing to rich input when it is open
    /// or directly to the PTY when it is closed.
    ///
    /// Returns `Some(CliAgentRouting)` indicating how the text was sent, or
    /// `None` if no CLI agent is active.
    pub fn try_send_text_to_cli_agent_or_rich_input(
        &mut self,
        text: String,
        ctx: &mut ViewContext<Self>,
    ) -> Option<CliAgentRouting> {
        self.active_cli_agent(ctx)?;
        if self.is_cli_agent_rich_input_open(ctx) {
            self.append_to_rich_input(&text, ctx);
            Some(CliAgentRouting::RichInput)
        } else {
            self.write_to_pty(text.into_bytes(), ctx);
            self.focus_terminal(ctx);
            Some(CliAgentRouting::Pty)
        }
    }

    /// Sends code review comments to a running CLI agent, routing to the
    /// rich input when it is open or directly to the PTY when closed.
    pub fn send_review_to_cli_agent_or_rich_input(
        &mut self,
        review: &ReviewCommentPacket,
        ctx: &mut ViewContext<Self>,
    ) -> anyhow::Result<()> {
        let text = cli_agent::build_review_prompt(review);
        self.try_send_text_to_cli_agent_or_rich_input(text, ctx)
            .ok_or_else(|| {
                anyhow::anyhow!("Start a third-party CLI agent before sending review comments.")
            })?;
        Ok(())
    }

    /// Sends diff file context hunks to a running CLI agent, routing to the
    /// rich input when open or the PTY when closed.
    #[cfg(feature = "local_fs")]
    pub fn send_diff_context_to_cli_agent_or_rich_input(
        &mut self,
        file_diffs: &std::collections::HashMap<
            String,
            Vec<crate::code_review::context::DiffSetHunk>,
        >,
        ctx: &mut ViewContext<Self>,
    ) -> Option<CliAgentRouting> {
        let text = cli_agent::build_diff_context_prompt(file_diffs);
        self.try_send_text_to_cli_agent_or_rich_input(text, ctx)
    }

    /// Sends a diff hunk location to a running CLI agent, routing to the
    /// rich input when open or the PTY when closed.
    pub fn send_diff_hunk_to_cli_agent_or_rich_input(
        &mut self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
        lines_added: u32,
        lines_removed: u32,
        ctx: &mut ViewContext<Self>,
    ) -> Option<CliAgentRouting> {
        let text = cli_agent::build_diff_hunk_prompt(
            file_path,
            start_line,
            end_line,
            lines_added,
            lines_removed,
        );
        self.try_send_text_to_cli_agent_or_rich_input(text, ctx)
    }

    fn handle_theme_change(&mut self, ctx: &mut ViewContext<Self>) {
        let appearance = Appearance::as_ref(ctx);
        let colors = color::List::from(&appearance.theme().clone().into());
        let mut model = self.model.lock();
        model.update_colors(colors);
        self.colors = colors;
        ctx.notify();
    }

    fn handle_reporting_settings_event(
        &mut self,
        _evt: &AltScreenReportingChangedEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.notify();
    }

    fn handle_session_settings_event(
        &mut self,
        evt: &SessionSettingsChangedEvent,
        ctx: &mut ViewContext<Self>,
    ) {
        match evt {
            SessionSettingsChangedEvent::HonorPS1 { .. } => {
                let session = self
                    .active_block_session_id()
                    .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id));

                if let Some(session) = session {
                    self.update_incompatible_configuration_banner(session.shell().plugins(), ctx)
                }

                // honor_ps1 affects whether the Warp prompt is active, which
                // determines if we need git status updates.
                self.update_git_status_subscription(ctx);
            }
            SessionSettingsChangedEvent::CLIAgentToolbarChipSelectionSetting { .. } => {
                // Force-close rich input when the Rich Input chip is removed so
                // it doesn't linger open with no toolbar button to manage it.
                if !is_rich_input_chip_in_cli_toolbar(ctx) {
                    self.close_cli_agent_rich_input(ctx);
                }
            }
            _ => {}
        }
    }

    fn block_prompt(model: &TerminalModel, sessions: &Sessions, block_index: BlockIndex) -> String {
        let block = match model.block_list().block_at(block_index) {
            None => return String::new(),
            Some(block) => block,
        };

        let prompt = if block.honor_ps1() {
            block.prompt_contents_to_string(false)
        } else if block.prompt_snapshot().is_some() {
            // Note that we're checking not only for the flag being enabled but also ensuring the
            // prompt_snapshot is defined. This is because some historical blocks from the restored
            // session may not have yet their prompt_snapshot value, and we stil want to show them
            // nicely.
            block
                .prompt_snapshot()
                .map(|prompt| prompt.to_string())
                .unwrap_or_default()
        } else {
            let session = block
                .session_id()
                .and_then(|session_id| sessions.get(session_id));
            let user_and_host_name_string = session.as_ref().and_then(|session| {
                prompt::user_and_host_name_string(
                    session.session_type().clone(),
                    session.hostname(),
                    session.user(),
                )
            });
            let home_dir = session
                .and_then(|session| session.home_dir().map(|directory| directory.to_owned()));

            format!(
                "{}{}{}{}{}",
                block
                    .conda_env()
                    .map_or_else(String::new, |b| format!("({b}) ")),
                block
                    .virtual_env_short_name()
                    .map_or_else(String::new, |b| format!("({b}) ")),
                user_and_host_name_string.unwrap_or_default(),
                prompt::display_path_string(block.pwd(), home_dir.as_deref()),
                block
                    .git_branch()
                    .map_or_else(String::new, |b| format!(" git:({b})")),
            )
        };

        // Skip the honor_ps1 case since there's no good place to display the extra text.

        prompt
    }

    /// Returns the duration as an std::time::Duration struct
    fn block_duration(&self, serialized_block: &SerializedBlock) -> Option<Duration> {
        (serialized_block.completed_ts? - serialized_block.start_ts?)
            .to_std()
            .ok()
    }

    fn block_duration_text(model: &TerminalModel, block_index: BlockIndex) -> Option<String> {
        model
            .block_list()
            .block_at(block_index)?
            .formatted_duration_string()
    }

    fn block_start_and_completed_ts(model: &TerminalModel, block_index: BlockIndex) -> String {
        let block = match model.block_list().block_at(block_index) {
            None => return String::new(),
            Some(block) => block,
        };

        let start = block.start_ts().map_or_else(String::new, |b| {
            format!("Started at: {}", b.format("%a %b %-d at %-I:%M %p"))
        });
        let end = block.completed_ts().map_or_else(String::new, |b| {
            format!("\nCompleted at: {}", b.format("%a %b %-d at %-I:%M %p"))
        });
        format!("{start}{end}")
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn is_find_bar_open(&self, app: &AppContext) -> bool {
        self.find_model.as_ref(app).is_find_bar_open()
    }

    pub fn is_find_bar_focused(&self, ctx: &AppContext) -> bool {
        self.find_bar.as_ref(ctx).is_editor_focused(ctx)
    }

    pub fn pwd(&self) -> Option<String> {
        self.active_block_metadata
            .as_ref()
            .and_then(BlockMetadata::current_working_directory)
            .map(|pwd| pwd.to_string())
    }

    pub fn pwd_if_local(&self, ctx: &AppContext) -> Option<String> {
        self.active_session_path_if_local(ctx)
            .map(|path| path.to_string_lossy().into_owned())
    }

    pub fn shell_launch_data_if_local(&self, ctx: &AppContext) -> Option<ShellLaunchData> {
        if !FeatureFlag::ShellSelector.is_enabled() {
            return None;
        }

        let session_id = self.active_block_session_id()?;
        let Some(session) = self.sessions.as_ref(ctx).get(session_id) else {
            log::warn!("Expected to have session for session ID {session_id:?}, but doesn't exist");
            return None;
        };
        if !session.is_local() {
            return None;
        }

        session.launch_data().cloned()
    }

    fn spawning_command_for_subshell_sessions(
        &self,
        app: &AppContext,
    ) -> HashMap<SessionId, SubshellSource> {
        self.sessions
            .as_ref(app)
            .spawning_command_for_subshell_sessions()
    }

    fn is_waterfall_gap_mode(&self, model: &TerminalModel, app: &AppContext) -> bool {
        let input_mode = *InputModeSettings::as_ref(app).input_mode.value();
        self.viewport_state(model.block_list(), input_mode, app)
            .is_waterfall_gap_mode()
    }

    pub fn get_terminal_view_render_context(
        &self,
        model: &TerminalModel,
        app: &AppContext,
    ) -> TerminalViewRenderContext {
        let (pane_state, active_session_state) = match self.focus_handle.as_ref() {
            Some(handle) => (
                handle.split_pane_state(app),
                if handle.is_active_session(app) {
                    ActiveSessionState::Active
                } else {
                    ActiveSessionState::Inactive
                },
            ),
            None => (SplitPaneState::NotInSplitPane, ActiveSessionState::Active),
        };

        TerminalViewRenderContext {
            size_info: *self.size_info(),
            scroll_position: self.scroll_position(),
            highlighted_url: self.highlighted_link.clone_inner(),
            link_tool_tip: self.open_grid_link_tool_tip.clone(),
            is_terminal_focused: self
                .view_handle
                .upgrade(app)
                .expect("terminal should upgrade")
                .is_focused(app),
            is_terminal_selecting: self.is_selecting(),
            is_context_menu_open: self.is_context_menu_open(),
            is_waterfall_gap_mode: self.is_waterfall_gap_mode(model, app),
            pane_state,
            active_session_state,
            selected_blocks: self.selected_blocks.clone(),
            input_box_element_key: self.input.as_ref(app).save_position_id(),
            terminal_view_id: self.view_id,
            spawning_command_for_subshell_sessions: self
                .spawning_command_for_subshell_sessions(app),
            obfuscate_secrets: get_secret_obfuscation_mode(app),
            hovered_secret: self.hovered_secret,
            horizontal_clipped_scroll_state: self.horizontal_clipped_scroll_state.clone(),
        }
    }

    fn render_filter_element(
        block_index: BlockIndex,
        active_filter_editor_block_index: Option<BlockIndex>,
        filter_mouse_state: MouseStateHandle,
        has_active_filter: bool,
        tool_tip_below_button: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let icon = Container::new(
            ConstrainedBox::new(if has_active_filter {
                icons::Icon::FilterFunnelFilled
                    .to_warpui_icon(appearance.theme().accent())
                    .finish()
            } else {
                icons::Icon::FilterFunnel
                    .to_warpui_icon(
                        appearance
                            .theme()
                            .sub_text_color(appearance.theme().surface_2()),
                    )
                    .finish()
            })
            .with_height(26.)
            .with_width(26.)
            .finish(),
        );

        let should_disable_filter_button =
            active_filter_editor_block_index.is_some_and(|active_filter_editor_block_index| {
                block_index == active_filter_editor_block_index
            });

        SavePosition::new(
            render_hoverable_block_button(
                icon,
                Some(ToolbeltButtonTooltip {
                    label: "Filter block output".to_string(),
                    tool_tip_below_button,
                }),
                should_disable_filter_button,
                true,
                filter_mouse_state,
                appearance.theme(),
                appearance.ui_builder(),
                move |ctx, _, _| {
                    ctx.dispatch_typed_action(TerminalAction::OpenBlockFilterEditor(block_index))
                },
            ),
            filter_button_position_id(block_index).as_str(),
        )
        .finish()
    }

    fn render_bookmark_element(
        index: BlockIndex,
        bookmark_mouse_state: MouseStateHandle,
        is_bookmarked: bool,
        tool_tip_below_button: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        let bookmark_fill_color: ColorU = theme.accent().into();
        let icon_color = if is_bookmarked {
            bookmark_fill_color
        } else {
            theme.sub_text_color(theme.surface_2()).into()
        };

        let icon_path = if is_bookmarked {
            "bundled/svg/bookmark_filled.svg"
        } else {
            "bundled/svg/bookmark.svg"
        };

        let icon = Container::new(
            ConstrainedBox::new(Icon::new(icon_path, icon_color).finish())
                .with_height(26.)
                .with_width(26.)
                .finish(),
        );

        render_hoverable_block_button(
            icon,
            Some(ToolbeltButtonTooltip {
                label: "Bookmark this block to quickly scroll to it".to_string(),
                tool_tip_below_button,
            }),
            false,
            true,
            bookmark_mouse_state,
            theme,
            appearance.ui_builder(),
            move |ctx, _, _| {
                ctx.dispatch_typed_action(TerminalAction::BookmarkBlock(index));
            },
        )
    }

    fn is_jump_to_bottom_of_block_element_hovered(&self) -> bool {
        self.mouse_states
            .jump_to_bottom_of_block_button
            .lock()
            .is_ok_and(|handle| handle.is_hovered())
    }

    fn render_jump_to_bottom_of_block_element(
        &self,
        overhanging_block: OverhangingBlock,
        is_long_running_command: bool,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let theme = appearance.theme();
        Hoverable::new(
            self.mouse_states.jump_to_bottom_of_block_button.clone(),
            move |state| {
                let icon_color: ColorU = theme.sub_text_color(theme.surface_2()).into();
                let icon_path = "bundled/svg/vertical_align_bottom.svg";

                let container = Container::new(
                    ConstrainedBox::new(Icon::new(icon_path, icon_color).finish())
                        .with_height(JUMP_TO_BOTTOM_OF_BLOCK_ICON_SIZE_PX.as_f32())
                        .with_width(JUMP_TO_BOTTOM_OF_BLOCK_ICON_SIZE_PX.as_f32())
                        .finish(),
                )
                .with_uniform_padding(JUMP_TO_BOTTOM_OF_BLOCK_BUTTON_PADDING_PX.as_f32())
                .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
                    JUMP_TO_BOTTOM_OF_BLOCK_CORNER_RADIUS_PX.as_f32(),
                )));

                let container = if state.is_hovered() || state.is_clicked() {
                    container.with_background(theme.surface_2())
                } else {
                    container
                };

                let mut stack = Stack::new().with_child(container.finish());

                if state.is_hovered() {
                    let input_mode = *InputModeSettings::as_ref(app).input_mode.value();
                    let tool_tip_text = if overhanging_block.is_most_recent_block()
                        && input_mode.is_inverted_blocklist()
                        && is_long_running_command
                    {
                        "Lock scrolling at bottom of block".to_string()
                    } else {
                        "Jump to the bottom of this block".to_string()
                    };

                    let tool_tip = appearance
                        .ui_builder()
                        .tool_tip(tool_tip_text)
                        .build()
                        .finish();

                    stack.add_positioned_child(
                        tool_tip,
                        OffsetPositioning::offset_from_parent(
                            vec2f(0., JUMP_TO_BOTTOM_OF_BLOCK_TOOLTIP_OFFSET_Y_PX.as_f32()),
                            ParentOffsetBounds::Unbounded,
                            ParentAnchor::TopRight,
                            ChildAnchor::BottomRight,
                        ),
                    );
                }

                stack.finish()
            },
        )
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(TerminalAction::ScrollToBottomOfOverhangingBlock(
                overhanging_block,
            ));
        })
        .on_hover(move |mouse_in, ctx, _, position| {
            // Since this element is on top of the block list element, we need to start a block hover here
            // rather than relying on the block list element itself to manage hover state.
            if mouse_in {
                ctx.dispatch_typed_action(TerminalAction::BlockHover(BlockHoverAction::Begin {
                    position,
                    block_index: overhanging_block.block_index(),
                }));
            } else {
                ctx.dispatch_typed_action(TerminalAction::BlockHover(BlockHoverAction::Clear));
            }
        })
        .finish()
    }

    fn render_label_element(
        index: BlockIndex,
        model: &TerminalModel,
        mouse_state: Option<&MouseStateHandle>,
        sessions: &Sessions,
        padding_x: Pixels,
        tool_tip_below_button: bool,
        appearance: &Appearance,
    ) -> Box<dyn Element> {
        let terminal_theme_prompt: ColorU = appearance
            .theme()
            .sub_text_color(appearance.theme().background())
            .into();

        let prompt = Text::new_inline(
            Self::block_prompt(model, sessions, index),
            appearance.monospace_font_family(),
            appearance.monospace_font_size() * WARP_PROMPT_HEIGHT_LINES,
        )
        .with_style(Properties::default().weight(appearance.monospace_font_weight()))
        .with_color(terminal_theme_prompt)
        .finish();

        let mut label_row = Flex::row().with_child(prompt);

        if let Some(duration_string) = Self::block_duration_text(model, index) {
            let duration = Text::new_inline(
                duration_string,
                appearance.monospace_font_family(),
                appearance.monospace_font_size() * WARP_PROMPT_HEIGHT_LINES,
            )
            .with_style(Properties::default().weight(appearance.monospace_font_weight()))
            .with_color(terminal_theme_prompt)
            .finish();

            label_row.add_child(if let Some(state) = mouse_state {
                Hoverable::new(state.clone(), |state| {
                    let mut stack = Stack::new().with_child(duration);
                    if state.is_hovered() {
                        let tool_tip = appearance
                            .ui_builder()
                            .tool_tip(Self::block_start_and_completed_ts(model, index))
                            .build()
                            .finish();
                        if tool_tip_below_button {
                            stack.add_positioned_child(
                                tool_tip,
                                OffsetPositioning::offset_from_parent(
                                    Vector2F::new(30., 5.),
                                    ParentOffsetBounds::Unbounded,
                                    ParentAnchor::BottomMiddle,
                                    ChildAnchor::TopMiddle,
                                ),
                            );
                        } else {
                            stack.add_positioned_child(
                                tool_tip,
                                OffsetPositioning::offset_from_parent(
                                    Vector2F::new(30., -5.),
                                    ParentOffsetBounds::Unbounded,
                                    ParentAnchor::TopMiddle,
                                    ChildAnchor::BottomMiddle,
                                ),
                            );
                        }
                    }
                    stack.finish()
                })
                .with_hover_in_delay(Duration::from_millis(500))
                .finish()
            } else {
                duration
            });
        }

        SavePosition::new(
            Container::new(label_row.finish())
                .with_padding_left(padding_x.as_f32())
                .with_padding_right(padding_x.as_f32())
                .with_padding_bottom(16.)
                .finish(),
            format!("block_index:{index}").as_str(),
        )
        .finish()
    }

    fn render_input(&self) -> Box<dyn Element> {
        let input = ChildView::new(&self.input).finish();
        Hoverable::new(self.input_hoverable_handle.clone(), |_| input).finish()
    }

    fn render_inline_banners(
        &self,
        appearance: &Appearance,
        app: &AppContext,
    ) -> HashMap<usize, Box<dyn Element>> {
        let mut inline_banners = HashMap::new();

        // If the notifications discovery banner is open, render it.
        if let NotificationsDiscoveryBanner::Open {
            trigger,
            request_outcome,
            state,
        } = &self.inline_banners_state.notifications_discovery_banner
        {
            inline_banners.insert(
                state.banner_id,
                render_inline_notifications_discovery_banner(
                    *trigger,
                    request_outcome.clone(),
                    state,
                    SessionSettings::as_ref(app).notifications.mode,
                    appearance,
                ),
            );
        }

        // If the notifications error banner is open, render it.
        if let NotificationsErrorBannerType::Open { state } = &self
            .inline_banners_state
            .notifications_error_banner
            .banner_type
        {
            let banner_title = self
                .inline_banners_state
                .notifications_error_banner
                .error
                .as_ref()
                .map(|e| e.notifications_error_banner_title())
                .unwrap_or("Error sending notification");

            inline_banners.insert(
                state.banner_id,
                render_inline_notifications_error_banner(
                    banner_title,
                    state,
                    &self.inline_banners_state.notifications_error_banner.error,
                    appearance,
                ),
            );
        }

        for (banner_id, state) in &self.inline_banners_state.ssh_banners {
            inline_banners.insert(
                *banner_id,
                render_inline_ssh_wrapper_banner(state, appearance),
            );
        }

        if let AliasExpansionBanner::Open { state } =
            &self.inline_banners_state.alias_expansion_banner
        {
            inline_banners.insert(state.id, render_alias_expansion_banner(state, appearance));
        }

        if let Some(ShellProcessTerminatedBanner {
            banner_id,
            was_premature_termination,
        }) = self.inline_banners_state.shell_process_terminated_banner
        {
            inline_banners.insert(
                banner_id,
                render_shell_process_terminated_banner(appearance, was_premature_termination),
            );
        }

        if let Some(open_in_warp_banner) = &self.inline_banners_state.open_in_warp_banner {
            inline_banners.insert(
                open_in_warp_banner.id,
                render_open_in_warp_banner(open_in_warp_banner, self.view_id, appearance),
            );
        }

        if let Some(vim_banner_state) = &self.inline_banners_state.vim_banner_state {
            inline_banners.insert(
                vim_banner_state.id,
                render_vim_mode_banner(vim_banner_state, appearance),
            );
        }

        inline_banners
    }

    #[cfg(feature = "integration_tests")]
    pub fn content_element_position_id(&self) -> &String {
        &self.content_element_position_id
    }

    fn render_alt_screen_element(
        &self,
        app: &AppContext,
        model: &TerminalModel,
        selection_range: Option<ExpandedSelectionRange<Point>>,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);

        let columns = self.size_info.columns();

        // Note: The Alt screen relies on the accuracy of the `padding` elements of SizeInfo
        // for things like hit detection and selection. Since we are taking into account the
        // padding separately (using `Align` and `ConstrainedBox`), we need to create a new
        // SizeInfo that reflects the lack of padding on the AltScreenElement directly
        let render_context = self.get_terminal_view_render_context(model, app);

        let enforce_minimum_contrast = *FontSettings::as_ref(app).enforce_minimum_contrast;
        let mut alt_screen_element = AltScreenElement::new(
            self.model.clone(),
            render_context,
            self.find_model.clone(),
            enforce_minimum_contrast,
            selection_range.map(|selection| match selection {
                ExpandedSelectionRange::Rect { rows } => rows.mapped(|(start, end)| start..end),
                ExpandedSelectionRange::Regular { start, end, .. } => vec1![start..end],
            }),
            appearance,
            self.alt_screen_scroll_top,
            // TODO(zachbai): Remove this.
            None,
        );
        if should_use_ligature_rendering(app) {
            alt_screen_element = alt_screen_element.with_ligature_rendering();
        }
        if self.should_hide_cli_agent_cursor_cell(app) {
            alt_screen_element = alt_screen_element.with_hide_cursor_cell();
        }
        let required_terminal_width = self.size_info.cell_width_px.as_f32() * (columns as f32)
            + 2. * self.size_info.padding_x_px().as_f32();

        let should_be_vertical_scrollable = false;

        let should_be_horizontal_scrollable = false;

        let theme = appearance.theme();
        let element = maybe_wrap_terminal_element_in_scrollable(
            should_be_vertical_scrollable,
            should_be_horizontal_scrollable,
            self.alt_screen_vertical_scroll_state.clone(),
            self.horizontal_clipped_scroll_state.clone(),
            required_terminal_width,
            theme,
            alt_screen_element,
        );

        SavePosition::new(
            Container::new(
                Align::new(
                    ConstrainedBox::new(
                        // We wrap in a `Clipped` to prevent grid text from partially bleeding into the pane header.
                        // This is different from a ClippedScrollable because the alt screen is not actually rendering
                        // unnecessary rows.
                        Clipped::new(element).finish(),
                    )
                    .finish(),
                )
                // Pin the alt-screen origin to the top-left of the pane (adjusted for padding)
                // to prevent a wiggle-like effect when resizing the pane.
                .top_left()
                .finish(),
            )
            .with_vertical_padding(self.size_info.padding_y_px().as_f32())
            .finish(),
            &self.content_element_position_id,
        )
        .finish()
    }

    /// Returns true when cursor rendering should be suppressed because the
    /// CLI agent rich input is open.
    fn should_hide_cli_agent_cursor_cell(&self, app: &AppContext) -> bool {
        CLIAgentSessionsModel::as_ref(app)
            .session(self.view_id)
            .is_some_and(|s| matches!(s.input_state, CLIAgentInputState::Open { .. }))
    }

    fn render_block_list_element(
        &self,
        model: &TerminalModel,
        input_mode: InputMode,
        is_scrollable: bool,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let theme = appearance.theme();
        let padding_x = self.size_info.padding_x_px;
        let sessions = self.sessions.clone();

        let inline_banners = self.render_inline_banners(appearance, app);

        let mut subshell_separators = HashMap::new();

        for (id, command) in self.warpify_state.get_subshell_separators() {
            subshell_separators.insert(*id, render_subshell_separator(command.clone(), appearance));
        }

        // Currently, it is assumed that only the active block can have a block banner, which
        // implies that there can only be one at a time. This assumption can be relaxed once we
        // have an actual use case for that.
        let block_banner = model
            .block_list()
            .active_block()
            .block_banner()
            .map(|banner| match banner {
                WithinBlockBanner::WarpifyBanner(state) => {
                    render_warpification_banner(state, appearance, app)
                }
            });

        let bookmarked_blocks: HashSet<_> = self.bookmarked_blocks.keys().copied().collect();
        let filtered_blocks: HashSet<_> = model.block_list().filtered_blocks();

        let snackbar_header_state = SnackbarHeaderState {
            snackbar_enabled: *BlockListSettings::as_ref(app).snackbar_enabled,
            show_snackbar: self.show_snackbar,
            hover_near_snackbar_area: self.hover_near_snackbar_area,
            state_handle: self.snackbar_header_state.state_handle.clone(),
        };

        let semantic_selection = SemanticSelection::as_ref(app);
        let selection_range = model
            .block_list()
            .renderable_selection(semantic_selection, input_mode.is_inverted_blocklist());

        let terminal_spacing =
            TerminalSettings::as_ref(app).terminal_spacing(appearance.line_height_ratio(), app);

        let enforce_minimum_contrast = *FontSettings::as_ref(app).enforce_minimum_contrast;

        let mut element = BlockListElement::new(
            self.model.clone(),
            self.find_model.clone(),
            input_mode,
            self.get_terminal_view_render_context(model, app),
            self.block_list_mouse_states.clone(),
            snackbar_header_state,
            &terminal_spacing,
            enforce_minimum_contrast,
            appearance,
            Box::new(move |range, label_mouse_states, model, app| {
                range
                    .iter()
                    .enumerate()
                    .map(|(i, index)| {
                        let mut label = Self::render_label_element(
                            *index,
                            model,
                            label_mouse_states.get(index),
                            sessions.as_ref(app),
                            padding_x,
                            i == 0,
                            Appearance::as_ref(app),
                        );
                        // Special-case the last block so there is a reliable way to target it
                        // regardless of the length of the list.
                        if i == range.len() - 1 {
                            label = SavePosition::new(label, "block_index:last").finish()
                        }
                        label
                    })
                    .collect()
            }),
            Box::new(move |range, hovered_index, mouse_states, app| {
                range
                    .iter()
                    .enumerate()
                    .map(|(i, block_index)| {
                        let mouse_state = mouse_states.get(block_index)?.clone();
                        let is_bookmarked = bookmarked_blocks.contains(block_index);

                        if is_bookmarked || hovered_index == Some(*block_index) {
                            Some(Self::render_bookmark_element(
                                *block_index,
                                mouse_state,
                                is_bookmarked,
                                i == 0,
                                Appearance::as_ref(app),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect()
            }),
            Box::new(
                move |range,
                      hovered_index,
                      active_filter_editor_block_index,
                      filtered_blocks,
                      mouse_states,
                      app| {
                    range
                        .iter()
                        .enumerate()
                        .map(|(i, block_index)| {
                            let mouse_state = mouse_states.get(block_index)?.clone();
                            let has_active_filter =
                                filtered_blocks.is_some_and(|filtered_blocks| {
                                    filtered_blocks.contains(block_index)
                                });
                            if has_active_filter
                                || hovered_index == Some(*block_index)
                                || active_filter_editor_block_index == Some(*block_index)
                            {
                                Some(Self::render_filter_element(
                                    *block_index,
                                    active_filter_editor_block_index,
                                    mouse_state,
                                    has_active_filter,
                                    i == 0,
                                    Appearance::as_ref(app),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect()
                },
            ),
            inline_banners,
            subshell_separators,
            selection_range,
            block_banner,
            self.input_size_at_last_frame(app).unwrap_or_default(),
            None,
        );

        if should_use_ligature_rendering(app) {
            element = element.with_ligature_rendering();
        }

        if self.should_hide_cli_agent_cursor_cell(app) {
            element = element.with_hide_cursor_cell();
        }

        element = element.with_filtered_blocks(filtered_blocks);

        if let Some(active_filter_editor_block_index) = self.active_filter_editor_block_index {
            element = element.with_active_block_filter_editor(active_filter_editor_block_index);
        }

        if !self.rich_content_views.is_empty() {
            element = element.with_rich_content(
                self.rich_content_views
                    .iter()
                    .map(RichContent::to_block_list_element_render_params),
            );
        }

        if let Some(hovered_block_index) = self.hovered_block_index {
            let block_list = model.block_list();

            // Is this block the first visible item in the viewport? If so, the tool tips should
            // render below their respective buttons or else they'll get cut off by the edge of the
            // element.
            let should_render_tooltip_below_button = self
                .viewport_state(block_list, input_mode, app)
                .iter()
                .next()
                .and_then(|item| item.block_index)
                == Some(hovered_block_index);

            element = element.with_hovered_index(
                hovered_block_index,
                model,
                should_render_tooltip_below_button,
                app,
            );
        }

        let total_height: Lines = model.block_list().block_heights().summary().height;
        let visible_rows = self.content_element_height_lines(app);

        // Since blocks in a blocklist can have different sizes, we want
        // to make sure we're rendering with enough columns to support them all.
        let columns_needed = model
            .block_list()
            .blocks()
            .iter()
            .filter(|b| b.is_visible())
            .map(|b| b.size().columns)
            .max()
            .unwrap_or(self.size_info.columns);

        let required_terminal_width = self.size_info.cell_width_px.as_f32()
            * (columns_needed as f32)
            + 2. * self.size_info.padding_x_px().as_f32();
        let should_be_vertical_scrollable =
            heights_approx_gt(total_height, visible_rows) && is_scrollable;

        let should_be_horizontal_scrollable = false;

        let block_list = maybe_wrap_terminal_element_in_scrollable(
            should_be_vertical_scrollable,
            should_be_horizontal_scrollable,
            self.blocklist_vertical_scroll_state.clone(),
            self.horizontal_clipped_scroll_state.clone(),
            required_terminal_width,
            theme,
            element,
        );

        let block_list = DropTarget::new(
            block_list,
            TerminalDropTargetData {
                terminal_view: self.view_handle.clone(),
            },
        )
        .finish();

        let is_waterfall_gap_mode =
            matches!(input_mode, InputMode::Waterfall) && model.block_list().active_gap().is_some();
        // In waterfall gap mode, we render the bookmark indicators on the waterfall gap element,
        // not the block list element.
        let element_to_save = if !self.bookmarked_blocks.is_empty() && !is_waterfall_gap_mode {
            self.render_bookmark_indicators(model, block_list, appearance, app)
        } else {
            block_list
        };
        SavePosition::new(element_to_save, &self.content_element_position_id).finish()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_waterfall_gap_element(
        &self,
        model: &TerminalModel,
        viewport: &ViewportState,
        active_gap: &Gap,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Stack {
        let input_element = if self.is_input_box_visible(model, app) {
            self.render_input()
        } else {
            // If the active block is running, the input element is empty.
            SavePosition::new(
                ConstrainedBox::new(Empty::new().finish())
                    .with_height(0.)
                    .finish(),
                &self.input.as_ref(app).save_position_id(),
            )
            .finish()
        };
        let waterfall_gap_element = WaterfallGapElement::new(
            self.render_block_list_element(model, InputMode::Waterfall, false, app),
            input_element,
            (model.block_list().block_heights().summary().height)
                .to_pixels(self.size_info.cell_height_px()),
            vec2f(
                self.size_info.pane_width_px().as_f32(),
                active_gap
                    .height()
                    .to_pixels(self.size_info.cell_height_px())
                    .as_f32(),
            ),
            self.size_info.cell_height_px(),
            viewport.scroll_top_in_pixels(),
            self.size_info.pane_height_px(),
        );

        let theme = appearance.theme();

        let scrollable = Scrollable::vertical(
            self.blocklist_vertical_scroll_state.clone(),
            waterfall_gap_element.finish_scrollable(),
            SCROLLBAR_WIDTH,
            theme.disabled_text_color(theme.background()).into(),
            theme.main_text_color(theme.background()).into(),
            Fill::None,
        )
        .finish();
        let gap_element = if !self.bookmarked_blocks.is_empty() {
            self.render_bookmark_indicators(model, scrollable, appearance, app)
        } else {
            scrollable
        };

        Stack::new().with_child(gap_element)
    }

    // In the case of waterfall mode with no gap, we need to handle left and right (for the context menu) clicks
    // in the empty area beneath the input that are typically handled by the block list element.
    fn render_waterfall_mode_background(
        &self,
        model: &TerminalModel,
        mut stack: Stack,
        app: &AppContext,
    ) -> Stack {
        let block_list_height_px = {
            (model.block_list().block_heights().summary().height)
                .to_pixels(self.size_info.cell_height_px)
        };
        let input_position_id: Rc<str> = self.input.as_ref(app).save_position_id().into();
        let position_id: Rc<str> = self.waterfall_background_position_id().into();

        /// Retrieves the offset position below the block.
        fn offset_position_outside_block(
            click_position: Vector2F,
            position_id: &str,
            input_position_id: &str,
            block_list_height_px: Pixels,
            ctx: &mut EventContext,
        ) -> Option<Vector2F> {
            let input_height_px = ctx
                .element_position_by_id(input_position_id)
                .map_or(Pixels::zero(), |r| r.height().into_pixels());
            let Some(rect) = ctx.element_position_by_id(position_id) else {
                log::warn!("'{position_id}' position should be saved");
                return None;
            };

            let offset_position = click_position - rect.origin();

            if offset_position.y().into_pixels() > block_list_height_px + input_height_px {
                Some(offset_position)
            } else {
                None
            }
        }

        // Define a click handler that works for both when the blocklist is totally empty and when we are
        // showing the shortcut hints, and for when there is empty space below the input, but there are blocks
        // above it.
        let click_handler = move |child: Box<dyn Element>| -> Box<dyn Element> {
            let saved = position_id.clone();

            SavePosition::new(
                EventHandler::new(child)
                    .on_right_mouse_down(
                        enclose!((position_id, input_position_id) move |ctx, _app, position | {
                                if let Some(position_in_terminal_view) = offset_position_outside_block(
                                    position,
                                    &position_id,
                                    &input_position_id,
                                    block_list_height_px,
                                    ctx,
                                ) {
                                    ctx.dispatch_typed_action(TerminalAction::BlockListContextMenu(
                                        BlockListMenuSource::OutsideBlockRightClick {
                                            position_in_terminal_view,
                                        },
                                    ));
                                    return DispatchEventResult::StopPropagation;
                                }
                                DispatchEventResult::PropagateToParent
                            }
                        ),
                    )
                    .on_left_mouse_down(
                        enclose!((position_id, input_position_id) move |ctx, _app, position| {
                            if offset_position_outside_block(
                                position,
                                &position_id,
                                &input_position_id,
                                block_list_height_px,
                                ctx,
                            )
                            .is_some()
                            {
                                ctx.dispatch_typed_action(TerminalAction::Focus);
                                return DispatchEventResult::StopPropagation;
                            }
                            DispatchEventResult::PropagateToParent
                        }),
                    )
                    .on_middle_mouse_down(
                        enclose!((position_id, input_position_id) move |ctx, _app, position| {
                            if offset_position_outside_block(
                                position,
                                &position_id,
                                &input_position_id,
                                block_list_height_px,
                                ctx,
                            )
                            .is_some()
                            {
                                ctx.dispatch_typed_action(TerminalAction::MiddleClickOnGrid {
                                    position: None,
                                });
                                return DispatchEventResult::StopPropagation;
                            }
                            DispatchEventResult::PropagateToParent
                        }),
                    )
                    .finish(),
                &saved,
            )
            .for_single_frame()
            .finish()
        };

        stack.add_child(
            Flex::column()
                .with_child(Shrinkable::new(1., click_handler(Empty::new().finish())).finish())
                .finish(),
        );
        stack
    }

    pub fn waterfall_background_position_id(&self) -> String {
        format!("waterfall_background__{}", self.view_id)
    }

    /// Renders the bookmark indicators over the given block list or waterfall gap element
    ///
    /// Will only create indicators if there are blocks bookmarked
    fn render_bookmark_indicators(
        &self,
        model: &TerminalModel,
        scrollable_child: Box<dyn Element>,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let total_block_height = model.block_list().block_heights().summary().height;
        let mut stack = Stack::new();
        stack.add_child(scrollable_child);

        let mut bookmark_position = IndicatorPositionArg {
            remaining_indicator_count: self.bookmarked_blocks.keys().count(),
            previous_indicator_top: Pixels::zero(),
        };

        let input_mode = *InputModeSettings::as_ref(app).input_mode.value();
        for (index, handle) in self
            .bookmarked_blocks
            .iter()
            .sorted_by(|a, b| match input_mode {
                InputMode::PinnedToBottom | InputMode::Waterfall => Ord::cmp(a.0, b.0),
                InputMode::PinnedToTop => Ord::cmp(b.0, a.0),
            })
        {
            let (offset, indicator) = self.create_bookmark_indicator(
                model,
                handle.clone(),
                *index,
                total_block_height,
                &mut bookmark_position,
                appearance,
                input_mode,
                app,
            );

            stack.add_positioned_child(
                indicator,
                OffsetPositioning::offset_from_parent(
                    offset,
                    ParentOffsetBounds::ParentByPosition,
                    ParentAnchor::TopRight,
                    ChildAnchor::TopRight,
                ),
            );

            let hovered = handle
                .lock()
                .expect("Handle should be available")
                .is_hovered();

            if hovered {
                if let Some(block) = model.block_list().block_at(*index) {
                    let snapshot = render_floating_block_snapshot(block, appearance);

                    stack.add_positioned_child(
                        snapshot,
                        OffsetPositioning::offset_from_parent(
                            vec2f(-BOOKMARK_PREVIEW_OFFSET, offset.y()),
                            ParentOffsetBounds::ParentByPosition,
                            ParentAnchor::TopRight,
                            ChildAnchor::TopRight,
                        ),
                    );
                }
            }
        }

        stack.finish()
    }

    /// Create the indicator for a bookmark
    ///
    /// The indicator will be scaled to match the height of the block relative to the total block
    /// list.
    ///
    /// We also return the offset vector from the top-right of the screen to position the indicator
    /// properly.
    #[allow(clippy::too_many_arguments)]
    fn create_bookmark_indicator(
        &self,
        model: &TerminalModel,
        handle: MouseStateHandle,
        index: BlockIndex,
        total_block_height: Lines,
        bookmark_position: &mut IndicatorPositionArg,
        appearance: &Appearance,
        input_mode: InputMode,
        app: &AppContext,
    ) -> (Vector2F, Box<dyn Element>) {
        let viewport = self.viewport_state(model.block_list(), input_mode, app);
        let start = viewport.top_of_block_in_lines(index);

        let top = bookmark_position.next_indicator_top(
            start,
            total_block_height,
            self.size_info.pane_height_px(),
        );

        let element = Hoverable::new(handle, |state| {
            let base_color = appearance.theme().accent().into_solid();
            let color = if state.is_hovered() {
                base_color
            } else {
                darken(base_color)
            };

            ConstrainedBox::new(
                Container::new(Rect::new().finish())
                    .with_background_color(color)
                    .finish(),
            )
            .with_width(BOOKMARK_INDICATOR_WIDTH)
            .with_height(BOOKMARK_INDICATOR_HEIGHT)
            .finish()
        })
        .on_click(move |ctx, _, _| {
            ctx.dispatch_typed_action(TerminalAction::JumpToBookmark(index));
        })
        .finish();

        (vec2f(0., top.as_f32()), element)
    }

    pub fn terminal_position_id(&self) -> String {
        self.position_id.clone()
    }

    fn context_menu_action(&mut self, action: &ContextMenuAction, ctx: &mut ViewContext<Self>) {
        use ContextMenuAction::*;

        match action {
            InsertSelectedText => self.context_menu_insert_selected_text(ctx),
            CopySelectedText => self.context_menu_copy_selected_text(ctx),
            CopyUrl { url_content } => self.context_menu_copy_url(url_content, ctx),
            CopyBlocks => self.context_menu_copy_blocks(ctx),
            CopyBlockCommands => self.context_menu_copy_block_commands(ctx),
            CopyBlockOutputs => self.context_menu_copy_block_outputs(ctx),
            FindWithinBlock => self.find_within_block(ctx),
            ScrollToBottomOfBlock => self.scroll_to_bottom_of_bottommost_selected_block(ctx),
            ScrollToTopOfBlock => self.scroll_to_top_of_topmost_selected_block(ctx),
            ToggleBookmark => self.bookmark_selected_block(ctx),
            CopyPrompt { position, part } => self.copy_prompt(position, part, ctx),
            CopyRprompt => self.copy_rprompt(ctx),
            EditPrompt => self.edit_prompt(ctx),

            EditCLIAgentToolbar => {
                if FeatureFlag::AgentToolbarEditor.is_enabled() {
                    ctx.emit(Event::OpenCLIAgentToolbarEditor);
                }
            }

            OpenWorkflowModal => self.open_workflow_modal(ctx),
            CopyBlockFilteredOutputs => self.context_menu_copy_filtered_block_outputs(ctx),
        }
    }

    fn handle_input_context_menu_action(
        &mut self,
        action: &InputContextMenuAction,
        ctx: &mut ViewContext<Self>,
    ) {
        use InputContextMenuAction::*;

        match action {
            CutSelectedText => self.cut_selected_text_from_input(ctx),
            CopySelectedText => self.copy_selected_text_from_input(ctx),
            SelectAll => self.select_all_text_from_input(ctx),
            Paste => self.paste_in_input(ctx),
            ShowCommandSearch => self.command_search_from_input(ctx),

            SaveAsWorkflow => self.save_as_workflow_from_input(ctx),
            ToggleInputHintText => self.toggle_input_hint_text(ctx),
        }
        self.close_context_menu(ctx, false);
    }

    fn selected_block_accessibility_content(
        &mut self,
        index: BlockIndex,
    ) -> Option<AccessibilityContent> {
        let model = self.model.lock();
        model.block_list().block_at(index).map(|block| {
            let status = if block.has_failed() {
                format!("failed, status code {}", block.exit_code().value())
            } else if block.is_background() {
                "background".to_string()
            } else if block.is_done() {
                "succeeded".to_string()
            } else {
                "in progress".to_string()
            };
            AccessibilityContent::new(
                format!("Block {index}: {}, {}.\n", block.command_to_string(), status),
                // TODO (a11y) Keybindings should be taken from the actual user's
                // configuration
                "Press cmd-C to read and copy both command and output, and cmd-option-shift-C to read and copy output only. Press cmd-B to bookmark the block: you could navigate between bookmarked blocks quickly using option-up and option-down.",
                WarpA11yRole::TextRole,
            )
        })
    }

    fn notifications_error_banner_action(
        &mut self,
        action: NotificationsErrorBannerAction,
        ctx: &mut ViewContext<Self>,
    ) {
        use NotificationsErrorBannerAction::*;

        match action {
            Troubleshoot => {
                ctx.open_url(NOTIFICATIONS_TROUBLESHOOT_URL);
            }
            Close => self.close_notification_error_banner(ctx),
            SetPermissions => {
                ctx.request_desktop_notification_permissions(move |view, outcome, ctx| {
                    // If the request was accepted, we can close the banner. Otherwise, keep it open, indicating the problem
                    // has not been resolved.
                    if matches!(outcome, RequestPermissionsOutcome::Accepted) {
                        view.close_notification_error_banner(ctx);
                    }
                });
            }
        }
    }

    fn close_notification_error_banner(&mut self, ctx: &mut ViewContext<Self>) {
        if let NotificationsErrorBannerType::Open { state, .. } = &self
            .inline_banners_state
            .notifications_error_banner
            .banner_type
        {
            self.model
                .lock()
                .block_list_mut()
                .remove_inline_banner(state.banner_id);
        }
        self.inline_banners_state
            .notifications_error_banner
            .banner_type = NotificationsErrorBannerType::Closed;
        ctx.notify();
    }

    fn notifications_discovery_banner_action(
        &mut self,
        action: NotificationsDiscoveryBannerAction,
        ctx: &mut ViewContext<Self>,
    ) {
        use NotificationsDiscoveryBannerAction::*;

        match action {
            LearnMore => {
                ctx.open_url(NOTIFICATIONS_LEARN_MORE_URL);
            }
            Troubleshoot => {
                ctx.open_url(NOTIFICATIONS_TROUBLESHOOT_URL);
            }
            TurnOn(_) => {
                let current_settings = SessionSettings::as_ref(ctx).notifications.value().clone();
                let new_settings = NotificationsSettings {
                    mode: NotificationsMode::Enabled,
                    ..current_settings
                };
                SessionSettings::handle(ctx).update(ctx, |session_settings, ctx| {
                    if let Err(e) = session_settings.notifications.set_value(new_settings, ctx) {
                        log::error!("Error persisting notifications setting: {e}");
                    }
                });

                // On Linux, immediately mark the request permission status as accepted since there's no concept of
                // requesting desktop notification permissions.
                #[cfg(target_os = "linux")]
                {
                    if let NotificationsDiscoveryBanner::Open {
                        request_outcome, ..
                    } = &mut self.inline_banners_state.notifications_discovery_banner
                    {
                        *request_outcome = Some(RequestPermissionsOutcome::Accepted);
                    }
                }

                ctx.request_desktop_notification_permissions(move |view, outcome, ctx| {
                    if let NotificationsDiscoveryBanner::Open {
                        request_outcome, ..
                    } = &mut view.inline_banners_state.notifications_discovery_banner
                    {
                        *request_outcome = Some(outcome.clone());
                    }
                    // Log unknown permission errors locally.
                    if let RequestPermissionsOutcome::OtherError { error_message } = &outcome {
                        log::error!(
                            "Unknown error when requesting notification permissions. error_msg: {error_message}"
                        );
                    }

                                        ctx.notify();
                });
                ctx.notify();
            }
            Configure => {
                ctx.emit(Event::OpenSettings(SettingsSection::Features));
            }
            Close => {
                // Update settings to mark notifications as dismissed to prevent banner from showing again
                let current_settings = SessionSettings::as_ref(ctx).notifications.value().clone();
                let new_settings = NotificationsSettings {
                    mode: NotificationsMode::Dismissed,
                    ..current_settings
                };
                SessionSettings::handle(ctx).update(ctx, |session_settings, ctx| {
                    if let Err(e) = session_settings.notifications.set_value(new_settings, ctx) {
                        log::error!("Error persisting notifications setting: {e}");
                    }
                });

                if let NotificationsDiscoveryBanner::Open { state, .. } =
                    &self.inline_banners_state.notifications_discovery_banner
                {
                    self.model
                        .lock()
                        .block_list_mut()
                        .remove_inline_banner(state.banner_id);
                }
                self.inline_banners_state.notifications_discovery_banner =
                    NotificationsDiscoveryBanner::Closed;
                ctx.notify();
            }
        }
    }

    fn ssh_banner_action(&self, action: SSHBannerAction, ctx: &mut ViewContext<Self>) {
        use SSHBannerAction::*;

        match action {
            LearnMore => {
                ctx.open_url("https://docs.warp.dev/terminal/warpify/ssh-legacy#implementation");
            }
            Settings => {
                if FeatureFlag::SSHTmuxWrapper.is_enabled() {
                    ctx.emit(Event::OpenSettings(SettingsSection::Warpify));
                } else {
                    ctx.emit(Event::OpenSettings(SettingsSection::Features));
                }
            }
        }
    }

    // Invokes the on_next_frame_drawn API to time from the provided block started at to the moment
    // the frame is drawn.
    // It doesn't matter when this method is called, as long as it's before the next frame is drawn.
    fn install_block_latency_log_callback(
        &mut self,
        block_latency_data: BlockLatencyData,
        ctx: &mut ViewContext<Self>,
    ) {
        let session_info = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
            .map(|session| {
                let shell_name = session.shell().shell_type().name();
                (session.is_legacy_ssh_session(), shell_name)
            });

        if let Some((is_ssh, shell)) = session_info {
            ctx.on_next_frame_drawn(move || {
                log::debug!(
                    "block latency: command={} shell={} is_ssh={} execution_ms={}",
                    block_latency_data.command,
                    shell,
                    is_ssh,
                    block_latency_data.started_at.elapsed().as_millis()
                );
            })
        } else {
            log::warn!("Could not log block latency since session info was none");
        }
    }

    /// Toggles the block filter on the last selected block, or the last non-hidden
    /// block if none are selected.
    ///
    /// When a filter is toggled off, it is set as inactive but the query remains
    /// saved on the block. It can be reactivated by toggling on. If there is no
    /// inactive query, toggling on a filter will simply open the filter editor.
    fn toggle_block_filter_on_selected_or_last_block(&mut self, ctx: &mut ViewContext<Self>) {
        let model = self.model.lock();
        let Some(selected_or_last_block_index) = self
            .selected_blocks
            .tail()
            .or_else(|| model.block_list().last_non_hidden_block_by_index())
        else {
            log::info!("No block found to toggle block filter on");
            return;
        };

        let Some(block_filter_query) = model
            .block_list()
            .block_at(selected_or_last_block_index)
            .map(|block| block.current_filter().cloned())
        else {
            log::warn!("No block found at given block index when toggling filter");
            return;
        };
        drop(model);

        if let Some(block_filter_query) = block_filter_query {
            let new_block_filter_query = BlockFilterQuery {
                is_active: !block_filter_query.is_active,
                ..block_filter_query
            };

            self.update_block_filter_for_block(
                selected_or_last_block_index,
                &new_block_filter_query,
                ctx,
            );
            if new_block_filter_query.is_active {
                self.open_block_filter_editor(
                    selected_or_last_block_index,
                    OpenedFromClick::No,
                    ctx,
                );
            } else {
                self.close_block_filter_editor(ctx);
                self.redetermine_global_focus(ctx);
            }
        } else {
            self.open_block_filter_editor(selected_or_last_block_index, OpenedFromClick::No, ctx);
        }
    }

    /// Replace the terminal input buffer with the given command that is meant to open a subshell.
    /// Set a flag that we should automatically bootstrap AKA "warpify" the subshell when we
    /// receive the [`AfterBlockStarted`] event.
    pub fn insert_subshell_command_and_bootstrap_if_supported(
        &mut self,
        command: &str,
        shell_type: Option<ShellType>,
        ctx: &mut ViewContext<Self>,
    ) {
        // If the shell type is not supported, it will be None.
        self.pending_auto_bootstrap_shell_type = shell_type;

        self.input.update(ctx, |input, ctx| {
            input.replace_buffer_content(command, ctx);
        });
    }

    fn reset_focus_after_rich_block(&mut self, ctx: &mut ViewContext<Self>) {
        self.redetermine_terminal_focus(ctx);
        self.input.update(ctx, |input, ctx| {
            input.editor().update(ctx, |editor, ctx| {
                editor.clear_autosuggestion(ctx);
            });
        });
    }

    pub fn cancel_env_var_block(&mut self, ctx: &mut ViewContext<Self>) {
        if let Some(block) = self.active_env_var_collection_block(ctx) {
            block.update(ctx, |view, ctx| {
                view.cancel(ctx);
            });
        }
    }

    fn add_env_var_block_to_blocklist(
        &mut self,
        collection_title: String,
        command: String,
        session_id: SessionId,
        ctx: &mut ViewContext<Self>,
    ) {
        let block_id = Uuid::new_v4().to_string();
        let env_var_collection_block = ctx.add_typed_action_view(|ctx| {
            EnvVarCollectionBlock::new(block_id.clone(), collection_title, command, ctx)
        });
        env_var_collection_block.update(ctx, |block, ctx| block.focus(ctx));

        ctx.subscribe_to_view(&env_var_collection_block, move |me, block, event, ctx| {
            let event = event.clone();
            match event {
                EnvVarCollectionBlockEvent::RanCommand(command) => {
                    ctx.emit(Event::ExecuteCommand(ExecuteCommandEvent {
                        command,
                        session_id,
                        workflow_id: None,
                        workflow_command: None,
                        should_add_command_to_history: false,
                        source: CommandExecutionSource::EnvVarCollection {
                            metadata: BlocklistEnvVarMetadata {
                                block_id: block_id.clone(),
                                should_hide_block: true,
                            },
                        },
                    }));

                    me.reset_focus_after_rich_block(ctx);
                }
                EnvVarCollectionBlockEvent::Cancelled => {
                    // Send the escape code corresponding to ctrl-c, indicating the running command
                    // should be terminated. Note that this will not revert already-run `export`s.
                    me.keydown_on_terminal("\u{0003}", ctx);
                    me.reset_focus_after_rich_block(ctx);
                }
                EnvVarCollectionBlockEvent::ToggledExpanded(block_id) => {
                    me.model
                        .lock()
                        .block_list_mut()
                        .toggle_visibility_of_block_for_env_var(&block_id);
                    me.redetermine_global_focus(ctx);
                    ctx.notify();
                }
                EnvVarCollectionBlockEvent::TextSelected => {
                    me.clear_selected_text_except(Some(block.id()), ctx);
                }
            }

            ctx.notify();
        });

        self.insert_rich_content(
            None,
            env_var_collection_block.clone(),
            Some(RichContentMetadata::EnvVarCollectionBlock {
                env_var_collection_block_handle: env_var_collection_block,
            }),
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: false,
            },
            ctx,
        );
    }

    fn display_non_local_environment_variable_error(
        &self,
        window_id: WindowId,
        ctx: &mut ViewContext<Self>,
    ) {
        ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
            toast_stack.add_ephemeral_toast(
                DismissibleToast::error(
                    "Can not invoke environment variable subshell in a non-local session"
                        .to_owned(),
                ),
                window_id,
                ctx,
            );
        });
    }

    #[allow(unused_variables)]
    fn get_shell_starter_local(&self, ctx: &mut ViewContext<Self>) -> Option<(String, ShellType)> {
        #[cfg(feature = "local_tty")]
        {
            // TODO(CORE-2300): This appears to be used for invoking env vars.
            // Before we close out CORE-2300, we should evaluate if we need to add
            // shell info here.
            let shell_starter = get_shell_starter(None, ctx)?;
            let shell_path = match &shell_starter {
                ShellStarter::Direct(direct_shell_starter)
                | ShellStarter::MSYS2(direct_shell_starter) => direct_shell_starter
                    .shell_path()
                    .to_string_lossy()
                    .to_string(),

                ShellStarter::Wsl(wsl_shell_starter) => wsl_shell_starter.shell_path(),
            };
            Some((shell_path, shell_starter.shell_type()))
        }

        #[cfg(not(feature = "local_tty"))]
        None
    }

    pub fn invoke_environment_variables(
        &mut self,
        env_var_collection: EnvVarCollection,
        in_subshell: bool,
        ctx: &mut ViewContext<Self>,
    ) {
        let session_id = self.active_block_session_id();

        if !in_subshell {
            let Some(shell_type) = self.active_session_shell_type(ctx) else {
                return;
            };
            self.invoke_env_vars_in_current_session(
                env_var_collection.clone(),
                shell_type,
                session_id,
                ctx,
            );
        } else {
            let window_id = ctx.window_id();
            let shell_session_info =
                if self.active_session_is_local(ctx).unwrap_or(false) || !in_subshell {
                    if let Some(shell_info) = self.get_shell_starter_local(ctx) {
                        shell_info
                    } else {
                        // TODO(PR): This can fail for reasons besides being "non-local". We can also
                        // not find a fallback shell.
                        self.display_non_local_environment_variable_error(window_id, ctx);
                        return;
                    }
                } else {
                    self.display_non_local_environment_variable_error(window_id, ctx);
                    return;
                };

            self.invoke_env_vars_in_subshell(
                env_var_collection,
                shell_session_info,
                window_id,
                ctx,
            );
        }
    }

    fn invoke_env_vars_in_current_session(
        &mut self,
        env_var_collection: EnvVarCollection,
        shell_type: ShellType,
        session_id: Option<SessionId>,
        ctx: &mut ViewContext<Self>,
    ) {
        if let Some(session_id) = session_id {
            self.add_env_var_block_to_blocklist(
                env_var_collection
                    .title
                    .clone()
                    .unwrap_or("Untitled".to_owned()),
                env_var_collection
                    .vars
                    .iter()
                    .map(|var| var.get_initialization_string(shell_type))
                    .collect_vec()
                    .join(" "),
                session_id,
                ctx,
            );
        } else {
            self.pending_env_var_collection = Some(env_var_collection)
        }
    }

    fn set_and_execute_subshell_command(
        &mut self,
        shell_command: &str,
        shell_type: ShellType,
        ctx: &mut ViewContext<Self>,
    ) {
        // Attempt to auto warpify the subshell when bootstrapped
        self.pending_auto_bootstrap_shell_type = Some(shell_type);

        self.input.update(ctx, |input, ctx| {
            input.set_pending_command(shell_command, ctx);
            input.execute_pending_command(ctx);
        });
    }

    fn invoke_env_vars_in_subshell(
        &mut self,
        env_var_collection: EnvVarCollection,
        shell_session_info: (String, ShellType),
        window_id: WindowId,
        ctx: &mut ViewContext<Self>,
    ) {
        let (shell_path_string, shell_type) = shell_session_info;
        if shell_type == ShellType::PowerShell {
            ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
                let toast =
                    DismissibleToast::error("PowerShell subshells not supported".to_owned());
                toast_stack.add_ephemeral_toast(toast, window_id, ctx);
            });
            return;
        }

        // Set the env vars before executing a subshell command so that it will be loaded on
        // subshell start
        self.env_vars = env_var_collection.vars;
        self.model.lock().set_env_var_collection_name(Some(
            env_var_collection.title.unwrap_or("Untitled".to_owned()),
        ));
        self.set_and_execute_subshell_command(&shell_path_string, shell_type, ctx);
    }

    #[cfg(feature = "integration_tests")]
    pub fn active_filter_editor_block_index(&self) -> Option<BlockIndex> {
        self.active_filter_editor_block_index
    }

    fn cursor_position_id(&self) -> String {
        self.cursor_position_id.clone()
    }

    fn drag_and_drop_files(&mut self, paths: &[String], ctx: &mut ViewContext<Self>) {
        self.is_file_drop_target = false;
        if paths.is_empty() {
            return;
        }

        // Focus this pane when files are dropped on it.
        self.redetermine_global_focus(ctx);

        // Check if we're in a long-running command
        let is_in_long_running_command = self
            .model
            .lock()
            .block_list()
            .active_block()
            .is_active_and_long_running();

        let image_filepaths = get_image_filepaths_from_paths(paths);

        if self.has_active_cli_agent_input_session(ctx) {
            let text_paths = paths
                .iter()
                .filter(|path| !image_filepaths.contains(path))
                .cloned()
                .collect::<Vec<_>>();
            if !image_filepaths.is_empty() {
                self.input.update(ctx, |input, ctx| {
                    input.queue_cli_image_paths(image_filepaths, ctx)
                });
            }
            if !text_paths.is_empty() {
                let text = warpui::clipboard_utils::escaped_paths_str(
                    &text_paths,
                    Some(self.shell_family(ctx)),
                );
                self.input.update(ctx, |input, ctx| {
                    input.insert_into_cli_agent_rich_input(&text, ctx)
                });
            }
            return;
        }

        let Some(session) = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
        else {
            return;
        };

        let sshed = self.model.lock().is_warpified_ssh() || session.is_legacy_ssh_session();
        if sshed && !paths.is_empty() && FeatureFlag::SshDragAndDrop.is_enabled() {
            self.initiate_ssh_file_upload(paths, ctx);
        } else {
            // For long-running commands in MSYS2/Git Bash on Windows, skip
            // conversion and shell escaping. Executables in git bash
            // aren't git bash _specific_, they still expect paths in
            // the native windows format.
            let is_msys2_long_running = cfg!(windows)
                && !session.is_wsl()
                && session.shell_family() == ShellFamily::Posix
                && is_in_long_running_command;
            if is_msys2_long_running {
                let input = warpui::clipboard_utils::escaped_paths_str(paths, None);
                self.typed_characters_on_terminal(&input, ctx);
                return;
            }

            // For WSL sessions on Windows, convert paths to /mnt/<drive>/... format
            // so the WSL session can read the file at the correct path.
            let paths_converted;
            let paths = if session.is_wsl() {
                paths_converted = paths
                    .iter()
                    .map(|p| warp_util::path::convert_windows_path_to_wsl(p))
                    .collect::<Vec<_>>();
                paths_converted.as_slice()
            } else {
                paths
            };

            let input =
                warpui::clipboard_utils::escaped_paths_str(paths, Some(self.shell_family(ctx)));
            self.typed_characters_on_terminal(&input, ctx);
        }
    }

    pub fn initiate_ssh_file_upload(&self, paths: &[String], ctx: &mut ViewContext<Self>) {
        let remote_pwd = self.pwd();
        if let Some(ssh_connection_info) = self.ssh_session_info(ctx) {
            let Some(ref ssh_host) = ssh_connection_info.host else {
                return;
            };
            self.ssh_file_upload.update(ctx, |file_upload, ctx| {
                file_upload.start_file_upload(
                    ssh_host,
                    paths,
                    &remote_pwd,
                    &ssh_connection_info,
                    ctx,
                )
            });
        }
    }

    pub fn propagate_password_request(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(Event::FileUploadPasswordPending)
    }

    pub fn propagate_upload_finished_event(
        &mut self,
        exit_code: ExitCode,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.emit(Event::FileUploadFinished(exit_code))
    }

    fn ssh_session_info(&self, ctx: &ViewContext<Self>) -> Option<InteractiveSshCommand> {
        let session = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))?;
        session
            .as_ref()
            .subshell_info()
            .as_ref()
            .and_then(|info| info.ssh_connection_info.clone())
    }

    fn warpify_ssh_session(&mut self, ctx: &mut ViewContext<Self>) {
        self.warpify_state.set_shell_detection_in_progress();
        self.begin_ssh_warpify_timeout(SSH_WARPIFY_TIMEOUT_DURATION, ctx);
        self.clear_line_editor_and_write_to_pty(
            convert_script_to_one_line(&begin_warpify_ssh_session_command(ctx)).into_bytes(),
            ctx,
        );
    }

    fn continue_warpify_ssh_session(
        &mut self,
        uname: &str,
        shell_type: ShellType,
        ctx: &mut ViewContext<Self>,
    ) {
        self.warpify_state.set_shell_type(&shell_type);
        self.model.lock().set_pending_warp_initiated_control_mode();
        if let Some(script) = warpify_ssh_session_command(uname, shell_type, ctx) {
            self.clear_line_editor_and_write_to_pty_with_mac_workaround_hack(
                convert_script_to_one_line(&script).into_bytes(),
                ctx,
            );
        } else {
            self.add_ssh_error_block(
                WarpificationUnavailableReason::UnsupportedShell {
                    shell_name: shell_type.name().to_string(),
                },
                ctx,
            );
        }
    }

    fn install_tmux_and_warpify(
        &mut self,
        ctx: &mut ViewContext<Self>,
        install_method: &TmuxInstallMethod,
    ) {
        let install_with_root_method = install_method.should_use_package_manager;
        let install_script = &install_method.script;
        self.model
            .lock()
            .set_pending_warp_initiated_control_mode_with_install_tmux(install_with_root_method);
        self.clear_line_editor_and_write_to_pty(
            convert_script_to_one_line(install_script).into_bytes(),
            ctx,
        );
    }

    fn begin_ssh_warpify_timeout(&mut self, duration: Duration, ctx: &mut ViewContext<Self>) {
        let timeout_id = self.warpify_state.replace_timeout_id();
        let active_block_id = self.model.lock().block_list().active_block_id().clone();
        let system_details = self
            .warpify_state
            .ssh_block_state()
            .and_then(|s| s.get_system_details(ctx))
            .to_owned();
        self.warpify_state.add_ssh_warpify_timeout_handle(ctx.spawn(
            async move {
                Timer::after(duration).await;
                (timeout_id, active_block_id, system_details)
            },
            |terminal_view, (timeout_id, active_block_id, system_details), ctx| {
                let is_shell_detection =
                    terminal_view.warpify_state.is_shell_detection_in_progress();
                if timeout_id == terminal_view.warpify_state.timeout_id()
                    && terminal_view.model.lock().block_list().active_block_id() == &active_block_id
                {
                    terminal_view.add_ssh_error_block(
                        WarpificationUnavailableReason::Timeout {
                            is_tmux_install: false,
                            is_shell_detection,
                            system_details,
                        },
                        ctx,
                    );
                }
            },
        ));
    }

    fn handle_detected_end_of_ssh_login(
        &mut self,
        check_type: &SshLoginStatus,
        ctx: &mut ViewContext<TerminalView>,
    ) {
        match check_type {
            SshLoginStatus::RecheckBeforeWarpifying => {
                // After we receive a line of output from ssh that is NOT prompting for user input (unlike "Enter passphrase: "),
                // we wait and repeat the check after a small delay in case the state returned to something that's user-input bound.
                // For example, say the output that kicked off this event was "Permission denied, please try again." and
                // ssh will subsequently re-prompt for user input. We want to avoid assuming that ssh authentication is completed until
                // we confirm twice that user input is not currently being requested.
                //
                // Note: 100ms is an estimate, not backed by any particular technical happenings.
                let active_block_id = self.model.lock().block_list().active_block_id().clone();
                ctx.spawn(
                    async {
                        warpui::r#async::Timer::after(Duration::from_secs(3)).await;
                        active_block_id
                    },
                    move |terminal_view, active_block_id, _| {
                        let mut model = terminal_view.model.lock();
                        if model.block_list().active_block_id() == &active_block_id {
                            model.check_for_end_of_ssh_login(true);
                        }
                    },
                );
            }
            SshLoginStatus::ReadyToWarpify => {
                // After the confirmation check, we are confident enough to auto-warpify or offer warpification.
                let Some(command) = &self.warpify_state.get_pending_ssh_command() else {
                    return;
                };
                let ssh_host = &self.warpify_state.get_pending_ssh_host();

                let shell_family = self.shell_family(ctx);
                let warpify_settings = WarpifySettings::as_ref(ctx);

                let ssh_interactive_session_event = evaluate_warpify_ssh_host(
                    command,
                    ssh_host.as_deref(),
                    shell_family,
                    warpify_settings,
                );

                if let SshInteractiveSessionDetected::ShouldPromptWarpification {
                    ref host,
                    ref command,
                } = ssh_interactive_session_event
                {
                    if FeatureFlag::WarpifyFooter.is_enabled() {
                        self.show_warpify_footer(
                            WarpificationMode::ssh(command.clone(), host.to_owned()),
                            ctx,
                        );
                    } else {
                        self.add_ssh_warpify_prompt(command, host.to_owned(), ctx)
                    }
                }
            }
        }
    }

    /// Parses the shell launch data and sets the necessary fields so a shell
    /// indicator is rendered in the tab bar and pane header. Does nothing on
    /// non-Windows platforms.
    pub fn on_active_shell_launch_data_updated(
        &mut self,
        shell_launch_data: Option<ShellLaunchData>,
        ctx: &mut ViewContext<Self>,
    ) {
        if !cfg!(windows) {
            return;
        }

        let shell_indicator_type = shell_launch_data
            .as_ref()
            .and_then(|data| ShellIndicatorType::try_from(data).ok());
        self.shell_indicator_type = shell_indicator_type;
        self.shell_detail = shell_launch_data.map(|launch_data| launch_data.shell_detail());

        // Notify pane header to re-render with updated shell indicator.
        self.pane_configuration.update(ctx, |config, ctx| {
            config.notify_header_content_changed(ctx);
        });
    }

    pub fn shell_indicator_type(&self) -> Option<ShellIndicatorType> {
        self.shell_indicator_type
    }

    /// Shows the warpify footer for a detected subshell/SSH command.
    fn show_warpify_footer(&mut self, mode: WarpificationMode, ctx: &mut ViewContext<Self>) {
        let model = self.model.lock();

        // Don't show the warpify footer when an agent is monitoring the command either.

        drop(model);

        self.use_agent_footer.update(ctx, |footer, ctx| {
            footer.set_warpify_mode(mode, ctx);
        });
        self.maybe_show_use_agent_footer_in_blocklist(ctx);
    }

    fn show_initialization_block(&mut self) {
        self.model
            .lock()
            .block_list_mut()
            .set_show_bootstrap_block(true);
    }

    pub(super) fn toggle_file_tree(&mut self, ctx: &mut ViewContext<Self>) {
        self.toggle_left_panel_file_tree(false, ctx);
    }
}

impl Entity for TerminalView {
    type Event = Event;
}

impl TypedActionView for TerminalView {
    type Action = TerminalAction;

    fn action_accessibility_contents(
        &mut self,
        action: &TerminalAction,
        ctx: &mut ViewContext<Self>,
    ) -> ActionAccessibilityContent {
        use ActionAccessibilityContent::*;
        use TerminalAction::*;

        match action {
            BlockHover(_)
            | BlockSnackbarHover { .. }
            | BlockNearSnackbarHover { .. }
            | MaybeLinkHover { .. } => Empty,
            BlockTextSelect(_) => {
                let semantic_selection = SemanticSelection::as_ref(ctx);
                let model = self.model.lock();
                model
                    .selection_to_string(semantic_selection, self.is_inverted_blocklist(ctx), ctx)
                    .map_or(Empty, |selected| {
                        Custom(AccessibilityContent::new_without_help(
                            selected,
                            WarpA11yRole::TextRole,
                        ))
                    })
            }
            BlockSelect { .. }
            | SelectPriorBlock
            | SelectNextBlock
            | SelectBookmarkUp
            | SelectBookmarkDown
            | Up
            | Down
            | JumpToBookmark(_)
            | ScrollToTopOfBlock { topmost_block: _ } => {
                if let Some(content) = self
                    .selected_blocks
                    .tail()
                    .and_then(|index| self.selected_block_accessibility_content(index))
                {
                    Custom(content)
                } else {
                    Empty
                }
            }
            BookmarkBlock(_) | BookmarkSelectedBlock => {
                Custom(AccessibilityContent::new_without_help(
                    "Toggle Bookmark block",
                    WarpA11yRole::TextRole,
                ))
            }
            ExpandBlockSelectionAbove | ExpandBlockSelectionBelow => {
                if let Some(mut content) = self
                    .selected_blocks
                    .tail()
                    .and_then(|index| self.selected_block_accessibility_content(index))
                {
                    let num_selected_text =
                        format!("Selected {} blocks.", self.num_non_hidden_selected_blocks());
                    content.value = format!("{}\n{}", num_selected_text, content.value);
                    Custom(content)
                } else {
                    Empty
                }
            }
            SelectAllBlocks => Custom(AccessibilityContent::new_without_help(
                format!(
                    "Selected all {} blocks.",
                    self.num_non_hidden_selected_blocks()
                ),
                WarpA11yRole::TextRole,
            )),
            ScrollToBottomOfSelectedBlocks => Custom(AccessibilityContent::new_without_help(
                "Scrolled to bottom of selected block".to_string(),
                WarpA11yRole::TextRole,
            )),
            ScrollToTopOfSelectedBlocks => Custom(AccessibilityContent::new_without_help(
                "Scrolled to top of selected block".to_string(),
                WarpA11yRole::TextRole,
            )),
            ScrollToBottomOfOverhangingBlock(_) => Custom(AccessibilityContent::new_without_help(
                "Scrolled to bottom of bottommost visible block".to_string(),
                WarpA11yRole::TextRole,
            )),
            CopyOutputs => {
                let mut outputs = vec![];
                self.with_non_hidden_selected_blocks(
                    |block| {
                        outputs.push(format!(
                            "Block {}.\nOutput: {}",
                            block.index(),
                            block.output_to_string()
                        ));
                    },
                    ctx,
                );
                let text = format!(
                    "Copied {} block outputs.\n{}",
                    outputs.len(),
                    outputs.join("\n")
                );
                Custom(AccessibilityContent::new_without_help(
                    text,
                    WarpA11yRole::TextRole,
                ))
            }
            Copy => {
                let mut blocks = vec![];
                self.with_non_hidden_selected_blocks(
                    |block| {
                        blocks.push(format!(
                            "Block {}: {}. Output: {}",
                            block.index(),
                            block.command_to_string(),
                            block.output_to_string()
                        ));
                    },
                    ctx,
                );
                let text = format!("Copied {} blocks.\n{}", blocks.len(), blocks.join("\n"));
                Custom(AccessibilityContent::new_without_help(
                    text,
                    WarpA11yRole::TextRole,
                ))
            }
            FocusInputAndClearSelection => {
                Custom(AccessibilityContent::new(
                    INPUT_A11Y_LABEL,
                    // TODO (a11y) use bindings from user settings
                    INPUT_A11Y_HELPER,
                    WarpA11yRole::TextareaRole,
                ))
            }
            KeyDown(key) => {
                let label = if key.eq("\x1b") {
                    INPUT_A11Y_LABEL
                } else {
                    key
                };
                Custom(AccessibilityContent::new_without_help(
                    label,
                    WarpA11yRole::TextareaRole,
                ))
            }
            OpenBlockFilterEditor(block_index) => Custom(AccessibilityContent::new_without_help(
                format!("Open block filter editor for block {block_index}"),
                WarpA11yRole::TextRole,
            )),
            ShowInitializationBlock => Custom(AccessibilityContent::new_without_help(
                "Showed initialization block",
                WarpA11yRole::TextareaRole,
            )),
            ShowWarpifySettings => Custom(AccessibilityContent::new_without_help(
                "Opened Warpify Settings",
                WarpA11yRole::ButtonRole,
            )),
            OpenFilesPalette { .. } => Custom(AccessibilityContent::new_without_help(
                "Opened file search palette",
                WarpA11yRole::ButtonRole,
            )),
            InsertCommandCorrection { .. }
            | BlockListContextMenu(_)
            | CloseContextMenu
            | Paste
            | MiddleClickOnGrid { .. }
            | MiddleClickOnInput
            | CopyCommands
            | MaybeHoverSecret { .. }
            | CopyGitBranch
            | ReinputCommands
            | ReinputCommandsWithSudo
            | ClearBuffer
            | Focus
            | ShowFindBar
            | PageUp
            | PageDown
            | Home
            | End
            | KeyboardSelectText(_)
            | ContextMenu(_)
            | SplitRight(_)
            | SplitLeft(_)
            | SplitDown(_)
            | SplitUp(_)
            | OpenGridLink(_)
            | OpenRichContentLink(_)
            | ToggleGridSecret { .. }
            | CopyGridSecret(_)
            | ShowInFileExplorer(_)
            | OpenFileInWarp(_)
            | CtrlD
            | CtrlC
            | ClearSelections
            | Close
            | TypedCharacters(_)
            | UserInputSequence(_)
            | ControlSequence(_)
            | TriggerSubshellBootstrap
            | ShowSubshellBanner(_)
            | DismissWarpifyBanner(_)
            | OpenBlockListContextMenu
            | AliasExpansionBanner(_)
            | VimModeBanner(_)
            | InsertMostRecentCommandCorrection
            | ImportSettings
            | DragAndDropFiles(_)
            | WarpifySSHSession
            | ShowWarpifySshBanner(_, _)
            | NotifySshErrorBlock(_)
            | ToggleBlockFilterOnSelectedOrLastBlock(_)
            | SetMarkedText { .. }
            | ClearMarkedText => ActionAccessibilityContent::from_debug(),

            #[cfg(feature = "local_fs")]
            OpenCodeInWarp { .. } => ActionAccessibilityContent::from_debug(),
            OpenInWarpBanner(action) => self.open_in_warp_banner_accessibility_content(*action),

            PickRepoToOpen => Custom(AccessibilityContent::new_without_help(
                "Use file picker to select a git repository".to_owned(),
                WarpA11yRole::PopoverRole,
            )),

            // Below are actions that are most likely irrelevant to users or are very noisy and the
            // debug version shouldn't be announced.
            Scroll { .. }
            | AltScroll { .. }
            | ClickOnGrid { .. }
            | MaybeDismissToolTip { .. }
            | MaybeClearAltSelect
            | AltScreenContextMenu { .. }
            | AltSelect(_)
            | AltMouseAction(_)
            | ToggleMaximizePane
            | PromptContextMenu { .. }
            | OpenInputContextMenu { .. }
            | InputContextMenuItem(_)
            | NotificationsDiscoveryBanner(_)
            | NotificationsErrorBanner(_)
            | LegacySSHBanner(_)
            | OpenWorkflowModal
            | OpenWorkflowModalForBlock(_)
            | OpenWorkflowModalWithLocalWorkflow(_)
            | ToggleSnackbarInActivePane
            | HyperlinkClick { .. }
            | StartFileDropTarget
            | StopFileDropTarget
            | RunNativeShellCompletions { .. }
            | DeleteAttachment { .. }
            | ToggleCodeReviewPane { .. }
            | AddProjectAtCurrentDirectory
            | DismissCodeToolbeltTooltip
            | OpenCLIAgentRichInput
            | ToggleSessionRecording
            | Osc52AllowBlockedClipboardOperation => Empty,
        }
    }

    fn handle_action(&mut self, action: &TerminalAction, ctx: &mut ViewContext<Self>) {
        use TerminalAction::*;
        let input_mode = *InputModeSettings::as_ref(ctx).input_mode.value();

        match action {
            Scroll { delta } => self.scroll(*delta, ctx),
            AltScroll { delta } => self.alt_scroll(*delta, ctx),
            ScrollToTopOfBlock { topmost_block } => {
                self.jump_to_previous_command(*topmost_block, ctx)
            }
            ScrollToTopOfSelectedBlocks => self.scroll_to_top_of_topmost_selected_block(ctx),
            ScrollToBottomOfOverhangingBlock(overhanging_block) => {
                self.scroll_to_bottom_of_overhanging_block(overhanging_block, ctx)
            }
            ScrollToBottomOfSelectedBlocks => {
                self.scroll_to_bottom_of_bottommost_selected_block(ctx)
            }
            BlockTextSelect(select_action) => self.block_text_select(select_action, ctx),
            BlockSelect {
                action,
                should_redetermine_focus,
            } => self.block_select(action, *should_redetermine_focus, ctx),
            BlockHover(hover_action) => self.block_hover(hover_action, ctx),
            BlockSnackbarHover { is_hovered } => self.block_snackbar_hover(*is_hovered, ctx),
            BlockNearSnackbarHover { is_hovered } => {
                self.block_near_snackbar_hover(*is_hovered, ctx)
            }
            ClickOnGrid {
                position,
                modifiers,
            } => self.click_on_grid(position, modifiers, ctx),
            MaybeLinkHover {
                position,
                from_editor,
            } => self.maybe_link_hover(position, *from_editor, ctx),
            MaybeHoverSecret { secret_handle } => self.maybe_hover_secret(*secret_handle, ctx),
            MaybeDismissToolTip { from_keybinding } => {
                if !self.dismiss_tooltips(ctx) && *from_keybinding {
                    // If we are not dismissing the link tooltip, pass the esc escape sequence
                    // down to the terminal.
                    self.keydown_on_terminal("\u{1b}", ctx)
                }
            }
            MaybeClearAltSelect => {
                let mut model = self.model.lock();
                if model.alt_screen().selection().is_some() {
                    model.alt_screen_mut().clear_selection();
                    ctx.notify();
                }
            }
            AltSelect(select_action) => self.alt_select(select_action, ctx),
            AltScreenContextMenu { position } => self.alt_screen_context_menu(*position, ctx),
            AltMouseAction(mouse_state) => self.alt_mouse_action(mouse_state, ctx),
            BlockListContextMenu(menu_state) => self.block_list_context_menu(menu_state, ctx),

            CloseContextMenu => self.close_context_menu(ctx, true),
            Paste => self.paste(false, ctx),
            Copy => self.copy(ctx),
            CopyOutputs => self.copy_outputs(ctx),
            CopyCommands => self.copy_commands(ctx),
            CopyGitBranch => {
                let prompt_position = match self.selected_blocks.tail() {
                    Some(selected_block_index) => PromptPosition::Block(selected_block_index),
                    None => PromptPosition::Input,
                };
                self.copy_prompt(&prompt_position, &PromptPart::GitBranch, ctx)
            }
            ReinputCommands => self.reinput_commands(false, ctx),
            ReinputCommandsWithSudo => self.reinput_commands(true, ctx),
            ClearBuffer => self.clear_buffer(ctx),
            Focus => self.redetermine_global_focus(ctx),
            FocusInputAndClearSelection => self.focus_input_and_clear_selections(ctx),
            ShowFindBar => self.show_find_bar(ctx),
            SelectPriorBlock => {
                match input_mode {
                    InputMode::PinnedToBottom | InputMode::Waterfall => {
                        self.select_less_recent_block(false /* is_shift_down */, ctx)
                    }
                    InputMode::PinnedToTop => {
                        self.select_more_recent_block(
                            true,  /* is_cmd_down */
                            false, /* is_shift_down */
                            ctx,
                        )
                    }
                }
            }
            SelectNextBlock => {
                match input_mode {
                    InputMode::PinnedToBottom | InputMode::Waterfall => self
                        .select_more_recent_block(
                            true,  /* is_cmd_down */
                            false, /* is_shift_down */
                            ctx,
                        ),
                    InputMode::PinnedToTop => {
                        self.select_less_recent_block(false /* is_shift_down */, ctx)
                    }
                }
            }
            Up => self.terminal_up(ctx),
            Down => self.terminal_down(ctx),
            PageUp => self.page_up(ctx),
            PageDown => self.page_down(ctx),
            Home => self.move_home(ctx),
            End => self.move_end(ctx),
            KeyboardSelectText(direction) => self.keyboard_select_text(ctx, direction),
            SelectBookmarkUp => match input_mode {
                InputMode::PinnedToBottom | InputMode::Waterfall => self.bookmark_up(ctx),
                InputMode::PinnedToTop => self.bookmark_down(ctx),
            },
            SelectBookmarkDown => match input_mode {
                InputMode::PinnedToBottom | InputMode::Waterfall => self.bookmark_down(ctx),
                InputMode::PinnedToTop => self.bookmark_up(ctx),
            },
            BookmarkSelectedBlock => self.bookmark_selected_block(ctx),
            UserInputSequence(bytes) => self.user_input_sequence(bytes, ctx),
            ControlSequence(bytes) => self.control_sequence_on_terminal(bytes, ctx),
            KeyDown(chars) => self.keydown_on_terminal(chars, ctx),
            TypedCharacters(chars) => self.typed_characters_on_terminal(chars, ctx),
            CtrlD => self.ctrl_d(ctx),
            CtrlC => self.handle_ctrl_c_input_event(0, ctx),
            ClearSelections => self.clear_selections(ctx),
            ContextMenu(context_action) => self.context_menu_action(context_action, ctx),
            Close => ctx.emit(Event::CloseRequested),
            SplitRight(chosen_shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitRight(chosen_shell.to_owned())))
            }
            SplitLeft(chosen_shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitLeft(chosen_shell.to_owned())))
            }
            SplitDown(chosen_shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitDown(chosen_shell.to_owned())))
            }
            SplitUp(chosen_shell) => {
                ctx.emit(Event::Pane(PaneEvent::SplitUp(chosen_shell.to_owned())))
            }
            ToggleMaximizePane => ctx.emit(Event::Pane(PaneEvent::ToggleMaximized)),
            PromptContextMenu {
                position_offset_from_prompt,
            } => self.show_prompt_context_menu(*position_offset_from_prompt, ctx),
            OpenInputContextMenu { position } => self.show_input_context_menu(*position, ctx),
            InputContextMenuItem(action) => self.handle_input_context_menu_action(action, ctx),
            SelectAllBlocks => self.select_all_blocks(ctx),
            BookmarkBlock(index) => self.bookmark_block(index, ctx),
            ExpandBlockSelectionAbove => {
                match input_mode {
                    InputMode::PinnedToBottom | InputMode::Waterfall => {
                        self.select_less_recent_block(true /* is_shift_down */, ctx)
                    }
                    InputMode::PinnedToTop => {
                        self.select_more_recent_block(
                            false, /* is_cmd_down */
                            true,  /* is_shift_down */
                            ctx,
                        )
                    }
                }
            }
            ExpandBlockSelectionBelow => {
                match input_mode {
                    InputMode::PinnedToBottom | InputMode::Waterfall => self
                        .select_more_recent_block(
                            false, /* is_cmd_down */
                            true,  /* is_shift_down */
                            ctx,
                        ),
                    InputMode::PinnedToTop => {
                        self.select_less_recent_block(true /* is_shift_down */, ctx)
                    }
                }
            }
            NotificationsErrorBanner(action) => {
                self.notifications_error_banner_action(*action, ctx)
            }
            NotificationsDiscoveryBanner(action) => {
                self.notifications_discovery_banner_action(*action, ctx)
            }
            LegacySSHBanner(action) => self.ssh_banner_action(*action, ctx),
            JumpToBookmark(index) => self.jump_to_bookmark(*index, ctx),
            InsertCommandCorrection { correction } => {
                self.insert_command_correction(correction, ctx);
            }
            ToggleGridSecret {
                handle,
                show_secret,
            } => self.toggle_grid_secret(handle, *show_secret, ctx),

            CopyGridSecret(secret_handle) => self.copy_grid_secret(secret_handle, ctx),

            OpenGridLink(link) => {
                self.open_highlighted_link(link, ctx);
            }
            OpenRichContentLink(link) => {
                self.open_rich_content_link(link, ctx);
            }
            ShowInFileExplorer(path) => {
                ctx.open_file_path_in_explorer(path);
            }
            OpenFileInWarp(path) => {
                self.open_file_in_warp(path.clone(), ctx);
            }
            #[cfg(feature = "local_fs")]
            OpenCodeInWarp {
                path,
                layout,
                line_col,
            } => {
                self.open_code_in_warp(
                    CodeSource::Link {
                        path: path.clone(),
                        range_start: *line_col,
                        range_end: None,
                    },
                    *layout,
                    ctx,
                );
            }
            OpenWorkflowModal => self.open_workflow_modal(ctx),

            OpenWorkflowModalForBlock(block_index) => {
                self.open_workflow_modal_from_block(*block_index, ctx)
            }
            OpenWorkflowModalWithLocalWorkflow(workflow_id) => {
                self.open_workflow_modal_with_existing(workflow_id.clone(), ctx)
            }
            OpenBlockListContextMenu => self.open_block_list_context_menu_via_keybinding(ctx),

            TriggerSubshellBootstrap => self.trigger_subshell_bootstrap(None, ctx),
            ShowSubshellBanner(command) => {
                // Abort handle is no longer needed since we've waited the 1s already.
                self.warpify_state.take_subshell_banner_abort_handle();

                let warpify_keybinding =
                    keybinding_name_to_keystroke("terminal:warpify_subshell", ctx);
                self.show_warpify_banner(
                    WarpificationMode::subshell(command.to_owned()),
                    "Subshell",
                    "subshell",
                    warpify_keybinding,
                    ctx,
                );
            }
            ShowWarpifySshBanner(command, host) => {
                let warpify_keybinding =
                    keybinding_name_to_keystroke("terminal:warpify_ssh_session", ctx);
                self.show_warpify_banner(
                    WarpificationMode::ssh(command.to_string(), host.to_owned()),
                    "SSH Session",
                    "SSH session",
                    warpify_keybinding,
                    ctx,
                );
            }
            DismissWarpifyBanner(remember) => {
                self.dismiss_warpify_banner(remember, ctx);
            }
            InsertMostRecentCommandCorrection => self.insert_most_recent_command_correction(ctx),
            AliasExpansionBanner(action) => self.alias_expansion_banner_action(*action, ctx),
            OpenInWarpBanner(action) => self.handle_open_in_warp_banner_action(*action, ctx),
            OpenBlockFilterEditor(block_index) => {
                self.open_block_filter_editor(*block_index, OpenedFromClick::Yes, ctx)
            }
            VimModeBanner(action) => self.handle_vim_banner_action(*action, ctx),
            ImportSettings => {
                #[cfg(feature = "local_fs")]
                {
                    self.add_settings_import_block(ctx);
                }
            }
            ToggleBlockFilterOnSelectedOrLastBlock(_) => {
                self.toggle_block_filter_on_selected_or_last_block(ctx);
            }
            ToggleSnackbarInActivePane => self.toggle_snackbar_in_active_pane(ctx),
            MiddleClickOnGrid { position } => self.middle_click_on_grid(position, ctx),
            MiddleClickOnInput => self.middle_click_on_input(ctx),

            DragAndDropFiles(paths) => {
                self.drag_and_drop_files(paths, ctx);
            }
            WarpifySSHSession => self.add_ssh_warpifying_block(ctx),
            NotifySshErrorBlock(action) => {
                if let Some(SshBlockState::Error {
                    handle: ssh_error_block_handle,
                }) = self.warpify_state.ssh_block_state()
                {
                    ssh_error_block_handle.update(ctx, |error_block, ctx| {
                        error_block.handle_action(action, ctx);
                    });
                }
            }

            HyperlinkClick(hyperlink) => {
                ctx.notify();
                ctx.open_url(&hyperlink.url);
            }
            StartFileDropTarget => {
                let Some(session) = self
                    .active_block_session_id()
                    .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
                else {
                    return;
                };
                let sshed = self.model.lock().is_warpified_ssh() || session.is_legacy_ssh_session();
                if sshed && !self.is_file_drop_target {
                    self.is_file_drop_target = true;
                    ctx.notify();
                }
            }
            StopFileDropTarget => {
                if self.is_file_drop_target {
                    self.is_file_drop_target = false;
                    ctx.notify();
                }
            }
            RunNativeShellCompletions {
                buffer_text,
                results_tx,
            } => {
                ctx.emit(Event::RunNativeShellCompletions {
                    buffer_text: buffer_text.clone(),
                    results_tx: results_tx.clone(),
                });
            }
            SetMarkedText {
                marked_text,
                selected_range,
            } => self.set_marked_text_on_terminal(marked_text, selected_range, ctx),
            ClearMarkedText => self.clear_marked_text_on_terminal(ctx),
            ShowInitializationBlock => self.show_initialization_block(),

            ShowWarpifySettings => ctx.emit(Event::OpenSettings(SettingsSection::Warpify)),
            DeleteAttachment { index: _ } => {}

            ToggleCodeReviewPane { entrypoint } => {
                ctx.emit(Event::ToggleCodeReviewPane(CodeReviewPanelArg {
                    repo_path: self.current_repo_path.clone(),
                    terminal_view: self.view_handle.clone(),
                    entrypoint: *entrypoint,
                    focus_new_pane: true,
                    cli_agent: None,
                }));
            }

            AddProjectAtCurrentDirectory => {
                // Get the current working directory and add it as a project
                if let Some(current_dir) = self.pwd() {
                    let path = PathBuf::from(&current_dir);

                    // Access the ProjectManagementModel and add the project
                    ProjectManagementModel::handle(ctx).update(ctx, |project_model, ctx| {
                        project_model.upsert_project(path, ctx);
                    });
                }
            }

            PickRepoToOpen => {
                ctx.dispatch_typed_action(&WorkspaceAction::OpenRepository { path: None });
            }
            OpenFilesPalette { source } => ctx.emit(Event::OpenFilesPalette { source: *source }),
            DismissCodeToolbeltTooltip => {
                CodeSettings::handle(ctx).update(ctx, |settings, ctx| {
                    if let Err(e) = settings
                        .dismissed_code_toolbelt_new_feature_popup
                        .set_value(true, ctx)
                    {
                        log::warn!(
                            "Failed to mark code toolbelt new feature popup as dismissed: {e}"
                        );
                    }
                });
                ctx.notify();
            }

            ToggleSessionRecording => {
                self.pty_recorder.update(ctx, |recorder, ctx| {
                    recorder.toggle_recording(ctx);
                });
            }
            OpenCLIAgentRichInput => {
                if self.has_active_cli_agent_input_session(ctx) {
                    self.close_cli_agent_rich_input_and_disable_auto_toggle(ctx);
                } else {
                    self.open_cli_agent_rich_input(CLIAgentInputEntrypoint::CtrlG, ctx);
                }
            }
            Osc52AllowBlockedClipboardOperation => {
                use crate::terminal::settings::Osc52ClipboardAccess;
                if let Some(blocked_type) = self.osc52_clipboard_blocked_type {
                    let new_value = match blocked_type {
                        Osc52ClipboardBlockedType::Write => Osc52ClipboardAccess::WriteOnly,
                        Osc52ClipboardBlockedType::Read => Osc52ClipboardAccess::ReadWrite,
                    };
                    TerminalSettings::handle(ctx).update(ctx, |terminal_settings, ctx| {
                        let _ = terminal_settings
                            .osc52_clipboard_access
                            .set_value(new_value, ctx);
                    });
                    self.osc52_clipboard_blocked_type = None;
                    ctx.notify();
                }
            }
        }
    }
}

impl View for TerminalView {
    fn ui_name() -> &'static str {
        "Terminal"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let semantic_selection = SemanticSelection::as_ref(app);
        let model = self.model.lock();
        let input_mode = *InputModeSettings::as_ref(app).input_mode.value();
        let viewport = self.viewport_state(model.block_list(), input_mode, app);
        let is_alt_screen_active = { model.is_alt_screen_active() };
        let is_long_running_command = {
            model
                .block_list()
                .active_block()
                .is_active_and_long_running()
        };

        let mut column = match input_mode {
            InputMode::PinnedToTop => Flex::column().with_reverse_orientation(),
            InputMode::PinnedToBottom | InputMode::Waterfall => Flex::column(),
        };

        let mut did_wrap_terminal_size = false;

        fn wrap_in_terminal_size_element(
            resize_tx: &Sender<Vector2F>,
            element: Box<dyn Element>,
        ) -> Box<dyn Element> {
            TerminalSizeElement::new(resize_tx.clone(), element).finish()
        }

        let mut stack = match (
            input_mode,
            model.block_list().active_gap(),
            is_alt_screen_active,
        ) {
            (InputMode::Waterfall, Some(active_gap), false) => {
                self.render_waterfall_gap_element(&model, &viewport, active_gap, appearance, app)
            }
            (input_mode, _, _) => {
                let output_area = if is_alt_screen_active {
                    did_wrap_terminal_size = true;
                    wrap_in_terminal_size_element(
                        &self.resize_tx,
                        self.render_alt_screen_element(
                            app,
                            &model,
                            model.alt_screen().selection_range(semantic_selection),
                        ),
                    )
                } else {
                    self.render_block_list_element(&model, input_mode, true, app)
                };

                column.add_child(Shrinkable::new(1., output_area).finish());

                if model.is_alt_screen_active() && self.should_render_use_agent_footer(&model, app)
                {
                    column.add_child(ChildView::new(&self.use_agent_footer).finish());
                }

                if self.is_input_box_visible(&model, app) {
                    column.add_child(self.render_input());
                }

                let stack = Stack::new()
                    .with_constrain_absolute_children()
                    .with_child(column.finish());
                if matches!(input_mode, InputMode::Waterfall) && !is_alt_screen_active {
                    self.render_waterfall_mode_background(&model, stack, app)
                } else {
                    stack
                }
            }
        };

        if self.is_any_tooltip_open() {
            self.render_grid_tooltip(&mut stack, &model, appearance, app);
        }

        match &self.context_menu_state.map(|c| c.menu_type) {
            Some(ContextMenuType::BlockList { menu_source }) => match menu_source {
                BlockListMenuSource::BlockOverflowButton { block_index }
                | BlockListMenuSource::BlockKeybinding { block_index } => stack
                    .add_positioned_overlay_child(
                        ChildView::new(&self.context_menu).finish(),
                        OffsetPositioning::offset_from_save_position_element(
                            format!("context_menu_button_{block_index}").as_str(),
                            vec2f(OVERFLOW_BUTTON_OFFSET_X, 0.),
                            PositionedElementOffsetBounds::WindowByPosition,
                            PositionedElementAnchor::TopLeft,
                            ChildAnchor::TopRight,
                        ),
                    ),

                BlockListMenuSource::RegularBlockRightClick {
                    position_in_terminal_view,
                    ..
                }
                | BlockListMenuSource::RegularTextRightClick {
                    position_in_terminal_view,
                }
                | BlockListMenuSource::RichContentBlockRightClick {
                    position_in_terminal_view,
                    ..
                }
                | BlockListMenuSource::OutsideBlockRightClick {
                    position_in_terminal_view,
                } => stack.add_positioned_overlay_child(
                    ChildView::new(&self.context_menu).finish(),
                    OffsetPositioning::offset_from_parent(
                        *position_in_terminal_view,
                        ParentOffsetBounds::WindowByPosition,
                        ParentAnchor::TopLeft,
                        ChildAnchor::TopLeft,
                    ),
                ),

                BlockListMenuSource::RichContentTextRightClick {
                    rich_content_view_id,
                    position_in_rich_content,
                } => stack.add_positioned_overlay_child(
                    ChildView::new(&self.context_menu).finish(),
                    OffsetPositioning::offset_from_save_position_element(
                        get_rich_content_position_id(rich_content_view_id),
                        *position_in_rich_content,
                        PositionedElementOffsetBounds::WindowByPosition,
                        PositionedElementAnchor::TopLeft,
                        ChildAnchor::TopLeft,
                    ),
                ),
            },
            Some(ContextMenuType::Prompt { position }) => stack.add_positioned_overlay_child(
                ChildView::new(&self.context_menu).finish(),
                OffsetPositioning::offset_from_save_position_element(
                    format!("prompt_area_{}", self.input.id()),
                    *position,
                    PositionedElementOffsetBounds::WindowByPosition,
                    PositionedElementAnchor::TopLeft,
                    ChildAnchor::BottomLeft,
                ),
            ),
            Some(ContextMenuType::AltScreen { position }) => stack.add_positioned_overlay_child(
                ChildView::new(&self.context_menu).finish(),
                OffsetPositioning::offset_from_parent(
                    *position,
                    ParentOffsetBounds::WindowByPosition,
                    ParentAnchor::TopLeft,
                    ChildAnchor::TopLeft,
                ),
            ),
            Some(ContextMenuType::Input { position }) => stack.add_positioned_overlay_child(
                ChildView::new(&self.context_menu).finish(),
                match input_mode {
                    InputMode::PinnedToBottom => {
                        OffsetPositioning::offset_from_save_position_element(
                            self.input.as_ref(app).editor_save_position_id(),
                            *position,
                            PositionedElementOffsetBounds::WindowByPosition,
                            PositionedElementAnchor::TopLeft,
                            ChildAnchor::BottomLeft,
                        )
                    }
                    InputMode::PinnedToTop | InputMode::Waterfall => {
                        OffsetPositioning::offset_from_save_position_element(
                            self.input.as_ref(app).editor_save_position_id(),
                            *position,
                            PositionedElementOffsetBounds::WindowByPosition,
                            PositionedElementAnchor::TopLeft,
                            ChildAnchor::TopLeft,
                        )
                    }
                },
            ),

            None => {}
        }

        if self.find_model.as_ref(app).is_find_bar_open() {
            stack.add_child(ChildView::new(&self.find_bar).finish());
        }

        if let Some(active_filter_editor_block_index) = self.active_filter_editor_block_index {
            stack.add_positioned_overlay_child(
                ChildView::new(&self.block_filter_editor).finish(),
                OffsetPositioning::offset_from_save_position_element(
                    filter_button_position_id(active_filter_editor_block_index),
                    vec2f(34., 12.),
                    PositionedElementOffsetBounds::ParentByPosition,
                    PositionedElementAnchor::BottomRight,
                    ChildAnchor::TopRight,
                ),
            );
        }

        // Only show one of these banners at a time, to avoid them visually
        // stacking on top of each other.
        if self.is_slow_bootstrap_banner_open
            && ContextFlag::ShowSlowShellStartupBanner.is_enabled()
        {
            stack.add_child(ChildView::new(&self.slow_bootstrap_banner).finish());
        } else if self.control_master_error_banner_state.is_open {
            stack.add_child(ChildView::new(&self.control_master_error_banner).finish());
        } else if self.is_incompatible_configuration_banner_open {
            stack.add_child(ChildView::new(&self.incompatible_configuration_banner).finish());
        } else if self.is_emacs_bindings_banner_open {
            stack.add_child(ChildView::new(&self.emacs_bindings_banner).finish());
        } else if self.osc52_clipboard_blocked_type.is_some() {
            stack.add_child(ChildView::new(&self.osc52_clipboard_blocked_banner).finish());
        }

        let block_list_settings = BlockListSettings::handle(app);
        let is_jump_to_bottom_enabled = *block_list_settings
            .as_ref(app)
            .show_jump_to_bottom_of_block_button
            .value();
        if is_jump_to_bottom_enabled {
            if let Some(overhanging_block) = viewport.overhanging_bottom_block(app) {
                let button_hovered = self.is_jump_to_bottom_of_block_element_hovered();
                let block_hovered = self
                    .hovered_block_index
                    .is_some_and(|hovered_index| hovered_index == overhanging_block.block_index());
                if (button_hovered || block_hovered)
                    && overhanging_block.visible_block_height_px()
                        > *JUMP_TO_BOTTOM_OVERHANG_THRESHOLD_PX
                {
                    let positioning = match (input_mode, self.is_input_box_visible(&model, app)) {
                        (InputMode::PinnedToBottom | InputMode::Waterfall, true) => {
                            // In waterfall or pinned to bottom mode, the button is positioned relative to the top right
                            // of the input area
                            OffsetPositioning::offset_from_save_position_element(
                                self.input.as_ref(app).save_position_id(),
                                vec2f(-10. - SCROLLBAR_WIDTH.as_f32(), -10.),
                                PositionedElementOffsetBounds::WindowByPosition,
                                PositionedElementAnchor::TopRight,
                                ChildAnchor::BottomRight,
                            )
                        }
                        // In pinned to top mode or the input is not visible, the button is positioned relative to the bottom right
                        // of the parent element
                        (_, _) => OffsetPositioning::offset_from_parent(
                            vec2f(-10. - SCROLLBAR_WIDTH.as_f32(), -10.),
                            ParentOffsetBounds::ParentByPosition,
                            ParentAnchor::BottomRight,
                            ChildAnchor::BottomRight,
                        ),
                    };
                    stack.add_positioned_child(
                        self.render_jump_to_bottom_of_block_element(
                            overhanging_block,
                            is_long_running_command,
                            appearance,
                            app,
                        ),
                        positioning,
                    );
                }
            }
        }

        // Add a border above the input view when there's an overhanging block (or below in input at the top
        // mode).

        if self.ssh_file_upload.as_ref(app).has_upload() {
            stack.add_child(
                Align::new(ChildView::new(&self.ssh_file_upload).finish())
                    .bottom_right()
                    .finish(),
            );
        }

        let element = if !did_wrap_terminal_size {
            wrap_in_terminal_size_element(
                &self.resize_tx,
                SavePosition::new(stack.finish(), &self.terminal_position_id()).finish(),
            )
        } else {
            SavePosition::new(stack.finish(), &self.terminal_position_id()).finish()
        };

        let final_element = if self.is_file_drop_target && FeatureFlag::SshDragAndDrop.is_enabled()
        {
            Container::new(element)
                .with_foreground_overlay(appearance.theme().accent_overlay())
                .finish()
        } else {
            element
        };

        final_element
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            self.maybe_report_focus_in(ctx);
            ctx.dispatch_typed_action(&PaneGroupAction::HandleFocusChange);

            ctx.notify();
        }
    }

    fn on_blur(&mut self, blur_ctx: &BlurContext, ctx: &mut ViewContext<Self>) {
        if blur_ctx.is_self_blurred() {
            // Make sure to close link tooltips when terminal view is not in focus.
            self.open_grid_link_tool_tip.take();

            self.maybe_report_focus_out(ctx);
            ctx.notify();
        }
    }

    fn keymap_context(&self, app: &AppContext) -> warpui::keymap::Context {
        let mut context = Self::default_keymap_context();
        context.map.insert(
            "TerminalView_BlockSelectionCardinality",
            self.selected_blocks.cardinality().as_keymap_context_value(),
        );

        let model_lock = self.model.lock();
        if model_lock.is_block_list_empty() {
            context.set.insert("TerminalView_EmptyBlockList");
        } else {
            context.set.insert("TerminalView_NonEmptyBlockList");
        }

        if self.is_input_box_visible(&model_lock, app) {
            context.set.insert(INPUT_BOX_VISIBLE_KEY);
        }

        if self.input.as_ref(app).editor().as_ref(app).is_focused() {
            context.set.insert("EditorFocused");
        }

        if model_lock.block_list().selection().is_some() {
            context.set.insert("ActiveBlockTextSelection");
        }

        if model_lock.alt_screen().selection().is_some() {
            context.set.insert("ActiveAltScreenSelection");
        }

        if model_lock.is_alt_screen_active() {
            context.set.insert("AltScreen");
        }

        let active_block = model_lock.block_list().active_block();
        if active_block.is_active_and_long_running() && !model_lock.is_alt_screen_active() {
            context.set.insert("LongRunningCommand");
        }

        // Add keyboard protocol context if enabled.
        if model_lock.is_term_mode_set(TermMode::KEYBOARD_PROTOCOL) {
            context.set.insert(init::KEYBOARD_PROTOCOL_ENABLED_KEY);
        }

        if CLIAgentSessionsModel::as_ref(app)
            .session(self.view_id)
            .is_some()
        {
            context.set.insert(init::CLI_AGENT_SESSION_ACTIVE_KEY);
            if *CLIAgentSettings::as_ref(app).should_render_cli_agent_footer {
                context.set.insert(flags::CLI_AGENT_FOOTER_ENABLED);

                if is_rich_input_chip_in_cli_toolbar(app) {
                    context.set.insert(flags::CLI_AGENT_RICH_INPUT_CHIP_ENABLED);
                }
            }
        }

        if let Some(WithinBlockBanner::WarpifyBanner(state)) =
            model_lock.block_list().active_block().block_banner()
        {
            if state.is_ssh() {
                context.set.insert("SshWarpificationBanner");
            } else {
                context.set.insert("SubshellBanner");
            }
        }

        // Also set the warpify context when the footer (flag-gated replacement
        // for the in-block banner) is active, so the ctrl-i keybinding works.
        if let Some(warpify_mode) = self.use_agent_footer.as_ref(app).warpify_mode(app) {
            if warpify_mode.is_ssh() {
                context.set.insert("SshWarpificationBanner");
            } else {
                context.set.insert("SubshellBanner");
            }
        }

        if let Some(SshBlockState::Error { .. }) = self.warpify_state.ssh_block_state() {
            context.set.insert(SSH_ERROR_BLOCK_VISIBLE_KEY);
        }

        if self.current_repo_path.is_some() {
            context.set.insert("InsideRepository");
        }

        context.set.insert("TerminalLocalSession");

        #[cfg(feature = "local_fs")]
        {
            let imported_config_model = ImportedConfigModel::as_ref(app);
            if !imported_config_model.finished_searching_for_settings()
                || imported_config_model.configs().count() >= 1
            {
                context.set.insert(flags::HAS_SETTINGS_TO_IMPORT_FLAG);
            }
        }

        context
    }

    fn active_cursor_position(&self, ctx: &ViewContext<Self>) -> Option<CursorInfo> {
        let cursor_id = self.cursor_position_id();
        let appearance = Appearance::as_ref(ctx);
        let font_size = appearance.monospace_font_size();

        ctx.element_position_by_id(cursor_id)
            .map(|position| CursorInfo {
                position,
                font_size,
            })
    }

    fn accessibility_data(&self, ctx: &mut ViewContext<Self>) -> Option<AccessibilityData> {
        const PER_BLOCK_LINE_LIMIT: usize = 5000;

        let terminal_session_content = if self.model.lock().is_alt_screen_active() {
            self.model.lock().alt_screen().output_to_string()
        } else {
            let last_five_blocks_content = {
                let model = self.model.lock();
                let blocks = model
                    .block_list()
                    .blocks()
                    .iter()
                    .filter(|block| block.is_visible())
                    .rev()
                    .take(5)
                    .collect_vec();

                // Produce a final string of the contents of each block, followed by the input.
                blocks
                    .iter()
                    .rev()
                    .map(|block| block.contents_to_string_with_line_limit(PER_BLOCK_LINE_LIMIT))
                    .collect_vec()
            };

            if self.is_input_box_visible(&self.model.lock(), ctx) {
                let (prompt_text, _rprompt) = self.input.as_ref(ctx).prompt_and_rprompt_text(ctx);
                let input_text = self.input.as_ref(ctx).buffer_text(ctx);
                last_five_blocks_content
                    .into_iter()
                    .chain([format!("{prompt_text} {input_text}")])
                    .join("\n")
            } else {
                last_five_blocks_content.join("\n")
            }
        };

        Some(AccessibilityData {
            content: terminal_session_content,
        })
    }
}

/// A menu positioning provider for when the input is rendered within the terminal.
struct TerminalViewMenuPositioningProvider {
    parent: WeakViewHandle<TerminalView>,
}

impl MenuPositioningProvider for TerminalViewMenuPositioningProvider {
    fn menu_position(&self, app: &AppContext) -> MenuPositioning {
        if let Some(terminal_view) = self.parent.upgrade(app) {
            let view_ref = terminal_view.as_ref(app);
            let TerminalView {
                model, size_info, ..
            } = view_ref;
            let model = model.lock();
            let input_mode = *InputModeSettings::as_ref(app).input_mode.value();
            let total_block_height_px = (model.block_list().block_heights().summary().height)
                .to_pixels(size_info.cell_height_px);

            // Menus are positioned as follows:
            // - Always above the input for pinned to bottom
            // - Always below the input for pinned to top
            // - In Waterfall mode with no-gap, conditionally above or below depending
            //   on whether the blocks take up more or less than half of the viewport size.
            // - In Waterfall mode with a gap, conditionally above or below depending
            //   on the size of the gap and the scroll position.  Basically, if the input is
            //   is more than halfway down the screen, the menus render above; otherwise they render below.
            let positioning = match (input_mode, model.block_list().active_gap()) {
                (InputMode::PinnedToBottom, _) => MenuPositioning::AboveInputBox,
                (InputMode::PinnedToTop, _) => MenuPositioning::BelowInputBox,
                (InputMode::Waterfall, None) => {
                    let height_ratio =
                        total_block_height_px.as_f32() / size_info.pane_height_px().as_f32();
                    if height_ratio < 0.5 {
                        MenuPositioning::BelowInputBox
                    } else {
                        MenuPositioning::AboveInputBox
                    }
                }
                (InputMode::Waterfall, Some(gap)) => {
                    let viewport = view_ref.viewport_state(model.block_list(), input_mode, app);
                    let scroll_top_px = viewport
                        .scroll_top_in_lines()
                        .to_pixels(size_info.cell_height_px());
                    // Calculate how far into the viewport the input is (in pixels from the top).
                    let input_position_in_viewport_px = total_block_height_px
                        - gap.height().to_pixels(size_info.cell_height_px())
                        - scroll_top_px;
                    let height_ratio = input_position_in_viewport_px.as_f32()
                        / size_info.pane_height_px().as_f32();
                    if height_ratio < 0.5 {
                        MenuPositioning::BelowInputBox
                    } else {
                        MenuPositioning::AboveInputBox
                    }
                }
            };
            return positioning;
        }

        // If we can't upgrade the terminal view reference, fall back to using just the InputMode setting
        let input_mode = *InputModeSettings::as_ref(app).input_mode.value();

        match input_mode {
            InputMode::PinnedToBottom => MenuPositioning::AboveInputBox,
            InputMode::PinnedToTop => MenuPositioning::BelowInputBox,
            InputMode::Waterfall => {
                // For Waterfall mode without terminal view context, default to BelowInputBox
                MenuPositioning::BelowInputBox
            }
        }
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        if let Some((is_bootstrapped, pending_shell, has_pending_ssh_session)) =
            self.model.try_lock().map(|model| {
                (
                    model.block_list().is_bootstrapped(),
                    model.pending_shell_type(),
                    model.has_pending_ssh_session(),
                )
            })
        {
            if has_pending_ssh_session || !is_bootstrapped {
                // Only treat session abandonment as an error if the session was
                // visible to the user at some point.  This filters out
                // bootstrap "failures" such as oh-my-zsh prompting the user
                // about an update while we're sourcing their rcfiles - we'll
                // never technically finish bootstrapping the shell.  If that
                // occurs in some non-visible tab, we don't want to conflate it
                // (an unanswered prompt) with an actual failure to bootstrap
                // the shell.
                let log_level = if self.was_ever_visible {
                    log::Level::Error
                } else {
                    log::Level::Warn
                };
                log::log!(
                    log_level,
                    "Session abandoned before bootstrap for shell {pending_shell:?} on ssh {has_pending_ssh_session}"
                );

                let was_ever_visible = self.was_ever_visible;
                let duration_since_start =
                    self.bootstrap_start.unwrap_or_else(Instant::now).elapsed();
                let _ = (was_ever_visible, duration_since_start);
            }
        };
    }
}

/// Returns an instance of [`SizeInfo`] that is to be used
/// when in the blocklist.
///
/// This should really only be used when it's only possible to be
/// in the blocklist (e.g. starting a session / creating a [`TerminalModel`]).
/// Otherwise, use [`create_size_info`].
pub fn create_size_info_for_blocklist(
    pane_size: Vector2F,
    font_cache: &FontCache,
    font_family_id: FamilyId,
    font_size: f32,
    line_height_ratio: f32,
) -> SizeInfo {
    let cell_size_px =
        grid_cell_dimensions(font_cache, font_family_id, font_size, line_height_ratio);

    // Note: `SizeInfo` treats the padding as symmetric, so for bottom-only padding we divide by 2
    let padding_x = PADDING_LEFT.into_pixels();
    let padding_y = (cell_size_px.y() * LONG_RUNNING_BOTTOM_PADDING_LINES / 2.).into_pixels();

    SizeInfo::new(
        pane_size,
        cell_size_px.x().into_pixels(),
        cell_size_px.y().into_pixels(),
        padding_x,
        padding_y,
    )
}

/// Returns an instance of [`SizeInfo`] that accounts for the current
/// terminal mode (alt-screen vs. blocklist).
#[allow(clippy::too_many_arguments)]
pub fn create_size_info(
    pane_size: Vector2F,
    model: &TerminalModel,
    sessions: &Sessions,
    font_cache: &FontCache,
    font_family_id: FamilyId,
    font_size: f32,
    line_height_ratio: f32,
    ctx: &AppContext,
) -> SizeInfo {
    let cell_size_px =
        grid_cell_dimensions(font_cache, font_family_id, font_size, line_height_ratio);
    let active_command = model
        .block_list()
        .active_block()
        .top_level_command(sessions);

    match *TerminalSettings::as_ref(ctx).alt_screen_padding {
        AltScreenPaddingMode::Custom { uniform_padding }
            if FeatureFlag::RemoveAltScreenPadding.is_enabled()
                && model.is_alt_screen_active()
                // If we don't know what the top-level command is,
                // it's not denylisted so we use the custom padding.
                && active_command.is_none_or(|cmd| {
                    !ALT_SCREEN_APPS_THAT_MUST_MATCH_BLOCKLIST_PADDING.contains(cmd.as_str())
                }) =>
        {
            SizeInfo::new(
                pane_size,
                cell_size_px.x().into_pixels(),
                cell_size_px.y().into_pixels(),
                uniform_padding,
                uniform_padding,
            )
        }
        _ => create_size_info_for_blocklist(
            pane_size,
            font_cache,
            font_family_id,
            font_size,
            line_height_ratio,
        ),
    }
}

/// Returns CellSizeAndWindowPadding for the given font params and line height.
pub fn cell_size_and_padding(
    font_cache: &FontCache,
    font_family_id: FamilyId,
    font_size: f32,
    line_height_ratio: f32,
) -> CellSizeAndWindowPadding {
    let cell_size_px =
        grid_cell_dimensions(font_cache, font_family_id, font_size, line_height_ratio);
    let (padding_x_px, padding_y_px) = (
        *PADDING_LEFT,
        cell_size_px.y() * LONG_RUNNING_BOTTOM_PADDING_LINES / 2.,
    );

    CellSizeAndWindowPadding {
        cell_width_px: cell_size_px.x().into_pixels(),
        cell_height_px: cell_size_px.y().into_pixels(),
        padding_x_px: padding_x_px.into_pixels(),
        padding_y_px: padding_y_px.into_pixels(),
    }
}

fn command_first_word_and_suffix(command: &str) -> Option<(&str, &str)> {
    let first_word = command.split_whitespace().next()?;
    let word_start = command.find(first_word)?;
    let rest = &command[word_start + first_word.len()..];
    Some((first_word, rest))
}

/// Conditionally wrap a terminal element (altscreen / blocklist element) in a scrollable element.
/// TODO: We should not conditionally composite the scrollable element.
#[allow(clippy::too_many_arguments)]
fn maybe_wrap_terminal_element_in_scrollable(
    is_scrollable_vertical: bool,
    is_scrollable_horizontal: bool,
    vertical_scroll_handle: ScrollStateHandle,
    horizontal_scroll_handle: ClippedScrollStateHandle,
    required_terminal_width: f32,
    theme: &WarpTheme,
    element: impl NewScrollableElement + 'static,
) -> Box<dyn Element> {
    let nonactive_thumb_background = theme.disabled_text_color(theme.background()).into();
    let active_thumb_background = theme.main_text_color(theme.background()).into();
    let track_background = Fill::None;
    let scrollbar_appearance = ScrollableAppearance::new(SCROLLBAR_WIDTH, true);
    match (is_scrollable_vertical, is_scrollable_horizontal) {
        (true, true) => {
            let config = DualAxisConfig::Manual {
                horizontal: AxisConfiguration::Clipped(ClippedAxisConfiguration {
                    handle: horizontal_scroll_handle,
                    max_size: Some(required_terminal_width),
                    stretch_child: false,
                }),
                vertical: AxisConfiguration::Manual(vertical_scroll_handle),
                child: NewScrollableElement::finish_scrollable(element),
            };

            NewScrollable::horizontal_and_vertical(
                config,
                nonactive_thumb_background,
                active_thumb_background,
                track_background,
            )
            .with_horizontal_scrollbar(scrollbar_appearance)
            .with_vertical_scrollbar(scrollbar_appearance)
            .finish()
        }
        (true, false) => {
            let config = SingleAxisConfig::Manual {
                handle: vertical_scroll_handle,
                child: NewScrollableElement::finish_scrollable(element),
            };
            NewScrollable::vertical(
                config,
                nonactive_thumb_background,
                active_thumb_background,
                track_background,
            )
            .with_vertical_scrollbar(scrollbar_appearance)
            .finish()
        }
        (false, true) => {
            let config = SingleAxisConfig::Clipped {
                handle: horizontal_scroll_handle,
                child: ConstrainedBox::new(element.finish())
                    .with_max_width(required_terminal_width)
                    .finish(),
            };
            NewScrollable::horizontal(
                config,
                nonactive_thumb_background,
                active_thumb_background,
                track_background,
            )
            .with_horizontal_scrollbar(scrollbar_appearance)
            .finish()
        }
        (false, false) => element.finish(),
    }
}

/// Returns `true` when the Rich Input chip is present in the user's CLI agent
/// footer toolbar configuration.
fn is_rich_input_chip_in_cli_toolbar(app: &AppContext) -> bool {
    let sel = &SessionSettings::as_ref(app).cli_agent_footer_chip_selection;
    sel.left_items()
        .iter()
        .chain(sel.right_items().iter())
        .any(|item| matches!(item, AgentToolbarItemKind::RichInput))
}

#[cfg(test)]
#[path = "view_test.rs"]
mod tests;
