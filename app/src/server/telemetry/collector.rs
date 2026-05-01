use std::sync::Arc;

use warpui::{Entity, ModelContext, SingletonEntity};

use crate::{
    server::server_api::ServerApi,
    settings::{PrivacySettings, PrivacySettingsChangedEvent},
};

use super::clear_event_queue;

/// App singleton responsible for ensuring hosted telemetry is not retained for upload.
pub struct TelemetryCollector {
    _server_api: Arc<ServerApi>,
}

impl TelemetryCollector {
    pub fn new(server_api: Arc<ServerApi>) -> Self {
        Self {
            _server_api: server_api,
        }
    }

    pub fn initialize_telemetry_collection(&self, ctx: &mut ModelContext<TelemetryCollector>) {
        self.remove_legacy_persisted_events();

        // Clear queued telemetry events when telemetry is enabled or disabled. If telemetry is
        // enabled later, Warper still has no hosted telemetry upload path.
        ctx.subscribe_to_model(&PrivacySettings::handle(ctx), |_me, event, _ctx| {
            if let PrivacySettingsChangedEvent::UpdateIsTelemetryEnabled { .. } = event {
                clear_event_queue();
            }
        });
    }

    /// Drains telemetry events when the app is shutting down without sending or persisting them.
    pub fn flush_telemetry_events_for_shutdown(&self, ctx: &mut ModelContext<TelemetryCollector>) {
        let _ = PrivacySettings::as_ref(ctx).get_snapshot(ctx);
        clear_event_queue();
    }

    fn remove_legacy_persisted_events(&self) {
        let legacy_file_name = "rudder_telemetry_events.json";
        let paths = [
            warp_core::paths::secure_state_dir()
                .unwrap_or_else(warp_core::paths::state_dir)
                .join(legacy_file_name),
            warp_core::paths::state_dir().join(legacy_file_name),
        ];

        for path in paths {
            match std::fs::remove_file(&path) {
                Ok(()) => log::info!(
                    "Removed legacy persisted telemetry queue at {}",
                    path.display()
                ),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => log::warn!(
                    "Failed to remove legacy persisted telemetry queue at {}: {err}",
                    path.display()
                ),
            }
        }
    }
}

impl Entity for TelemetryCollector {
    type Event = ();
}

impl SingletonEntity for TelemetryCollector {}
