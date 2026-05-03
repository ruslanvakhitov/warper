#[cfg(not(target_family = "wasm"))]
use crate::ai::mcp::templatable::{CloudTemplatableMCPServerModel, TemplatableMCPServer};
use crate::{
    ai::{
        ambient_agents::scheduled::ScheduledAmbientAgent,
        cloud_environments::{AmbientAgentEnvironment, AmbientAgentEnvironmentObjectModel},
        execution_profiles::{AIExecutionProfile, CloudAIExecutionProfileModel},
        facts::{AIFact, CloudAIFactModel},
    },
    cloud_object::{
        model::{
            actions::{ObjectAction, ObjectActionHistory, ObjectActionType, ObjectActions},
            generic_string_model::{
                GenericStringModel, GenericStringObjectId, Serializer, StringModel,
            },
            persistence::{CloudModel, CloudModelEvent, UpdateSource},
        },
        CloudModelType, CloudObject, CloudObjectEventEntrypoint, CloudObjectLocation,
        GenericCloudObject, GenericStringObjectFormat, JsonObjectType, ObjectIdType, Owner,
        Revision, ServerFolder, ServerNotebook, ServerObject, ServerWorkflow, Space,
    },
    drive::{
        folders::{CloudFolderModel, FolderId},
        CloudObjectTypeAndId,
    },
    env_vars::{CloudEnvVarCollectionModel, EnvVarCollection},
    notebooks::{CloudNotebookModel, NotebookId},
    persistence::ModelEvent,
    server::ids::{ClientId, HashableId, ObjectUid, ServerId, SyncId, ToServerId},
    settings::cloud_preferences::Preference,
    util::sync::Condition,
    workflows::{
        workflow::Workflow,
        workflow_enum::{CloudWorkflowEnumModel, WorkflowEnum},
        CloudWorkflowModel, WorkflowId,
    },
    workspaces::user_profiles::UserProfileWithUID,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    future::{ready, Future, Ready},
    sync::{mpsc::SyncSender, Arc},
};
use warp_graphql::mcp_gallery_template::MCPGalleryTemplate;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

lazy_static! {
    static ref DUPLICATE_OBJECT_NAME_REGEX: Regex =
        Regex::new(r" \((\d+)\)$").expect("regex should not fail to compile");
}

#[derive(Debug, PartialEq)]
pub enum OperationSuccessType {
    Success,
    Failure,
    Rejection,
    Denied(String),
    FeatureNotAvailable,
}

#[derive(Debug, PartialEq)]
pub enum ObjectOperation {
    Create { initiated_by: InitiatedBy },
    Update,
    MoveToFolder,
    MoveToDrive,
    Trash,
    TakeEditAccess,
    Untrash,
    Delete { initiated_by: InitiatedBy },
    EmptyTrash,
}

#[derive(Debug)]
pub struct ObjectOperationResult {
    pub success_type: OperationSuccessType,
    pub operation: ObjectOperation,
    pub client_id: Option<ClientId>,
    pub server_id: Option<ServerId>,
    pub num_objects: Option<i32>,
}

#[derive(Debug)]
pub enum UpdateManagerEvent {
    ObjectOperationComplete { result: ObjectOperationResult },
    CloudPreferencesUpdated { updated: Vec<Preference> },
    MCPGalleryUpdated { templates: Vec<MCPGalleryTemplate> },
    AmbientTaskUpdated { timestamp: DateTime<Utc> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitiatedBy {
    User,
    System,
}

#[derive(Default)]
pub struct InitialLoadResponse {
    pub updated_notebooks: Vec<ServerNotebook>,
    pub deleted_notebooks: Vec<NotebookId>,
    pub updated_workflows: Vec<ServerWorkflow>,
    pub deleted_workflows: Vec<WorkflowId>,
    pub updated_folders: Vec<ServerFolder>,
    pub deleted_folders: Vec<FolderId>,
    pub updated_generic_string_objects:
        HashMap<GenericStringObjectFormat, Vec<Box<dyn ServerObject>>>,
    pub deleted_generic_string_objects: Vec<GenericStringObjectId>,
    pub user_profiles: Vec<UserProfileWithUID>,
    pub action_histories: Vec<ObjectActionHistory>,
    pub mcp_gallery: Vec<MCPGalleryTemplate>,
}

#[derive(Debug)]
pub struct GenericStringObjectInput<T, S>
where
    T: StringModel<
            CloudObjectType = GenericCloudObject<GenericStringObjectId, GenericStringModel<T, S>>,
        > + 'static,
    S: Serializer<T> + 'static,
{
    pub id: ClientId,
    pub model: GenericStringModel<T, S>,
    pub initial_folder_id: Option<SyncId>,
    pub entrypoint: CloudObjectEventEntrypoint,
}

pub struct UpdateManager {
    model_event_sender: Option<SyncSender<ModelEvent>>,
    has_initial_load: Condition,
}

impl UpdateManager {
    pub fn new(
        model_event_sender: Option<SyncSender<ModelEvent>>,
        _ctx: &mut ModelContext<Self>,
    ) -> Self {
        let has_initial_load = Condition::new();
        has_initial_load.set();
        Self {
            model_event_sender,
            has_initial_load,
        }
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(None, ctx)
    }

    #[cfg(any(test, feature = "integration_tests"))]
    pub fn spawned_futures(&self) -> &[warpui::r#async::FutureId] {
        &[]
    }

    fn save_to_db(&self, events: impl IntoIterator<Item = ModelEvent>) {
        if let Some(model_event_sender) = &self.model_event_sender {
            for event in events {
                if let Err(err) = model_event_sender.send(event) {
                    log::error!("Error saving to database: {err:?}");
                }
            }
        }
    }

    fn emit_operation(
        &self,
        ctx: &mut ModelContext<Self>,
        success_type: OperationSuccessType,
        operation: ObjectOperation,
        client_id: Option<ClientId>,
        server_id: Option<ServerId>,
        num_objects: Option<i32>,
    ) {
        ctx.emit(UpdateManagerEvent::ObjectOperationComplete {
            result: ObjectOperationResult {
                success_type,
                operation,
                client_id,
                server_id,
                num_objects,
            },
        });
    }

    fn save_in_memory_object_to_sqlite(&self, cloud_model: &CloudModel, uid: &ObjectUid) {
        if let Some(cloud_object) = cloud_model.get_by_uid(uid) {
            self.save_to_db([cloud_object.upsert_event()]);
        }
    }

    pub fn remove_team_objects(&mut self, left_team_uid: ServerId, ctx: &mut ModelContext<Self>) {
        let cloud_model = CloudModel::handle(ctx);
        let objects_to_remove = cloud_model
            .as_ref(ctx)
            .all_cloud_objects_in_space(
                Space::Team {
                    team_uid: left_team_uid,
                },
                ctx,
            )
            .map(|object| object.cloud_object_type_and_id())
            .collect::<Vec<_>>();

        cloud_model.update(ctx, |cloud_model, ctx| {
            for object in &objects_to_remove {
                cloud_model.delete_object(object.sync_id(), ctx);
            }
        });
        ObjectActions::handle(ctx).update(ctx, |object_actions, ctx| {
            for object in &objects_to_remove {
                object_actions.delete_actions_for_object(&object.uid(), ctx);
            }
        });

        self.save_to_db([ModelEvent::DeleteObjects {
            ids: objects_to_remove
                .into_iter()
                .map(|object| (object.sync_id(), object.object_id_type()))
                .collect(),
        }]);
    }

    pub fn reset_initial_load(&self) {
        self.has_initial_load.set();
    }

    pub fn mock_initial_load(
        &mut self,
        _response: InitialLoadResponse,
        ctx: &mut ModelContext<Self>,
    ) {
        CloudModel::handle(ctx).update(ctx, |_, ctx| {
            ctx.emit(CloudModelEvent::InitialLoadCompleted);
        });
        self.has_initial_load.set();
    }

    pub fn initial_load_complete(&self) -> impl Future<Output = ()> {
        self.has_initial_load.wait()
    }

    pub fn replace_object_with_conflict(&mut self, uid: &ObjectUid, ctx: &mut ModelContext<Self>) {
        let cloud_model_handle = CloudModel::handle(ctx);
        let had_conflicts = cloud_model_handle.update(ctx, |cloud_model, ctx| {
            match cloud_model.get_mut_by_uid(uid) {
                Some(object) if object.has_conflicting_changes() => {
                    object.replace_object_with_conflict();
                    ctx.emit(CloudModelEvent::ObjectUpdated {
                        type_and_id: object.cloud_object_type_and_id(),
                        source: UpdateSource::Local,
                    });
                    true
                }
                _ => false,
            }
        });

        if had_conflicts {
            self.save_in_memory_object_to_sqlite(cloud_model_handle.as_ref(ctx), uid);
        }
    }

    pub fn update_ai_fact(
        &mut self,
        ai_fact: AIFact,
        ai_fact_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(CloudAIFactModel::new(ai_fact), ai_fact_id, revision_ts, ctx);
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn update_templatable_mcp_server(
        &mut self,
        templatable_mcp_server: TemplatableMCPServer,
        templatable_mcp_server_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudTemplatableMCPServerModel::new(templatable_mcp_server),
            templatable_mcp_server_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_workflow(
        &mut self,
        workflow: Workflow,
        workflow_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudWorkflowModel::new(workflow),
            workflow_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_workflow_enum(
        &mut self,
        workflow_enum: WorkflowEnum,
        workflow_enum_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudWorkflowEnumModel::new(workflow_enum),
            workflow_enum_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_env_var_collection(
        &mut self,
        env_var_collection: EnvVarCollection,
        env_var_collection_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudEnvVarCollectionModel::new(env_var_collection),
            env_var_collection_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_ambient_agent_environment(
        &mut self,
        environment: AmbientAgentEnvironment,
        environment_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            AmbientAgentEnvironmentObjectModel::new(environment),
            environment_id,
            revision_ts,
            ctx,
        );
    }

    pub fn update_notebook_data(
        &mut self,
        data: Arc<String>,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&notebook_id).cloned();
        if let Some(notebook) = cloud_model.get_notebook(&notebook_id) {
            self.update_object(
                CloudNotebookModel {
                    title: notebook.model().title.to_owned(),
                    data: data.to_string(),
                    ai_document_id: notebook.model().ai_document_id,
                    conversation_id: notebook.model().conversation_id.clone(),
                },
                notebook_id,
                revision,
                ctx,
            );
        }
    }

    pub fn update_notebook_title(
        &mut self,
        title: Arc<String>,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&notebook_id).cloned();
        if let Some(notebook) = cloud_model.get_notebook(&notebook_id) {
            self.update_object(
                CloudNotebookModel {
                    title: title.to_string(),
                    data: notebook.model().data.to_owned(),
                    ai_document_id: notebook.model().ai_document_id,
                    conversation_id: notebook.model().conversation_id.clone(),
                },
                notebook_id,
                revision,
                ctx,
            );
        }
    }

    pub fn move_object_to_location(
        &mut self,
        object_id: CloudObjectTypeAndId,
        new_location: CloudObjectLocation,
        ctx: &mut ModelContext<Self>,
    ) {
        if let CloudObjectLocation::Trash = new_location {
            return self.trash_object(object_id, ctx);
        }

        let uid = object_id.uid();
        let operation = match new_location {
            CloudObjectLocation::Space(space) => {
                let Some(owner) = crate::workspaces::user_workspaces::UserWorkspaces::as_ref(ctx)
                    .space_to_owner(space, ctx)
                else {
                    return;
                };
                CloudModel::handle(ctx).update(ctx, |model, ctx| {
                    model.update_object_location(&uid, Some(owner), None, ctx);
                });
                ObjectOperation::MoveToDrive
            }
            CloudObjectLocation::Folder(folder_id) => {
                CloudModel::handle(ctx).update(ctx, |model, ctx| {
                    model.update_object_location(&uid, None, Some(folder_id), ctx);
                });
                ObjectOperation::MoveToFolder
            }
            CloudObjectLocation::Trash => unreachable!(),
        };

        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &uid);
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            operation,
            None,
            object_id.server_id(),
            None,
        );
        ctx.notify();
    }

    pub fn duplicate_object(
        &mut self,
        cloud_object_type_and_id: &CloudObjectTypeAndId,
        ctx: &mut ModelContext<Self>,
    ) {
        match cloud_object_type_and_id {
            CloudObjectTypeAndId::Notebook(notebook_id) => {
                self.duplicate_object_internal::<NotebookId, CloudNotebookModel>(notebook_id, ctx);
            }
            CloudObjectTypeAndId::Workflow(workflow_id) => {
                self.duplicate_object_internal::<WorkflowId, CloudWorkflowModel>(workflow_id, ctx);
            }
            CloudObjectTypeAndId::GenericStringObject { object_type, id }
                if matches!(
                    object_type,
                    GenericStringObjectFormat::Json(JsonObjectType::EnvVarCollection)
                ) =>
            {
                self.duplicate_object_internal::<GenericStringObjectId, CloudEnvVarCollectionModel>(
                    id, ctx,
                );
            }
            _ => log::warn!("Tried to duplicate an unsupported local object type"),
        }
    }

    fn duplicate_object_internal<K, M>(&mut self, id: &SyncId, ctx: &mut ModelContext<Self>)
    where
        K: HashableId + ToServerId + Debug + Into<String> + Clone + Copy + Send + Sync + 'static,
        M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
    {
        let Some((duplicate_model, client_id, owner, initial_folder_id, entrypoint)) = ({
            let cloud_model = CloudModel::as_ref(ctx);
            cloud_model.get_object_of_type::<K, M>(id).map(|object| {
                let client_id = ClientId::new();
                let owner = object.permissions.owner;
                let initial_folder_id = object.metadata.folder_id;
                let entrypoint = CloudObjectEventEntrypoint::Unknown;
                let mut duplicate_model = object.model().clone();
                let duplicate_name = self.get_next_duplicate_object_name(
                    object as &dyn CloudObject,
                    cloud_model,
                    ctx,
                );
                duplicate_model.set_display_name(&duplicate_name);
                (
                    duplicate_model,
                    client_id,
                    owner,
                    initial_folder_id,
                    entrypoint,
                )
            })
        }) else {
            return;
        };
        self.create_object(
            duplicate_model,
            owner,
            client_id,
            entrypoint,
            true,
            initial_folder_id,
            InitiatedBy::User,
            ctx,
        );
    }

    pub fn create_ai_fact(
        &mut self,
        ai_fact: AIFact,
        client_id: ClientId,
        owner: Owner,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudAIFactModel::new(ai_fact),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            InitiatedBy::User,
            ctx,
        );
    }

    #[cfg(not(target_family = "wasm"))]
    pub fn create_templatable_mcp_server(
        &mut self,
        templatable_mcp_server: TemplatableMCPServer,
        client_id: ClientId,
        owner: Owner,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudTemplatableMCPServerModel::new(templatable_mcp_server),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            initiated_by,
            ctx,
        );
    }

    pub fn create_ambient_agent_environment(
        &mut self,
        ambient_agent_environment: AmbientAgentEnvironment,
        client_id: ClientId,
        owner: Owner,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            AmbientAgentEnvironmentObjectModel::new(ambient_agent_environment),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            InitiatedBy::User,
            ctx,
        )
    }

    pub fn create_scheduled_ambient_agent_online(
        &mut self,
        _scheduled_ambient_agent: ScheduledAmbientAgent,
        _client_id: ClientId,
        _owner: Owner,
        _ctx: &mut ModelContext<Self>,
    ) -> Ready<Result<ServerId>> {
        ready(Err(anyhow!(
            "hosted scheduled ambient agents are removed in Warper"
        )))
    }

    pub fn update_scheduled_ambient_agent_online(
        &mut self,
        _scheduled_ambient_agent: ScheduledAmbientAgent,
        _scheduled_ambient_agent_id: SyncId,
        _revision_ts: Option<Revision>,
        _ctx: &mut ModelContext<Self>,
    ) -> Ready<Result<()>> {
        ready(Err(anyhow!(
            "hosted scheduled ambient agents are removed in Warper"
        )))
    }

    pub fn create_ai_execution_profile(
        &mut self,
        ai_execution_profile: AIExecutionProfile,
        client_id: ClientId,
        owner: Owner,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudAIExecutionProfileModel::new(ai_execution_profile),
            owner,
            client_id,
            Default::default(),
            false,
            None,
            InitiatedBy::User,
            ctx,
        );
    }

    pub fn update_ai_execution_profile(
        &mut self,
        ai_execution_profile: AIExecutionProfile,
        ai_execution_profile_id: SyncId,
        revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) {
        self.update_object(
            CloudAIExecutionProfileModel::new(ai_execution_profile),
            ai_execution_profile_id,
            revision_ts,
            ctx,
        );
    }

    pub fn delete_ai_execution_profile(
        &mut self,
        ai_execution_profile_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.delete_object_by_user(
            CloudObjectTypeAndId::GenericStringObject {
                object_type: GenericStringObjectFormat::Json(JsonObjectType::AIExecutionProfile),
                id: ai_execution_profile_id,
            },
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_notebook(
        &mut self,
        client_id: ClientId,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        model: CloudNotebookModel,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            model,
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_workflow(
        &mut self,
        workflow: Workflow,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        client_id: ClientId,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudWorkflowModel::new(workflow),
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            InitiatedBy::User,
            ctx,
        );
    }

    pub fn create_workflow_enum(
        &mut self,
        workflow_enum: WorkflowEnum,
        owner: Owner,
        client_id: ClientId,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudWorkflowEnumModel::new(workflow_enum),
            owner,
            client_id,
            entrypoint,
            force_expand,
            None,
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_env_var_collection(
        &mut self,
        client_id: ClientId,
        owner: Owner,
        initial_folder_id: Option<SyncId>,
        model: CloudEnvVarCollectionModel,
        entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            model,
            owner,
            client_id,
            entrypoint,
            force_expand,
            initial_folder_id,
            InitiatedBy::User,
            ctx,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_folder(
        &mut self,
        name: String,
        owner: Owner,
        client_id: ClientId,
        initial_folder_id: Option<SyncId>,
        force_expand: bool,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        self.create_object(
            CloudFolderModel::new(&name, false),
            owner,
            client_id,
            Default::default(),
            force_expand,
            initial_folder_id,
            initiated_by,
            ctx,
        );
    }

    pub fn bulk_create_generic_string_objects<S, T>(
        &mut self,
        owner: Owner,
        inputs: Vec<GenericStringObjectInput<T, S>>,
        ctx: &mut ModelContext<Self>,
    ) where
        T: StringModel<
                CloudObjectType = GenericCloudObject<
                    GenericStringObjectId,
                    GenericStringModel<T, S>,
                >,
            > + 'static,
        S: Serializer<T> + 'static,
    {
        let mut objects = Vec::new();
        for input in inputs {
            let object_id = SyncId::ClientId(input.id);
            CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
                let object =
                    GenericCloudObject::<GenericStringObjectId, GenericStringModel<T, S>>::new_local(
                        input.model,
                        owner,
                        input.initial_folder_id,
                        input.id,
                    );
                cloud_model.create_object(object_id, object, ctx);
            });
            if let Some(object) = CloudModel::as_ref(ctx)
                .get_object_of_type::<GenericStringObjectId, GenericStringModel<T, S>>(&object_id)
            {
                objects.push(object.clone());
            }
        }

        self.save_to_db([GenericStringModel::<T, S>::bulk_upsert_event(&objects)]);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_object<K, M>(
        &mut self,
        model: M,
        owner: Owner,
        client_id: ClientId,
        _entrypoint: CloudObjectEventEntrypoint,
        force_expand: bool,
        initial_folder_id: Option<SyncId>,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) where
        K: HashableId + ToServerId + Debug + Into<String> + Clone + Copy + Send + Sync + 'static,
        M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
    {
        let object_id = SyncId::ClientId(client_id);
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            let object = GenericCloudObject::<K, M>::new_local(
                model.clone(),
                owner,
                initial_folder_id,
                client_id,
            );
            cloud_model.create_object(object_id, object, ctx);

            if force_expand {
                cloud_model.force_expand_object_and_ancestors(object_id, ctx);
            }
        });

        if let Some(object) = CloudModel::as_ref(ctx).get_object_of_type::<K, M>(&object_id) {
            self.save_to_db([object.upsert_event()]);
        }
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::Create { initiated_by },
            Some(client_id),
            None,
            None,
        );
    }

    pub fn update_object<K, M>(
        &mut self,
        model: M,
        object_id: SyncId,
        _revision_ts: Option<Revision>,
        ctx: &mut ModelContext<Self>,
    ) where
        K: HashableId + ToServerId + Debug + Into<String> + Clone + Copy + Send + Sync + 'static,
        M: CloudModelType<IdType = K, CloudObjectType = GenericCloudObject<K, M>> + 'static,
    {
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            cloud_model.update_object_from_edit(model.clone(), object_id, ctx);
        });

        if let Some(object) = CloudModel::as_ref(ctx).get_object_of_type::<K, M>(&object_id) {
            self.save_to_db([object.upsert_event()]);
        };
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::Update,
            object_id.into_client(),
            object_id.into_server(),
            None,
        );
    }

    pub fn record_object_action(
        &mut self,
        id_and_type: CloudObjectTypeAndId,
        action_type: ObjectActionType,
        data: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        let action_timestamp = Utc::now();
        let object_action = ObjectActions::handle(ctx).update(ctx, |object_actions_model, ctx| {
            object_actions_model.insert_action(
                id_and_type.uid(),
                id_and_type.sqlite_uid_hash(),
                action_type,
                data,
                action_timestamp,
                ctx,
            )
        });
        self.save_to_db([ModelEvent::InsertObjectAction { object_action }]);
    }

    fn set_notebook_current_editor(
        &self,
        notebook_id: &SyncId,
        editor_uid: Option<String>,
        ctx: &mut ModelContext<Self>,
    ) {
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(notebook) = cloud_model.get_notebook_mut(notebook_id) {
                notebook.metadata.set_current_editor(editor_uid);
                ctx.notify();
            }
        });
    }

    pub fn grab_notebook_edit_access(
        &mut self,
        notebook_id: SyncId,
        _optimistically_grant_access: bool,
        ctx: &mut ModelContext<Self>,
    ) {
        let user_uid = crate::auth::AuthStateProvider::as_ref(ctx)
            .get()
            .user_id()
            .map(|uid| uid.as_string());
        self.set_notebook_current_editor(&notebook_id, user_uid, ctx);
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &notebook_id.uid());
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::TakeEditAccess,
            None,
            notebook_id.into_server(),
            None,
        );
    }

    pub fn give_up_notebook_edit_access(
        &mut self,
        notebook_id: SyncId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.set_notebook_current_editor(&notebook_id, None, ctx);
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &notebook_id.uid());
    }

    pub fn trash_object(&mut self, id: CloudObjectTypeAndId, ctx: &mut ModelContext<Self>) {
        let uid = id.uid();
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(object) = cloud_model.get_mut_by_uid(&uid) {
                object.metadata_mut().trashed_ts =
                    Some(warp_graphql::scalars::time::ServerTimestamp::new(Utc::now()));
                object
                    .metadata_mut()
                    .pending_changes_statuses
                    .has_pending_metadata_change = false;
                ctx.emit(CloudModelEvent::ObjectTrashed {
                    type_and_id: object.cloud_object_type_and_id(),
                    source: UpdateSource::Local,
                });
                ctx.notify();
            }
        });
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &uid);
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::Trash,
            None,
            id.server_id(),
            None,
        );
    }

    pub fn untrash_object(&mut self, id: CloudObjectTypeAndId, ctx: &mut ModelContext<Self>) {
        let uid = id.uid();
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            if let Some(object) = cloud_model.get_mut_by_uid(&uid) {
                object.metadata_mut().trashed_ts = None;
                object
                    .metadata_mut()
                    .pending_changes_statuses
                    .pending_untrash = false;
                ctx.emit(CloudModelEvent::ObjectUntrashed {
                    type_and_id: object.cloud_object_type_and_id(),
                    source: UpdateSource::Local,
                });
                ctx.notify();
            }
        });
        self.save_in_memory_object_to_sqlite(CloudModel::as_ref(ctx), &uid);
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::Untrash,
            None,
            id.server_id(),
            None,
        );
    }

    pub fn delete_object_by_user(
        &mut self,
        id: CloudObjectTypeAndId,
        ctx: &mut ModelContext<Self>,
    ) {
        self.delete_object_with_initiated_by(id, InitiatedBy::User, ctx);
    }

    pub fn delete_object_with_initiated_by(
        &mut self,
        id: CloudObjectTypeAndId,
        initiated_by: InitiatedBy,
        ctx: &mut ModelContext<Self>,
    ) {
        let num_deleted = self.on_object_delete_success(vec![id.sync_id()], ctx);
        self.emit_operation(
            ctx,
            OperationSuccessType::Success,
            ObjectOperation::Delete { initiated_by },
            id.sync_id().into_client(),
            id.server_id(),
            Some(num_deleted),
        );
    }

    pub fn empty_trash(&mut self, space: Space, ctx: &mut ModelContext<Self>) {
        let trashed_ids = CloudModel::as_ref(ctx)
            .trashed_cloud_objects_in_space(space, ctx)
            .map(|object| object.sync_id())
            .collect::<Vec<_>>();
        let num_deleted = self.on_object_delete_success(trashed_ids, ctx);
        self.emit_operation(
            ctx,
            if num_deleted == 0 {
                OperationSuccessType::Rejection
            } else {
                OperationSuccessType::Success
            },
            ObjectOperation::EmptyTrash,
            None,
            None,
            Some(num_deleted),
        );
    }

    pub fn on_object_delete_success(
        &mut self,
        deleted_ids: Vec<SyncId>,
        ctx: &mut ModelContext<'_, UpdateManager>,
    ) -> i32 {
        let all_object_uids: Vec<ObjectUid> = deleted_ids.iter().map(|&id| id.uid()).collect();
        let mut num_deleted_objects = 0;
        let mut sync_ids_and_types: Vec<(SyncId, ObjectIdType)> = Vec::new();
        CloudModel::handle(ctx).update(ctx, |cloud_model, ctx| {
            (sync_ids_and_types, num_deleted_objects) =
                cloud_model.delete_objects_by_id(all_object_uids.clone(), ctx);
        });

        ObjectActions::handle(ctx).update(ctx, |object_actions, ctx| {
            for uid in all_object_uids {
                object_actions.delete_actions_for_object(&uid, ctx);
            }
        });

        if num_deleted_objects > 0 {
            self.save_to_db([ModelEvent::DeleteObjects {
                ids: sync_ids_and_types,
            }]);
        }

        num_deleted_objects
    }

    pub fn rename_folder(
        &mut self,
        folder_id: SyncId,
        new_name: String,
        ctx: &mut ModelContext<Self>,
    ) {
        let cloud_model = CloudModel::as_ref(ctx);
        let revision = cloud_model.current_revision(&folder_id).cloned();
        if let Some(folder) = cloud_model.get_folder(&folder_id) {
            self.update_object(
                CloudFolderModel {
                    name: new_name,
                    is_open: folder.model().is_open,
                    is_warp_pack: folder.model().is_warp_pack,
                },
                folder_id,
                revision,
                ctx,
            );
        }
    }

    fn get_next_duplicate_object_name(
        &self,
        original_cloud_object: &dyn CloudObject,
        cloud_model: &CloudModel,
        app: &AppContext,
    ) -> String {
        let original_name = original_cloud_object.display_name();
        let same_type_and_folder_names = cloud_model
            .active_cloud_objects_in_location_without_descendents(
                original_cloud_object.location(cloud_model, app),
                app,
            )
            .filter(|&object| object.object_type() == original_cloud_object.object_type())
            .map(|object| object.display_name())
            .collect::<HashSet<String>>();

        let mut duplicate_name = get_duplicate_object_name(&original_name);
        while same_type_and_folder_names.contains(&duplicate_name) {
            duplicate_name = get_duplicate_object_name(&duplicate_name);
        }
        duplicate_name
    }

    fn sync_actions_for_objects_to_sqlite(
        &mut self,
        object_uids: Vec<&ObjectUid>,
        ctx: &mut ModelContext<Self>,
    ) {
        let actions = ObjectActions::handle(ctx).read(ctx, |object_actions_model, _ctx| {
            object_actions_model.get_actions_for_objects(object_uids)
        });
        let actions_to_sync: Vec<ObjectAction> = actions.values().flatten().cloned().collect();
        self.save_to_db([ModelEvent::SyncObjectActions { actions_to_sync }]);
    }
}

pub fn get_duplicate_object_name(original_name: &str) -> String {
    match DUPLICATE_OBJECT_NAME_REGEX
        .captures(original_name)
        .and_then(|caps| caps.get(1))
        .and_then(|num| num.as_str().parse::<usize>().ok())
    {
        Some(num) => {
            let new_num = num.saturating_add(1);

            if new_num == usize::MAX {
                format!("{original_name} (1)")
            } else {
                DUPLICATE_OBJECT_NAME_REGEX
                    .replace(original_name, format!(" ({new_num})"))
                    .to_string()
            }
        }
        None => format!("{original_name} (1)"),
    }
}

impl Entity for UpdateManager {
    type Event = UpdateManagerEvent;
}

impl SingletonEntity for UpdateManager {}
