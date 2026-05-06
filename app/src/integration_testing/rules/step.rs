use warpui::{integration::TestStep, windowing::WindowManager, WindowId};

use crate::{
    ai::facts::view::AIFactPage, integration_testing::view_getters::workspace_view,
    warp_server_client::ids::SyncId,
};

/// Open the rule pane saved at `key` in the active tab of the window saved at `window_key`
pub fn open_rule_pane(window_key: impl Into<String>, key: impl Into<String>) -> TestStep {
    let window_key = window_key.into();
    let key = key.into();

    TestStep::new("Open rule pane").with_action(move |app, _, data| {
        let window_id: &WindowId = data.get(&window_key).expect("No saved window ID");
        let fact_id: &SyncId = data.get(&key).expect("No saved rule ID");
        workspace_view(app, *window_id).update(app, |workspace, ctx| {
            // Focus the window first
            WindowManager::as_ref(ctx).show_window_and_focus_app(*window_id);

            // Open the AI facts pane
            let page = AIFactPage::RuleEditor {
                sync_id: Some(*fact_id),
            };
            workspace.open_ai_fact_collection_pane(None, Some(page), ctx);
        })
    })
}
