use crate::ai::agent::conversation::{AIConversationId, ConversationStatus};

use super::{ConversationDetailsData, CreditsInfo, PanelMode};

#[test]
fn default_details_are_local_conversation_details() {
    let data = ConversationDetailsData::default();

    assert!(matches!(
        data.mode,
        PanelMode::Conversation {
            directory: None,
            ai_conversation_id: None,
            status: None,
        }
    ));
}

#[test]
fn conversation_mode_carries_only_local_conversation_id() {
    let conversation_id = AIConversationId::new();
    let data = ConversationDetailsData {
        mode: PanelMode::Conversation {
            directory: Some("/tmp/project".to_string()),
            ai_conversation_id: Some(conversation_id),
            status: Some(ConversationStatus::Success),
        },
        title: "Local conversation".to_string(),
        credits: Some(CreditsInfo::LocalConversation(1.0)),
        ..Default::default()
    };

    assert!(matches!(
        data.mode,
        PanelMode::Conversation {
            ai_conversation_id: Some(id),
            ..
        } if id == conversation_id
    ));
}
