use warpui::{AppContext, Entity, SingletonEntity};

/// Local process identity used for app-focus bookkeeping.
pub struct LocalActorIdentity;

impl LocalActorIdentity {
    pub fn initialize(_ctx: &AppContext) -> Self {
        Self
    }
}

pub struct LocalActorIdentityProvider;

impl LocalActorIdentityProvider {
    pub fn new(_: LocalActorIdentity) -> Self {
        Self
    }
}

impl Entity for LocalActorIdentityProvider {
    type Event = ();
}

impl SingletonEntity for LocalActorIdentityProvider {}
