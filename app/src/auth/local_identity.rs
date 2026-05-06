use uuid::Uuid;
use warpui::{AppContext, Entity, SingletonEntity};

/// Local process identity used for app-focus bookkeeping.
pub struct LocalActorIdentity {
    local_instance_id: Uuid,
}

impl LocalActorIdentity {
    pub fn initialize(_ctx: &AppContext) -> Self {
        Self {
            local_instance_id: Uuid::new_v4(),
        }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            local_instance_id: Uuid::new_v4(),
        }
    }

    pub fn local_instance_id(&self) -> String {
        self.local_instance_id.to_string()
    }
}

pub struct LocalActorIdentityProvider {
    identity: LocalActorIdentity,
}

impl LocalActorIdentityProvider {
    pub fn new(identity: LocalActorIdentity) -> Self {
        Self { identity }
    }

    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            identity: LocalActorIdentity::new_for_test(),
        }
    }

    pub fn get(&self) -> &LocalActorIdentity {
        &self.identity
    }
}

impl Entity for LocalActorIdentityProvider {
    type Event = ();
}

impl SingletonEntity for LocalActorIdentityProvider {}
