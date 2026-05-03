use warpui::{Element, Entity, TypedActionView, View, ViewContext};

use crate::{
    ai::document::ai_document_model::AIDocumentId,
    cloud_object::{Owner, Space},
    env_vars::{manager::EnvVarCollectionSource, CloudEnvVarCollection},
    notebooks::manager::NotebookSource,
    server::ids::SyncId,
    workflows::{manager::WorkflowOpenSource, CloudWorkflow, WorkflowViewMode},
};

use super::{items::WarpDriveItemId, CloudObjectTypeAndId, DriveObjectType};

pub const MIN_SIDEBAR_WIDTH: f32 = 250.0;
pub const MAX_SIDEBAR_WIDTH_RATIO: f32 = 0.75;
pub const WARP_DRIVE_POSITION_ID: &str = "warp_drive";

pub struct DrivePanel;

#[derive(Clone, Debug)]
pub enum DrivePanelAction {
    OpenSearch,
    FocusDriveIndex,
}

#[derive(Clone, Debug)]
pub enum DrivePanelEvent {
    RunWorkflow(Box<CloudWorkflow>),
    InvokeEnvironmentVariables {
        env_var_collection: Box<CloudEnvVarCollection>,
        in_subshell: bool,
    },
    OpenSearch,
    OpenAIFactCollection,
    OpenMCPServerCollection,
    OpenImportModal {
        owner: Owner,
        initial_folder_id: Option<SyncId>,
    },
    OpenWorkflowModalWithNew {
        space: Space,
        initial_folder_id: Option<SyncId>,
    },
    OpenWorkflowModalWithCloudWorkflow(SyncId),
    OpenNotebook(NotebookSource),
    OpenEnvVarCollection(EnvVarCollectionSource),
    OpenWorkflowInPane(WorkflowOpenSource, WorkflowViewMode),
    FocusWarpDrive,
    AttachPlanAsContext(AIDocumentId),
}

impl DrivePanel {
    pub fn new(_ctx: &mut ViewContext<Self>) -> Self {
        Self
    }

    pub fn reset_and_open_to_main_index(&mut self, _ctx: &mut ViewContext<Self>) {}

    pub fn set_focused_item(&mut self, _id: WarpDriveItemId, _ctx: &mut ViewContext<Self>) {}

    pub fn move_object_to_team_owner(
        &mut self,
        _cloud_object_type_and_id: CloudObjectTypeAndId,
        _space: Space,
        _ctx: &mut ViewContext<Self>,
    ) {
    }

    pub fn set_focused_index(&mut self, _index: Option<usize>, _ctx: &mut ViewContext<Self>) {}

    pub fn set_selected_object(
        &mut self,
        _id: Option<WarpDriveItemId>,
        _ctx: &mut ViewContext<Self>,
    ) {
    }
}

impl Entity for DrivePanel {
    type Event = DrivePanelEvent;
}

impl TypedActionView for DrivePanel {
    type Action = DrivePanelAction;

    fn handle_action(&mut self, _action: &Self::Action, _ctx: &mut ViewContext<Self>) {}
}

impl View for DrivePanel {
    fn ui_name() -> &'static str {
        "WarpDrivePanel"
    }

    fn render(&self, _app: &warpui::AppContext) -> Box<dyn Element> {
        warpui::elements::Empty::new().finish()
    }
}
