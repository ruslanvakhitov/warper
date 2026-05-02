mod context;
pub mod context_provider;
mod events;
mod macros;

pub use context::telemetry_context;
pub use events::*;

use crate::auth::UserUid;
use crate::settings::PrivacySettingsSnapshot;
use anyhow::Result;
use futures::FutureExt;
use std::future::Future;
use std::path::Path;

/// Removes all telemetry events from the app telemetry event queue.
pub fn clear_event_queue() {
    let _ = warpui::telemetry::flush_events();
}

pub struct TelemetryApi {
    pub(super) client: http_client::Client,
}

impl Default for TelemetryApi {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryApi {
    pub fn new() -> Self {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                let client = http_client::Client::new_for_test();
            } else if #[cfg(target_family = "wasm")] {
                let client = http_client::Client::default();
            } else {
                use std::time::Duration;

                let client = http_client::Client::from_client_builder(
                    // Keep a local HTTP client available for retained network-log instrumentation.
                    reqwest::Client::builder()
                        // Don't allow insecure connections; they will be rejected by
                        // the server with a 403 Forbidden.
                        .https_only(true)
                        // Keep idle connections in the pool for up to 55s. AWS
                        // Application Load Balancers will drop idle connections after
                        // 60s and the default pool idle timeout is 90s; a pool idle
                        // timeout longer than the server timeout can lead to errors
                        // upon trying to use an idle connection.
                        .pool_idle_timeout(Duration::from_secs(55))
                        .connect_timeout(Duration::from_secs(10)),
                ).expect("Client should be constructed since we use a compatibility layer to use reqwest::Client");
            }
        }

        Self { client }
    }

    // Drains telemetry events from the global queue. Warper intentionally has no hosted telemetry
    // upload path, so flushed events are dropped locally.
    // Returns the number of events that were flushed.
    pub async fn flush_events(&self, _settings_snapshot: PrivacySettingsSnapshot) -> Result<usize> {
        let events = warpui::telemetry::flush_events();
        let event_count = events.len();
        Ok(event_count)
    }

    /// Removes a legacy persisted telemetry queue without uploading it.
    pub async fn remove_persisted_telemetry_events(
        &self,
        path: &Path,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => {
                log::info!(
                    "Removed legacy persisted telemetry queue at {}",
                    path.display()
                );
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        Ok(())
    }

    /// Drains queued telemetry events without persisting them for a future upload.
    pub fn flush_and_persist_events(
        &self,
        _max_event_count: usize,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        clear_event_queue();
        Ok(())
    }

    #[cfg(test)]
    fn flush_and_persist_events_at_path(
        &self,
        _max_event_count: usize,
        _settings_snapshot: PrivacySettingsSnapshot,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        clear_event_queue();
        let path = path.as_ref();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Records no hosted telemetry. The event is accepted and dropped so callers do not attempt
    /// retries or persistence-for-upload.
    pub async fn send_telemetry_event(
        &self,
        user_id: Option<UserUid>,
        anonymous_id: String,
        event: impl warp_core::telemetry::TelemetryEvent,
        settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        let event = warpui::telemetry::create_event(
            user_id.map(|uid| uid.as_string()),
            anonymous_id,
            event.name().into(),
            event.payload(),
            event.contains_ugc(),
            warpui::time::get_current_time(),
        );

        self.send_telemetry_event_internal(event, settings_snapshot)
            .await
    }

    /// Internal implementation for sending telemetry events. This reduces code size, since
    // we:
    // 1. Return a boxed future, so calling `async` functions don't need to inline this one.
    // 2. Don't have to monomorphize for each telemetry event implementation.
    fn send_telemetry_event_internal(
        &self,
        _event: warpui::telemetry::Event,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> impl Future<Output = Result<()>> + '_ {
        let work = async move {
            log::debug!("Dropping telemetry event because hosted telemetry upload is disabled.");
            Ok(())
        };

        // On WASM, the work future is non-Send, because the HTTP request future contains a reference to a JS
        // value (which is fine, since our WASM executor is single-threaded). On all other platforms, we must
        // return a Send future in order to use the background executor.
        cfg_if::cfg_if! {
            if #[cfg(target_family = "wasm")] {
                work.boxed_local()
            } else {
                work.boxed()
            }
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
