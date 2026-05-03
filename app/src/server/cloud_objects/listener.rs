use crate::{
    cloud_object::{
        model::actions::ObjectActionHistory, ServerCloudObject, ServerMetadata, ServerPermissions,
    },
    server::ids::ServerId,
    workspaces::user_profiles::UserProfileWithUID,
};
use chrono::{DateTime, Utc};
use warpui::{Entity, ModelContext, SingletonEntity};

pub enum ListenerEvent {}

/// Local-only shell for the removed remote object subscription listener.
pub struct Listener;

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum ObjectUpdateMessage {
    ObjectMetadataChanged {
        metadata: ServerMetadata,
    },
    ObjectPermissionsChanged,
    ObjectPermissionsChangedV2 {
        object_uid: ServerId,
        permissions: ServerPermissions,
        user_profiles: Vec<UserProfileWithUID>,
    },
    ObjectContentChanged {
        server_object: Box<ServerCloudObject>,
        last_editor: Option<UserProfileWithUID>,
    },
    ObjectDeleted {
        object_uid: ServerId,
    },
    ObjectActionOccurred {
        history: ObjectActionHistory,
    },
    TeamMembershipsChanged,
    AmbientTaskUpdated {
        task_id: String,
        timestamp: DateTime<Utc>,
    },
}

impl Listener {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    #[cfg(test)]
    pub fn mock(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn has_current_subscription_abort_handle(&self) -> bool {
        false
    }
}

impl Entity for Listener {
    type Event = ListenerEvent;
}

impl SingletonEntity for Listener {}
