use warpui::{async_assert_eq, integration::AssertionWithDataCallback};

use crate::{ai::facts::view::AIFactPage, integration_testing::view_getters::workspace_view};

pub fn assert_rule_pane_open(key: impl Into<String>) -> AssertionWithDataCallback {
    let key = key.into();
    Box::new(move |app, window_id, data| {
        workspace_view(app, window_id).read(app, |workspace, _ctx| {
            let _ = &key;
            workspace.ai_fact_view().read(app, |ai_fact_view, _ctx| {
                let current_page = ai_fact_view.current_page();
                async_assert_eq!(current_page, AIFactPage::Rules, "Rule pane should be open")
            })
        })
    })
}
