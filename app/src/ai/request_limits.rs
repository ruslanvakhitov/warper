use chrono::{DateTime, Utc};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

pub struct AIRequestUsageModel {
    next_refresh_time: DateTime<Utc>,
}

impl AIRequestUsageModel {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {
            next_refresh_time: Utc::now() + chrono::Duration::days(30),
        }
    }

    pub fn has_any_ai_remaining(&self, _app: &AppContext) -> bool {
        true
    }

    pub fn next_refresh_time(&self) -> chrono::DateTime<Utc> {
        self.next_refresh_time
    }
}

impl Entity for AIRequestUsageModel {
    type Event = ();
}

impl SingletonEntity for AIRequestUsageModel {}
