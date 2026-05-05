/// Type-checks a retained telemetry event expression without sending, queueing, or persisting it.
#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr, $ctx:expr) => {
        if false {
            #[allow(unused_imports)]
            use warp_core::telemetry::TelemetryEvent as _;
            let event = $event;
            let _ = event.enablement_state();
            let _ = &$ctx;
        }
    };
}

/// Type-checks a retained telemetry event expression from callers that only have an [`App`].
#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr, $app_ctx:expr) => {
        if false {
            #[allow(unused_imports)]
            use warp_core::telemetry::TelemetryEvent as _;
            let event = $event;
            let _ = event.enablement_state();
            let _ = &$app_ctx;
        }
    };
}

/// Type-checks a retained telemetry event expression from background-thread callers.
#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state: expr, $event:expr, $executor:expr) => {
        if false {
            #[allow(unused_imports)]
            use warp_core::telemetry::TelemetryEvent as _;
            let event = $event;
            let _ = event.enablement_state();
            let _ = &$auth_state;
            let _ = &$executor;
        }
    };
}
