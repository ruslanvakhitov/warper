use warpui::{async_assert_eq, integration::AssertionWithDataCallback};

use crate::{
    ai::facts::view::AIFactPage, integration_testing::view_getters::workspace_view,
    warp_server_client::ids::SyncId,
};

pub fn assert_rule_pane_open(key: impl Into<String>) -> AssertionWithDataCallback {
    let key = key.into();
    Box::new(move |app, window_id, data| {
        workspace_view(app, window_id).read(app, |workspace, _ctx| {
            let sync_id: &SyncId = data.get(&key).expect("No saved AI fact ID");
            workspace.ai_fact_view().read(app, |ai_fact_view, _ctx| {
                let current_page = ai_fact_view.current_page();
                async_assert_eq!(
                    current_page,
                    AIFactPage::RuleEditor {
                        sync_id: Some(*sync_id)
                    },
                    "Rule pane should be open"
                )
            })
        })
    })
}
