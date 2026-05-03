use chrono::{DateTime, Utc};
use derivative::Derivative;
use std::sync::Arc;
use uuid::Uuid;
use warp_graphql::scalars::time::ServerTimestamp;
use warpui::{r#async::FutureId, Entity, ModelContext, SingletonEntity};

use super::ids::{ClientId, ObjectUid, ServerId, SyncId};
use crate::ai::mcp::templatable::CloudTemplatableMCPServerModel;
use crate::server::cloud_objects::update_manager::InitiatedBy;
use crate::{
    ai::cloud_agent_config::AgentConfigObjectModel,
    ai::cloud_environments::AmbientAgentEnvironmentObjectModel,
    ai::{
        ambient_agents::scheduled::CloudScheduledAmbientAgentModel,
        execution_profiles::CloudAIExecutionProfileModel, facts::CloudAIFactModel,
        mcp::CloudMCPServerModel,
    },
    cloud_object::{
        model::actions::{
            ObjectAction, ObjectActionHistory, ObjectActionSubtype, ObjectActionType,
        },
        CloudObject, CloudObjectEventEntrypoint, GenericStringObjectFormat,
        GenericStringObjectUniqueKey, ObjectType, Owner, Revision, RevisionAndLastEditor,
        ServerCloudObject, ServerCreationInfo,
    },
    drive::{folders::CloudFolderModel, CloudObjectTypeAndId},
    env_vars::CloudEnvVarCollectionModel,
    notebooks::CloudNotebookModel,
    settings::cloud_preferences::CloudPreferenceModel,
    workflows::{workflow_enum::CloudWorkflowEnumModel, CloudWorkflowModel},
};

/// Serialized local model payload used by retained notebook/workflow/env-var
/// persistence. WARPER-001 removed the hosted sync queue that used to upload
/// these payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedModel(String);

impl SerializedModel {
    pub fn new(s: String) -> Self {
        Self(s)
    }

    pub fn model_as_str(&self) -> &str {
        &self.0
    }

    pub fn take(self) -> String {
        self.0
    }
}

impl From<String> for SerializedModel {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GenericStringObjectToCreate {
    pub id: ClientId,
    pub format: GenericStringObjectFormat,
    pub serialized_model: Arc<SerializedModel>,
    pub initial_folder_id: Option<SyncId>,
    pub entrypoint: CloudObjectEventEntrypoint,
    pub uniqueness_key: Option<GenericStringObjectUniqueKey>,
    pub initiated_by: InitiatedBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QueueItemId(Uuid);

impl QueueItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Derivative, Debug)]
#[derivative(PartialEq, Eq, Clone)]
pub enum QueueItem {
    CreateObject {
        object_type: ObjectType,
        owner: Owner,
        id: ClientId,
        title: Option<Arc<String>>,
        serialized_model: Option<Arc<SerializedModel>>,
        initial_folder_id: Option<SyncId>,
        entrypoint: CloudObjectEventEntrypoint,
        initiated_by: InitiatedBy,
    },
    CreateWorkflow {
        object_type: ObjectType,
        owner: Owner,
        id: ClientId,
        #[derivative(PartialEq = "ignore")]
        model: Arc<CloudWorkflowModel>,
        initial_folder_id: Option<SyncId>,
        entrypoint: CloudObjectEventEntrypoint,
        initiated_by: InitiatedBy,
    },
    BulkCreateGenericStringObjects {
        owner: Owner,
        objects: Vec<GenericStringObjectToCreate>,
    },
    UpdateNotebook {
        model: Arc<CloudNotebookModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateWorkflow {
        model: Arc<CloudWorkflowModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateFolder {
        id: SyncId,
        model: Arc<CloudFolderModel>,
    },
    UpdateLocalPreference {
        model: Arc<CloudPreferenceModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateEnvVarCollection {
        model: Arc<CloudEnvVarCollectionModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateWorkflowEnum {
        model: Arc<CloudWorkflowEnumModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateAIFact {
        model: Arc<CloudAIFactModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateMCPServer {
        model: Arc<CloudMCPServerModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateAIExecutionProfile {
        model: Arc<CloudAIExecutionProfileModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateTemplatableMCPServer {
        model: Arc<CloudTemplatableMCPServerModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateAgentEnvironment {
        model: Arc<AmbientAgentEnvironmentObjectModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateScheduledAmbientAgent {
        model: Arc<CloudScheduledAmbientAgentModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    UpdateAgentConfig {
        model: Arc<AgentConfigObjectModel>,
        id: SyncId,
        revision: Option<Revision>,
    },
    RecordObjectAction {
        id_and_type: CloudObjectTypeAndId,
        action_type: ObjectActionType,
        action_timestamp: DateTime<Utc>,
        data: Option<String>,
    },
}

impl QueueItem {
    pub fn from_cached_objects(
        objects: impl Iterator<Item = Box<dyn CloudObject>>,
    ) -> Vec<QueueItem> {
        objects
            .filter_map(|object| {
                object
                    .create_object_queue_item(
                        CloudObjectEventEntrypoint::default(),
                        InitiatedBy::User,
                    )
                    .or_else(|| Some(object.update_object_queue_item(None)))
            })
            .collect()
    }

    pub fn from_unsynced_actions(
        actions: impl Iterator<Item = (CloudObjectTypeAndId, ObjectAction)>,
    ) -> Vec<QueueItem> {
        actions
            .filter_map(|(id_and_type, action)| match action.action_subtype {
                ObjectActionSubtype::SingleAction {
                    timestamp,
                    data,
                    pending: true,
                    ..
                } => Some(QueueItem::RecordObjectAction {
                    id_and_type,
                    action_type: action.action_type,
                    action_timestamp: timestamp,
                    data,
                }),
                _ => None,
            })
            .collect()
    }
}

#[derive(Derivative, Clone, Debug)]
#[derivative(PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub enum CreationFailureReason {
    UniqueKeyConflict {
        id: String,
        initiated_by: InitiatedBy,
    },
    Denied {
        message: String,
        client_id: ClientId,
        initiated_by: InitiatedBy,
    },
    Other {
        id: String,
        initiated_by: InitiatedBy,
    },
}

#[derive(Derivative, Clone, Debug)]
#[derivative(PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
#[allow(clippy::large_enum_variant)]
pub enum LocalObjectQueueEvent {
    ObjectCreationSuccessful {
        server_creation_info: ServerCreationInfo,
        client_id: ClientId,
        revision_and_editor: RevisionAndLastEditor,
        metadata_ts: ServerTimestamp,
        initiated_by: InitiatedBy,
    },
    ObjectUpdateSuccessful {
        server_id: ServerId,
        revision_and_editor: RevisionAndLastEditor,
    },
    ObjectUpdateRejected {
        id: String,
        #[derivative(PartialEq = "ignore")]
        object: Arc<ServerCloudObject>,
    },
    ObjectUpdateFeatureNotAvailable {
        id: String,
    },
    ObjectCreationFailure {
        reason: CreationFailureReason,
    },
    ObjectUpdateFailure {
        id: SyncId,
    },
    ReportObjectActionFailed {
        uid: ObjectUid,
        action_timestamp: DateTime<Utc>,
    },
    ReportObjectActionSucceeded {
        uid: ObjectUid,
        action_timestamp: DateTime<Utc>,
        action_history: ObjectActionHistory,
    },
}

/// Local-only object mutation queue.
///
/// WARPER-001 removed the remote object worker. This queue only keeps
/// pending local mutation records available for SQLite-oriented callers; it
/// never spawns network work or retries.
pub struct LocalObjectQueue {
    queue: Vec<(QueueItemId, QueueItem)>,
    spawned_futures: Vec<FutureId>,
    should_dequeue: bool,
}

impl LocalObjectQueue {
    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(Default::default(), ctx)
    }

    pub fn new(queue_items: Vec<QueueItem>, _ctx: &mut ModelContext<Self>) -> Self {
        Self {
            queue: queue_items
                .into_iter()
                .map(|queue_item| (QueueItemId::new(), queue_item))
                .collect(),
            spawned_futures: vec![],
            should_dequeue: false,
        }
    }

    pub fn queue(&self) -> &[(QueueItemId, QueueItem)] {
        &self.queue
    }

    pub fn is_dequeueing(&self) -> bool {
        self.should_dequeue
    }

    pub fn start_dequeueing(&mut self, _ctx: &mut ModelContext<Self>) {
        self.should_dequeue = false;
    }

    pub fn stop_dequeueing(&mut self) {
        self.should_dequeue = false;
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn enqueue(&mut self, item: QueueItem, _ctx: &mut ModelContext<Self>) -> QueueItemId {
        let queue_id = QueueItemId::new();
        self.queue.push((queue_id, item));
        queue_id
    }

    pub fn spawned_futures(&self) -> &[FutureId] {
        &self.spawned_futures
    }
}

impl Entity for LocalObjectQueue {
    type Event = LocalObjectQueueEvent;
}

impl SingletonEntity for LocalObjectQueue {}
