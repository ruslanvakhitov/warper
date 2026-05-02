use warpui::{Entity, ModelContext, SingletonEntity};

#[derive(Clone)]
pub struct TeamTesterStatus {}

impl TeamTesterStatus {
    #[cfg(test)]
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }

    #[cfg(test)]
    pub fn mock(ctx: &mut ModelContext<Self>) -> Self {
        Self::new(ctx)
    }

    /// Keep the legacy signal inert for call sites compiled out of the startup path.
    pub fn initiate_data_pollers(&mut self, force_refresh: bool, ctx: &mut ModelContext<Self>) {
        let _ = force_refresh;
        ctx.emit(TeamTesterStatusEvent::InitiateDataPollers)
    }
}

pub enum TeamTesterStatusEvent {
    InitiateDataPollers,
}

impl Entity for TeamTesterStatus {
    type Event = TeamTesterStatusEvent;
}

impl SingletonEntity for TeamTesterStatus {}
