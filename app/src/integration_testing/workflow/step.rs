use warpui::{integration::TestStep, windowing::WindowManager, WindowId};

use crate::{
    drive::LocalObjectOpenSettings,
    integration_testing::view_getters::workspace_view,
    server::ids::SyncId,
    workflows::{manager::WorkflowOpenSource, WorkflowViewMode},
};

use super::open_workflow_count;

/// Open the workflow saved at `workflow_key` in the active tab of the window saved at `window_key`
pub fn open_workflow(window_key: impl Into<String>, workflow_key: impl Into<String>) -> TestStep {
    let window_key = window_key.into();
    let workflow_key = workflow_key.into();

    let workflow_other_key = workflow_key.clone();
    TestStep::new("Open workflow")
        .with_action(move |app, _, data| {
            let workflow_id: &SyncId = data.get(&workflow_key).expect("No saved workflow ID");
            let window_id: &WindowId = data.get(&window_key).expect("No saved window ID");
            workspace_view(app, *window_id).update(app, |workspace, ctx| {
                // If the workflow isn't open yet, opening it won't focus the window (we only change
                // focus if switching to an already-open window). Since the user wouldn't be able to
                // open a workflow in an unfocused window, switch focus explicitly here.
                WindowManager::as_ref(ctx).show_window_and_focus_app(*window_id);
                workspace.open_workflow_in_pane(
                    &WorkflowOpenSource::Existing(*workflow_id),
                    &LocalObjectOpenSettings::default(),
                    WorkflowViewMode::View,
                    ctx,
                );
            })
        })
        .add_named_assertion_with_data_from_prior_step(
            "Check workflow is open",
            move |app, _, data| {
                let workflow_id: &SyncId =
                    data.get(&workflow_other_key).expect("No workflow ID found");
                async_assert!(open_workflow_count(app, *workflow_id) == 1)
            },
        )
}
