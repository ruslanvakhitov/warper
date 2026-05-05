use std::{fmt, marker::PhantomData};

use serde_json::Value;
use strum::IntoEnumIterator;

// Re-export for macro use.
#[doc(hidden)]
#[cfg(not(target_family = "wasm"))]
pub use inventory::submit;

use crate::{
    channel::{Channel, ChannelState},
    features::FeatureFlag,
};

/// Core trait defining retained telemetry event metadata.
///
/// Warper does not upload, queue, or persist telemetry events. The event surface remains so
/// existing UI code and event documentation can compile while hosted telemetry is amputated.
pub trait TelemetryEvent: RegisteredTelemetryEvent {
    /// Returns the name of the telemetry event.
    ///
    /// The name should be a stable identifier that uniquely identifies this type of event.
    /// It is used for local event documentation and should remain consistent over time.
    ///
    /// Returns a borrowed string to avoid allocations for static event names.
    fn name(&self) -> &'static str;

    /// Returns optional structured data associated with this event.
    ///
    /// The payload allows events to include additional context or metadata beyond
    /// just the event name. This is useful for including dynamic data about the
    /// event occurrence.
    ///
    /// Returns None if the event has no additional data to report.
    fn payload(&self) -> Option<Value>;

    /// Returns a human-readable description of what this event represents.
    ///
    /// The description should clearly explain the event for local documentation.
    fn description(&self) -> &'static str;

    /// Determines if an event is enabled in the current build. This only works when all
    /// feature flags are set appropriately, so this should be used when running
    /// the bundled app.
    fn enablement_state(&self) -> EnablementState;

    /// Returns whether this retained event shape can contain user-generated content (UGC).
    fn contains_ugc(&self) -> bool;

    /// Returns an iterator over the descriptors for all telemetry events of this type.
    fn event_descs() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>>;
}

#[macro_export]
macro_rules! register_telemetry_event {
    ($event:ty) => {
        impl $crate::telemetry::RegisteredTelemetryEvent for $event {}

        #[cfg(not(target_family = "wasm"))]
        $crate::telemetry::submit! {
            $crate::telemetry::TelemetryEventRegistration::<$event>::adapt()
        }
    };
}

/// Marker trait for known telemetry events. We rely on this to print an exhaustive telemetry
/// table in Warp's documentation.
///
/// DO NOT implement this trait directly - use the [`register_telemetry_event!`] macro instead.
pub trait RegisteredTelemetryEvent {}

/// An abstract description of a telemetry event we may emit. Every [`TelemetryEvent`] has a
/// corresponding [`TelemetryEventDesc`].
pub trait TelemetryEventDesc: fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn enablement_state(&self) -> EnablementState;
}

/// A type-erased version of [`TelemetryEventRegistration`]. This is only used by the
/// [`register_telemetry_event!`] macro implementation.
#[doc(hidden)]
pub trait AnyTelemetryEventRegistration: Sync {
    /// Returns an iterator over the descriptors for all telemetry events in this [`TelemetryEvent`] implementation.
    fn events(&self) -> Box<dyn Iterator<Item = Box<dyn TelemetryEventDesc>>>;
}

/// Adapter for statically registering all [`TelemetryEvent`] implementations.
#[doc(hidden)]
pub struct TelemetryEventRegistration<T: TelemetryEvent + 'static> {
    /// Marker that `TelemetryEventRegistration` references `T`, but doesn't own a `T` value.
    /// See https://doc.rust-lang.org/nomicon/phantom-data.html
    _marker: PhantomData<fn(T) -> T>,
}

impl<T: TelemetryEvent + 'static> TelemetryEventRegistration<T> {
    pub const fn adapt() -> &'static dyn AnyTelemetryEventRegistration {
        &Self {
            _marker: PhantomData,
        }
    }
}

impl<T: TelemetryEvent + 'static> AnyTelemetryEventRegistration for TelemetryEventRegistration<T> {
    fn events(&self) -> Box<dyn Iterator<Item = Box<dyn TelemetryEventDesc>>> {
        Box::new(T::event_descs())
    }
}

/// Returns an iterator over all discriminants of `T` as [`TelemetryEventDesc`]s.
///
/// Telemetry events that use [`strum`] may use this to implement [`TelemetryEvent::event_descs`].
pub fn enum_events<T>() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>>
where
    T: strum::IntoDiscriminant,
    T::Discriminant: strum::IntoEnumIterator + TelemetryEventDesc + 'static,
{
    T::Discriminant::iter()
        .map(|discriminant| Box::new(discriminant) as Box<dyn TelemetryEventDesc>)
}

// Collect adapters for all registered telemetry events. Because `inventory::collect!` requires a
// concrete type, we use `&static dyn Trait` to erase the generics.
#[cfg(not(target_family = "wasm"))]
inventory::collect!(&'static dyn AnyTelemetryEventRegistration);

/// Returns all registered telemetry events. This is not available in WASM builds, as it relies on
/// the [`inventory`] crate, which does not fully work in our WASM configuration.
#[cfg(not(target_family = "wasm"))]
pub fn all_events() -> impl Iterator<Item = Box<dyn TelemetryEventDesc>> {
    inventory::iter::<&'static dyn AnyTelemetryEventRegistration>().flat_map(|meta| meta.events())
}

/// Gives information about when a telemetry event is enabled.
#[derive(Debug)]
pub enum EnablementState {
    Always,
    /// The telemetry event is enabled when a particular feature flag is enabled.
    Flag(FeatureFlag),
    /// The event is enabled if the app is running in one of the contained channels.
    ChannelSpecific {
        channels: Vec<Channel>,
    },
}

impl EnablementState {
    pub fn is_enabled(&self) -> bool {
        match self {
            EnablementState::Always => true,
            EnablementState::Flag(flag) => flag.is_enabled(),
            EnablementState::ChannelSpecific { channels } => {
                let app_channel = ChannelState::channel();
                channels.contains(&app_channel)
            }
        }
    }
}
