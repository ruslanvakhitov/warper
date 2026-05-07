use warpui::{Entity, SingletonEntity};

pub struct SettingsInitializer;

impl Default for SettingsInitializer {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsInitializer {
    pub fn new() -> Self {
        Self
    }
}

impl Entity for SettingsInitializer {
    type Event = ();
}

impl SingletonEntity for SettingsInitializer {}
