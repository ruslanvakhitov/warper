/// Drains a telemetry event immediately through the local telemetry API instead of leaving it in
/// the event queue.
#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr, $ctx:expr) => {
        #[allow(unused_imports)]
        use warp_core::telemetry::TelemetryEvent as _;
        let event = $event;
        if event.enablement_state().is_enabled() {
            let _ = event;
        }
    };
}

/// Drains a telemetry event immediately. This is the same as [`send_telemetry_sync_from_ctx`],
/// but can be used when the caller only has access to an [`App`] and not a `ViewContext`.
#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {
        #[allow(unused_imports)]
        use warp_core::telemetry::TelemetryEvent as _;
        let event = $event;
        if event.enablement_state().is_enabled() {
            let _ = event;
        }
    };
}

/// Records a telemetry event in the local queue asynchronously. This is the same as the
/// [`send_telemetry_from_ctx`], except can be called any time you have an Arc<Background>.
/// This should only be called when invoking one of the other macros isn't possible; for example,
/// when you are already on a background thread and thus can't access any app context.
#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state: expr, $event:expr, $executor:expr) => {
        #[allow(unused_imports)]
        use warp_core::telemetry::TelemetryEvent as _;
        let event = $event;
        if event.enablement_state().is_enabled() {
            let _ = &$auth_state;
            let _ = &$executor;
            let _ = event;
        }
    };
}
