use serde::{Deserialize, Serialize};
use warp_core::context_flag::ContextFlag;
use warpui::AppContext;

use workflow::Workflow;

pub mod aliases;
pub mod categories;
pub mod command_parser;
pub mod info_box;
pub mod local_workflows;
pub mod workflow;
pub mod workflow_enum;

use crate::notebooks::{NotebookId, NotebookLocation};
use crate::server::ids::ServerId;

pub use categories::{CategoriesView, CategoriesViewEvent};

pub fn init(_app: &mut AppContext) {}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum WorkflowSource {
    Global,
    Local,
    Project,
    Team {
        team_uid: ServerId,
    },
    PersonalCloud,
    AI,
    Notebook {
        notebook_id: Option<NotebookId>,
        team_uid: Option<ServerId>,
        location: NotebookLocation,
    },

    /// A hardcoded workflow type that allows Warp to surface features as Workflows (e.g.
    /// a command to see our network log)
    App,
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Hash, PartialOrd)]
pub enum WorkflowSelectionSource {
    WarpDrive,
    CommandPalette,
    UniversalSearch,
    Voltron,
    AI,
    Notebook,
    SlashMenu,
    UpArrowHistory,
    WorkflowView,
    AgentMode,
    Undefined,
    Alias,
}

#[derive(Debug, Clone, Copy)]
pub enum WorkflowViewMode {
    View,
    Edit,
    Create,
}

impl WorkflowViewMode {
    /// The editing mode supported for a workflow.
    ///
    /// Editing is disabled if the user does not have edit permissions.
    pub fn supported_edit_mode(
        _workflow_id: Option<crate::server::ids::SyncId>,
        _app: &AppContext,
    ) -> Self {
        Self::Edit
    }

    /// The viewing mode supported for this workflow.
    ///
    /// Viewing is disabled if the user is allowed to edit the workflow and in a context where
    /// running workflows is supported.
    pub fn supported_view_mode(
        _workflow_id: Option<crate::server::ids::SyncId>,
        _app: &AppContext,
    ) -> Self {
        if ContextFlag::RunWorkflow.is_enabled() {
            Self::Edit
        } else {
            Self::View
        }
    }

    fn is_editable(&self) -> bool {
        match self {
            Self::View => false,
            Self::Edit | Self::Create => true,
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct WorkflowId(ServerId);
crate::server_id_traits! { WorkflowId, "Workflow" }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AIWorkflowOrigin {
    CommandSearch,
    AgentMode,
}

/// Wrapper type for a workflow from local files, local object storage, generated AI output, or a notebook.
#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowType {
    /// Saved workflows sourced from local, global, project, app collections, saved locally.
    Local(Workflow),
    /// Ephemeral/transient workflows created from AI output.
    AIGenerated {
        workflow: Workflow,
        origin: AIWorkflowOrigin,
    },
    /// A workflow that's part of a cloud notebook.
    Notebook(Workflow),
}

impl WorkflowType {
    pub fn as_workflow(&self) -> &Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated { workflow, .. } => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    /// Returns the contained [`Workflow`], consuming `self`.
    pub fn take_workflow(self) -> Workflow {
        match self {
            WorkflowType::Local(workflow) => workflow,
            WorkflowType::AIGenerated { workflow, .. } => workflow,
            WorkflowType::Notebook(workflow) => workflow,
        }
    }

    pub fn sync_id(&self) -> Option<crate::server::ids::SyncId> {
        None
    }

    pub fn server_id(&self) -> Option<WorkflowId> {
        None
    }

    /// We don't show env var selection for Agent Mode suggested commands.
    pub(super) fn should_show_env_var_selection(&self) -> bool {
        !matches!(self, WorkflowType::AIGenerated { .. },)
    }
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod tests;
