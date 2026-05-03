use warpui::AppContext;

use crate::{ai::agent::conversation::AIConversationId, server::ids::ServerId};

pub use warp_server_client::drive::sharing::{
    LinkSharingSubjectType, SharingAccessLevel, Subject, TeamKind, UserKind,
};

#[derive(Debug, Clone)]
pub enum ShareableObject {
    LocalObject(ServerId),
    AIConversation(AIConversationId),
}

impl ShareableObject {
    pub fn link(&self, _app: &AppContext) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ContentEditability {
    ReadOnly,
    RequiresLogin,
    Editable,
}

impl ContentEditability {
    pub fn can_edit(self) -> bool {
        matches!(self, ContentEditability::Editable)
    }
}
