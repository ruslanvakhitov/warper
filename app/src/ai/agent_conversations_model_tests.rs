use chrono::{Duration, Local};
use warpui::App;

use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::history_model::BlocklistAIHistoryModel;
use crate::ai::conversation_navigation::ConversationNavigationData;

use super::{
    AgentConversationsModel, AgentManagementFilters, AgentRunDisplayStatus, ConversationItem,
    ConversationMetadata, CreatedOnFilter, StatusFilter,
};

fn create_test_model() -> AgentConversationsModel {
    AgentConversationsModel {
        conversations: Default::default(),
    }
}

fn create_test_conversation_metadata(
    conversation_id: AIConversationId,
    title: &str,
    last_updated: chrono::DateTime<Local>,
) -> ConversationMetadata {
    ConversationMetadata {
        nav_data: ConversationNavigationData {
            id: conversation_id,
            title: title.to_string(),
            initial_query: None,
            last_updated,
            terminal_view_id: None,
            window_id: None,
            pane_view_locator: None,
            initial_working_directory: None,
            latest_working_directory: None,
            is_selected: false,
            is_in_active_pane: false,
            is_closed: false,
            server_conversation_token: None,
        },
    }
}

#[test]
fn display_status_maps_conversation_states() {
    assert_eq!(
        AgentRunDisplayStatus::from_conversation_status(
            &crate::ai::agent::conversation::ConversationStatus::InProgress,
        ),
        AgentRunDisplayStatus::ConversationInProgress
    );
    assert_eq!(
        AgentRunDisplayStatus::from_conversation_status(
            &crate::ai::agent::conversation::ConversationStatus::Success,
        )
        .status_filter(),
        StatusFilter::Done
    );
    assert_eq!(
        AgentRunDisplayStatus::from_conversation_status(
            &crate::ai::agent::conversation::ConversationStatus::Error,
        )
        .status_filter(),
        StatusFilter::Failed
    );
}

#[test]
fn get_tasks_and_conversations_returns_local_conversations_only() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));

        let mut model = create_test_model();
        let older_id = AIConversationId::new();
        let newer_id = AIConversationId::new();
        let now = Local::now();
        model.conversations.insert(
            older_id,
            create_test_conversation_metadata(older_id, "Older", now - Duration::hours(2)),
        );
        model.conversations.insert(
            newer_id,
            create_test_conversation_metadata(newer_id, "Newer", now),
        );

        app.update(|ctx| {
            let ids: Vec<_> = model
                .get_tasks_and_conversations(&AgentManagementFilters::default(), ctx)
                .map(|item| item.navigation_data().id)
                .collect();

            assert_eq!(ids, vec![newer_id, older_id]);
        });
    });
}

#[test]
fn created_on_filter_excludes_old_conversations() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));

        let mut model = create_test_model();
        let recent_id = AIConversationId::new();
        let old_id = AIConversationId::new();
        let now = Local::now();
        model.conversations.insert(
            recent_id,
            create_test_conversation_metadata(recent_id, "Recent", now - Duration::hours(1)),
        );
        model.conversations.insert(
            old_id,
            create_test_conversation_metadata(old_id, "Old", now - Duration::days(2)),
        );

        app.update(|ctx| {
            let ids: Vec<_> = model
                .get_tasks_and_conversations(
                    &AgentManagementFilters {
                        created_on: CreatedOnFilter::Last24Hours,
                        ..Default::default()
                    },
                    ctx,
                )
                .map(|item| item.navigation_data().id)
                .collect();

            assert_eq!(ids, vec![recent_id]);
        });
    });
}

#[test]
fn legacy_filter_payload_ignores_removed_remote_filters() {
    let legacy = r#"{
        "owners": "PersonalOnly",
        "status": "All",
        "source": "All",
        "created_on": "All",
        "creator": "All",
        "artifact": "All",
        "environment": "All",
        "harness": "claude"
    }"#;

    let decoded: AgentManagementFilters =
        serde_json::from_str(legacy).expect("legacy remote filters should be ignored");
    assert_eq!(decoded, AgentManagementFilters::default());
}

#[test]
fn conversation_item_open_action_targets_local_conversation() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| BlocklistAIHistoryModel::new(vec![], &[]));

        let conversation_id = AIConversationId::new();
        let metadata = create_test_conversation_metadata(conversation_id, "Local", Local::now());
        let item = ConversationItem::Conversation(&metadata);

        app.update(|ctx| {
            let action = item.get_open_action(None, ctx);
            match action {
                crate::workspace::WorkspaceAction::RestoreOrNavigateToConversation {
                    conversation_id: action_id,
                    ..
                } => assert_eq!(action_id, conversation_id),
                other => panic!("unexpected action: {other:?}"),
            }
        });
    });
}
