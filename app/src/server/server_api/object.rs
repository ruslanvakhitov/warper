use crate::{
    cloud_object::{
        BulkCreateCloudObjectResult, BulkCreateGenericStringObjectsRequest,
        CreateCloudObjectResult, CreateObjectRequest, GenericStringObjectFormat,
        GenericStringObjectUniqueKey, Owner, Revision, ServerFolder, ServerNotebook, ServerObject,
        ServerWorkflow, UpdateCloudObjectResult,
    },
    drive::folders::FolderId,
    notebooks::NotebookId,
    server::{ids::ServerId, sync_queue::SerializedModel},
    workflows::WorkflowId,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[cfg(test)]
use mockall::{automock, predicate::*};

use crate::cloud_object::model::{
    actions::{ObjectActionHistory, ObjectActionType},
    generic_string_model::GenericStringObjectId,
};

/// Identifies a guest to remove from an object.
#[derive(Clone, Debug)]
pub enum GuestIdentifier {
    /// Remove a user guest by their email address.
    Email(String),
    /// Remove a team guest by their team UID.
    TeamUid(ServerId),
}

fn hosted_object_api_removed() -> anyhow::Error {
    anyhow!("hosted Warp Drive/cloud object APIs are removed in Warper")
}

#[cfg_attr(test, automock)]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait ObjectClient: 'static + Send + Sync {
    async fn create_workflow(
        &self,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        Err(hosted_object_api_removed())
    }

    async fn update_workflow(
        &self,
        _workflow_id: WorkflowId,
        _data: SerializedModel,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<ServerWorkflow>> {
        Err(hosted_object_api_removed())
    }

    async fn bulk_create_generic_string_objects(
        &self,
        _owner: Owner,
        _objects: &[BulkCreateGenericStringObjectsRequest],
    ) -> Result<BulkCreateCloudObjectResult> {
        Err(hosted_object_api_removed())
    }

    async fn create_generic_string_object(
        &self,
        _format: GenericStringObjectFormat,
        _uniqueness_key: Option<GenericStringObjectUniqueKey>,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        Err(hosted_object_api_removed())
    }

    async fn create_notebook(
        &self,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        Err(hosted_object_api_removed())
    }

    async fn update_notebook(
        &self,
        _notebook_id: NotebookId,
        _title: Option<String>,
        _data: Option<SerializedModel>,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<ServerNotebook>> {
        Err(hosted_object_api_removed())
    }

    async fn create_folder(
        &self,
        _request: CreateObjectRequest,
    ) -> Result<CreateCloudObjectResult> {
        Err(hosted_object_api_removed())
    }

    async fn update_folder(
        &self,
        _folder_id: FolderId,
        _name: SerializedModel,
    ) -> Result<UpdateCloudObjectResult<ServerFolder>> {
        Err(hosted_object_api_removed())
    }

    async fn update_generic_string_object(
        &self,
        _object_id: GenericStringObjectId,
        _model: SerializedModel,
        _revision: Option<Revision>,
    ) -> Result<UpdateCloudObjectResult<Box<dyn ServerObject>>> {
        Err(hosted_object_api_removed())
    }

    async fn record_object_action(
        &self,
        _id: ServerId,
        _action_type: ObjectActionType,
        _timestamp: DateTime<Utc>,
        _data: Option<String>,
    ) -> Result<ObjectActionHistory> {
        Err(hosted_object_api_removed())
    }
}

pub struct LocalOnlyObjectClient;

#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
impl ObjectClient for LocalOnlyObjectClient {}
