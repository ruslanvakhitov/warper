mod context;
pub mod context_provider;
mod events;
mod macros;

pub use context::telemetry_context;
pub use events::*;

/// Removes all telemetry events from the app telemetry event queue.
pub fn clear_event_queue() {
    let _ = warpui::telemetry::flush_events();
}
