use crate::ai::active_agent_views_model::ActiveAgentViewsModel;
use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};
use crate::ai::blocklist::{format_credits, BlocklistAIHistoryEvent, BlocklistAIHistoryModel};
use crate::ai::conversation_navigation::ConversationNavigationData;
use crate::features::FeatureFlag;
use crate::ui_components::icons::Icon;
use crate::workspace::{RestoreConversationLayout, WorkspaceAction};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp_core::ui::theme::{color::internal_colors, WarpTheme};
use warpui::color::ColorU;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum StatusFilter {
    #[default]
    All,
    Working,
    Done,
    Failed,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum CreatedOnFilter {
    #[default]
    All,
    Last24Hours,
    Past3Days,
    LastWeek,
}

#[derive(Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct AgentManagementFilters {
    #[serde(default)]
    pub status: StatusFilter,
    #[serde(default)]
    pub created_on: CreatedOnFilter,
}

impl AgentManagementFilters {
    pub fn reset_all_but_owner(&mut self) {
        self.status = StatusFilter::default();
        self.created_on = CreatedOnFilter::default();
    }

    pub fn is_filtering(&self) -> bool {
        self.status != StatusFilter::default() || self.created_on != CreatedOnFilter::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRunDisplayStatus {
    ConversationInProgress,
    ConversationSucceeded,
    ConversationError,
    ConversationBlocked { blocked_action: String },
    ConversationCancelled,
}

impl AgentRunDisplayStatus {
    pub fn from_conversation_status(status: &ConversationStatus) -> Self {
        match status {
            ConversationStatus::InProgress => Self::ConversationInProgress,
            ConversationStatus::Success => Self::ConversationSucceeded,
            ConversationStatus::Error => Self::ConversationError,
            ConversationStatus::Cancelled => Self::ConversationCancelled,
            ConversationStatus::Blocked { blocked_action } => Self::ConversationBlocked {
                blocked_action: blocked_action.clone(),
            },
        }
    }

    pub fn status_filter(&self) -> StatusFilter {
        match self {
            AgentRunDisplayStatus::ConversationInProgress => StatusFilter::Working,
            AgentRunDisplayStatus::ConversationSucceeded => StatusFilter::Done,
            AgentRunDisplayStatus::ConversationError
            | AgentRunDisplayStatus::ConversationBlocked { .. }
            | AgentRunDisplayStatus::ConversationCancelled => StatusFilter::Failed,
        }
    }

    pub fn is_cancellable(&self) -> bool {
        self.is_working()
    }

    pub fn is_working(&self) -> bool {
        matches!(self, AgentRunDisplayStatus::ConversationInProgress)
    }

    pub fn status_icon_and_color(&self, theme: &WarpTheme) -> (Icon, ColorU) {
        match self {
            AgentRunDisplayStatus::ConversationInProgress => {
                (Icon::ClockLoader, theme.ansi_fg_magenta())
            }
            AgentRunDisplayStatus::ConversationSucceeded => (Icon::Check, theme.ansi_fg_green()),
            AgentRunDisplayStatus::ConversationError => (Icon::Triangle, theme.ansi_fg_red()),
            AgentRunDisplayStatus::ConversationBlocked { .. } => {
                (Icon::StopFilled, theme.ansi_fg_yellow())
            }
            AgentRunDisplayStatus::ConversationCancelled => {
                (Icon::StopFilled, internal_colors::neutral_5(theme))
            }
        }
    }
}

impl std::fmt::Display for AgentRunDisplayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRunDisplayStatus::ConversationInProgress => write!(f, "In progress"),
            AgentRunDisplayStatus::ConversationSucceeded => write!(f, "Done"),
            AgentRunDisplayStatus::ConversationError => write!(f, "Error"),
            AgentRunDisplayStatus::ConversationBlocked { .. } => write!(f, "Blocked"),
            AgentRunDisplayStatus::ConversationCancelled => write!(f, "Cancelled"),
        }
    }
}

/// Stores local conversation metadata needed for display in conversation views.
pub struct ConversationMetadata {
    pub nav_data: ConversationNavigationData,
}

pub enum ConversationItem<'a> {
    Conversation(&'a ConversationMetadata),
}

impl ConversationItem<'_> {
    pub fn title(&self, app: &AppContext) -> String {
        let ConversationItem::Conversation(metadata) = self;
        let history_model = BlocklistAIHistoryModel::as_ref(app);
        history_model
            .conversation(&metadata.nav_data.id)
            .and_then(|conv| conv.title().clone())
            .unwrap_or(metadata.nav_data.title.clone())
    }

    pub fn status(&self, app: &AppContext) -> ConversationStatus {
        let ConversationItem::Conversation(metadata) = self;
        let history_model = BlocklistAIHistoryModel::as_ref(app);
        history_model
            .conversation(&metadata.nav_data.id)
            .map(|conv| conv.status().clone())
            .unwrap_or(ConversationStatus::Success)
    }

    pub fn display_status(&self, app: &AppContext) -> AgentRunDisplayStatus {
        AgentRunDisplayStatus::from_conversation_status(&self.status(app))
    }

    pub fn display_request_usage(&self, app: &AppContext) -> Option<String> {
        let ConversationItem::Conversation(metadata) = self;
        let history_model = BlocklistAIHistoryModel::as_ref(app);
        history_model
            .conversation(&metadata.nav_data.id)
            .map(|conv| conv.credits_spent())
            .or_else(|| {
                history_model
                    .get_conversation_metadata(&metadata.nav_data.id)
                    .and_then(|m| m.credits_spent)
            })
            .map(format_credits)
    }

    pub fn last_updated(&self) -> DateTime<Utc> {
        let ConversationItem::Conversation(metadata) = self;
        metadata.nav_data.last_updated.into()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        let ConversationItem::Conversation(metadata) = self;
        metadata.nav_data.last_updated.into()
    }

    pub fn navigation_data(&self) -> &ConversationNavigationData {
        let ConversationItem::Conversation(metadata) = self;
        &metadata.nav_data
    }

    fn matches_status(&self, status_filter: &StatusFilter, app: &AppContext) -> bool {
        match status_filter {
            StatusFilter::All => true,
            StatusFilter::Working | StatusFilter::Done | StatusFilter::Failed => {
                self.display_status(app).status_filter() == *status_filter
            }
        }
    }

    pub fn get_open_action(
        &self,
        restore_layout: Option<RestoreConversationLayout>,
        app: &AppContext,
    ) -> WorkspaceAction {
        let ConversationItem::Conversation(metadata) = self;
        let is_active =
            ActiveAgentViewsModel::as_ref(app).is_conversation_open(metadata.nav_data.id, app);
        let nav_data = &metadata.nav_data;
        WorkspaceAction::RestoreOrNavigateToConversation {
            conversation_id: nav_data.id,
            window_id: nav_data.window_id,
            pane_view_locator: is_active.then_some(nav_data.pane_view_locator).flatten(),
            terminal_view_id: nav_data.terminal_view_id,
            restore_layout,
        }
    }
}

pub struct AgentConversationsModel {
    conversations: HashMap<AIConversationId, ConversationMetadata>,
}

pub enum AgentConversationsModelEvent {
    ConversationsLoaded,
}

impl Entity for AgentConversationsModel {
    type Event = AgentConversationsModelEvent;
}

impl SingletonEntity for AgentConversationsModel {}

impl AgentConversationsModel {
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let history_model = BlocklistAIHistoryModel::handle(ctx);
        ctx.subscribe_to_model(&history_model, move |me, event, ctx| {
            me.handle_history_event(event, ctx);
        });

        let active_views_model = ActiveAgentViewsModel::handle(ctx);
        ctx.subscribe_to_model(&active_views_model, |me, _event, ctx| {
            me.sync_conversations(ctx);
        });

        let mut model = Self {
            conversations: HashMap::new(),
        };

        model.sync_conversations(ctx);
        model
    }

    pub fn sync_conversations(&mut self, ctx: &mut ModelContext<Self>) {
        if !FeatureFlag::InteractiveConversationManagementView.is_enabled() {
            return;
        }

        self.conversations = ConversationNavigationData::all_conversations(ctx)
            .into_iter()
            .map(|nav_data| (nav_data.id, ConversationMetadata { nav_data }))
            .collect();

        ctx.emit(AgentConversationsModelEvent::ConversationsLoaded);
    }

    pub fn has_items(&self) -> bool {
        !self.conversations.is_empty()
    }

    pub fn has_conversations(&self) -> bool {
        !self.conversations.is_empty()
    }

    fn handle_history_event(
        &mut self,
        event: &BlocklistAIHistoryEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        if !FeatureFlag::InteractiveConversationManagementView.is_enabled() {
            return;
        }
        match event {
            BlocklistAIHistoryEvent::StartedNewConversation { .. }
            | BlocklistAIHistoryEvent::SetActiveConversation { .. }
            | BlocklistAIHistoryEvent::AppendedExchange { .. }
            | BlocklistAIHistoryEvent::SplitConversation { .. }
            | BlocklistAIHistoryEvent::RestoredConversations { .. }
            | BlocklistAIHistoryEvent::RemoveConversation { .. }
            | BlocklistAIHistoryEvent::DeletedConversation { .. }
            | BlocklistAIHistoryEvent::ClearedConversationsInTerminalView { .. }
            | BlocklistAIHistoryEvent::ClearedActiveConversation { .. } => {
                self.sync_conversations(ctx);
            }
            BlocklistAIHistoryEvent::UpdatedConversationArtifacts { .. }
            | BlocklistAIHistoryEvent::UpdatedConversationStatus { .. }
            | BlocklistAIHistoryEvent::CreatedSubtask { .. }
            | BlocklistAIHistoryEvent::UpgradedTask { .. }
            | BlocklistAIHistoryEvent::ReassignedExchange { .. }
            | BlocklistAIHistoryEvent::UpdatedTodoList { .. }
            | BlocklistAIHistoryEvent::UpdatedAutoexecuteOverride { .. }
            | BlocklistAIHistoryEvent::UpdatedConversationMetadata { .. }
            | BlocklistAIHistoryEvent::UpdatedStreamingExchange { .. }
            | BlocklistAIHistoryEvent::ConversationServerTokenAssigned { .. } => {}
        }
    }

    pub fn get_tasks_and_conversations(
        &self,
        filters: &AgentManagementFilters,
        app: &AppContext,
    ) -> impl Iterator<Item = ConversationItem<'_>> {
        let status_filter = move |conversation: &ConversationItem| {
            conversation.matches_status(&filters.status, app)
        };

        let now = Utc::now();
        let created_cutoff = match filters.created_on {
            CreatedOnFilter::All => None,
            CreatedOnFilter::Last24Hours => Some(now - chrono::Duration::hours(24)),
            CreatedOnFilter::Past3Days => Some(now - chrono::Duration::days(3)),
            CreatedOnFilter::LastWeek => Some(now - chrono::Duration::days(7)),
        };

        let created_on_filter = move |conversation: &ConversationItem| match created_cutoff {
            Some(cutoff) => conversation.created_at() >= cutoff,
            None => true,
        };

        self.conversations
            .values()
            .map(ConversationItem::Conversation)
            .filter(status_filter)
            .filter(created_on_filter)
            .sorted_by(|a, b| b.last_updated().cmp(&a.last_updated()))
    }

    pub fn conversations_iter(&self) -> impl Iterator<Item = ConversationItem<'_>> {
        self.conversations
            .values()
            .map(ConversationItem::Conversation)
            .sorted_by(|a, b| b.last_updated().cmp(&a.last_updated()))
    }

    pub fn get_conversation(
        &self,
        conversation_id: &AIConversationId,
    ) -> Option<ConversationItem<'_>> {
        self.conversations
            .get(conversation_id)
            .map(ConversationItem::Conversation)
    }

    pub(crate) fn reset(&mut self) {
        self.conversations.clear();
    }
}

#[cfg(test)]
#[path = "agent_conversations_model_tests.rs"]
mod tests;
