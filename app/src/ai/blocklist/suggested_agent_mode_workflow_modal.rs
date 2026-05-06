use crate::{
    ai::agent::SuggestedAgentModeWorkflow,
    workflows::{WorkflowSelectionSource, WorkflowSource, WorkflowType},
};
use std::{collections::HashMap, default::Default, sync::Arc};
use warp_server_client::ids::SyncId;
use warpui::{
    elements::Empty, keymap::FixedBinding, AppContext, Element, Entity, TypedActionView, View,
    ViewContext,
};

/// A modal component for displaying and managing suggested agent mode workflows.
/// This component wraps a WorkflowView in a modal dialog with proper styling and
/// event handling.
#[derive(Debug, Clone, Default)]
pub struct SuggestedAgentModeWorkflowModal {
    workflow_and_id: Option<SuggestedAgentModeWorkflowAndId>,
}

#[derive(Debug, Clone)]
pub struct SuggestedAgentModeWorkflowAndId {
    pub workflow: SuggestedAgentModeWorkflow,
    pub sync_id: SyncId,
}

#[derive(Debug, Clone)]
pub enum SuggestedAgentModeWorkflowModalAction {
    /// Triggered when the modal should be cancelled/closed
    Cancel,
}

#[derive(Debug, Clone)]
pub enum SuggestedAgentModeWorkflowModalEvent {
    /// Emitted when the modal should be closed
    Close,
    /// Emitted when a new workflow is successfully created
    WorkflowCreated,
    /// Emitted when the workflow should be run
    RunWorkflow {
        workflow: Arc<WorkflowType>,
        source: Box<WorkflowSource>,
        argument_override: Option<HashMap<String, String>>,
        workflow_selection_source: WorkflowSelectionSource,
    },
}

pub fn init(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    app.register_fixed_bindings([FixedBinding::new(
        "escape",
        SuggestedAgentModeWorkflowModalAction::Cancel,
        id!("SuggestedAgentModeWorkflowModal"),
    )]);
}

impl SuggestedAgentModeWorkflowModal {
    pub fn close(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.emit(SuggestedAgentModeWorkflowModalEvent::Close);
    }

    pub fn open_workflow(
        &mut self,
        workflow_and_id: &SuggestedAgentModeWorkflowAndId,
        ctx: &mut ViewContext<Self>,
    ) {
        self.workflow_and_id = Some(workflow_and_id.clone());
        self.close(ctx);
    }
}

impl Entity for SuggestedAgentModeWorkflowModal {
    type Event = SuggestedAgentModeWorkflowModalEvent;
}

impl View for SuggestedAgentModeWorkflowModal {
    fn ui_name() -> &'static str {
        "SuggestedAgentModeWorkflowModal"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl TypedActionView for SuggestedAgentModeWorkflowModal {
    type Action = SuggestedAgentModeWorkflowModalAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            SuggestedAgentModeWorkflowModalAction::Cancel => self.close(ctx),
        }
    }
}
