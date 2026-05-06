use std::ffi::OsStr;

use byte_unit::Byte;
use chrono::{DateTime, Local};
use serde::Serialize;
use sysinfo::ProcessesToUpdate;
use warpui::{Entity, ModelContext, SingletonEntity};

use crate::{server::event_metadata, system::memory_footprint};

/// The threshold at which we emit a memory usage warning.
const MEMORY_USAGE_WARNING_THRESHOLD: Option<Byte> = byte_unit::Byte::GIGABYTE.multiply(10);

/// The refresh interval for system information, in seconds.
const REFRESH_INTERVAL_S: usize = 5;
/// The refresh interval for system information.
const REFRESH_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(REFRESH_INTERVAL_S as u64);

pub enum SystemInfoEvent {
    /// There is new system info available for consumers to query.
    Refreshed,
    /// The application is using a large quantity of memory.
    MemoryUsageHigh,
}

pub struct SystemInfo {
    /// A structure we can use to efficiently query system information.
    system: sysinfo::System,
    /// Whether or not we've already emitted an event due to high memory usage.
    has_emitted_memory_warning_event: bool,
    /// The long OS version.
    long_os_version: Option<String>,
}

impl SystemInfo {
    /// Creates a new [`SystemInfo`] model and begins periodic fetching of
    /// system information.
    ///
    /// Currently only retrieves and exposes memory usage information for the
    /// current process.
    pub fn new(ctx: &mut ModelContext<Self>) -> Self {
        let mut me = Self {
            system: sysinfo::System::new(),
            has_emitted_memory_warning_event: false,
            long_os_version: sysinfo::System::long_os_version(),
        };

        // Initialize the underlying system info.  This is necessary in order
        // for our first read of CPU stats to be accurate, as they are computed
        // as a delta between the previous refresh and now.
        me.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Self::current_pid()]),
            false, /* refresh_dead_processes */
            Self::refresh_kind(),
        );

        // If we're doing automated heap usage tracking, set up periodic
        // refreshes of the memory usage data.
        Self::schedule_refresh(ctx);

        me
    }

    /// Returns the amount of memory being used by the current process, in
    /// bytes.
    pub fn used_memory(&self) -> Byte {
        self.system
            .process(Self::current_pid())
            .expect("current process should exist")
            .memory()
            .into()
    }

    /// Returns the full memory footprint of the current process, in bytes.
    ///
    /// Unlike [`used_memory`] (RSS), this includes memory that has been
    /// swapped out or compressed by the OS.  On macOS this matches the value
    /// shown by Activity Monitor.
    pub fn memory_footprint(&self) -> Byte {
        memory_footprint::memory_footprint_bytes().into()
    }

    /// Returns the average CPU usage over the refresh interval.
    ///
    /// If one CPU core is utilized at 100%, this will return 1.  It may return
    /// a value >1 on multi-core machines.
    pub fn cpu_usage(&self) -> f32 {
        let total_usage = self
            .system
            .process(Self::current_pid())
            .expect("current process should exist")
            .cpu_usage();
        total_usage / 100.
    }

    pub fn long_os_version(&self) -> Option<&str> {
        self.long_os_version.as_deref()
    }

    fn schedule_refresh(ctx: &mut ModelContext<Self>) {
        ctx.spawn(
            async {
                warpui::r#async::Timer::after(REFRESH_INTERVAL).await;
            },
            |me, _, ctx| {
                me.refresh(ctx);
                Self::schedule_refresh(ctx);
            },
        );
    }

    fn refresh(&mut self, ctx: &mut ModelContext<Self>) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[Self::current_pid()]),
            false, /* refresh_dead_processes */
            Self::refresh_kind(),
        );
        ctx.emit(SystemInfoEvent::Refreshed);

        let rss = self.used_memory();
        let footprint = self.memory_footprint();
        self.check_for_excessive_memory_usage(rss, footprint, ctx);
    }

    /// Checks for excessive memory usage. This may emit a local warning event
    /// and trigger a local heap profile dump if excessive usage is detected.
    ///
    /// The threshold check uses `memory_footprint` (which includes swapped
    /// and compressed pages) so we actually detect high memory situations.
    fn check_for_excessive_memory_usage(
        &mut self,
        rss: Byte,
        memory_footprint: Byte,
        ctx: &mut ModelContext<Self>,
    ) {
        if self.has_emitted_memory_warning_event {
            return;
        }

        // Use footprint (not RSS) for the threshold so we catch memory
        // that has been swapped out or compressed by the OS.
        if memory_footprint
            < MEMORY_USAGE_WARNING_THRESHOLD.expect("Threshold should not overflow u64")
        {
            return;
        }

        // Collect a detailed memory breakdown for diagnostics.
        let memory_breakdown = memory_footprint::memory_breakdown();

        // If we're tracking heap usage and detect excessive memory usage,
        // dump the current heap profiling data locally.
        #[cfg(feature = "heap_usage_tracking")]
        {
            let breakdown_for_heap_profile = memory_breakdown.clone();
            ctx.spawn(
                crate::profiling::dump_jemalloc_heap_profile(breakdown_for_heap_profile),
                |_, _, _| {},
            );
        }

        let total_application_usage_bytes = rss.as_u64();

        ctx.emit(SystemInfoEvent::MemoryUsageHigh);
        self.has_emitted_memory_warning_event = true;
    }

    /// Returns the pid of the current process.
    fn current_pid() -> sysinfo::Pid {
        sysinfo::get_current_pid().expect("Platform should support process IDs")
    }

    /// Returns the [`sysinfo::ProcessRefreshKind`] that should be used when
    /// retrieving information about the current process.
    fn refresh_kind() -> sysinfo::ProcessRefreshKind {
        sysinfo::ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu()
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn refresh_all_processes(&mut self) {
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true, /* remove_dead_processes */
            Self::refresh_kind(),
        );
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn processes_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a sysinfo::Process> {
        self.system.processes_by_name(OsStr::new(name))
    }
}

impl Entity for SystemInfo {
    type Event = SystemInfoEvent;
}

impl SingletonEntity for SystemInfo {}

#[derive(Copy, Clone)]
struct MemoryUsageStats {
    total_application_usage_bytes: usize,
    total_blocks: usize,
    total_lines: usize,

    /// Statistics about blocks that have been seen in the past 5 minutes.
    active_block_stats: BlockMemoryStats,
    /// Statistics about blocks that haven't been seen since [5m, 1h).
    inactive_5m_stats: BlockMemoryStats,
    /// Statistics about blocks that haven't been seen since [1h, 24h).
    inactive_1h_stats: BlockMemoryStats,
    /// Statistics about blocks that haven't been seen since [24h, ..).
    inactive_24h_stats: BlockMemoryStats,
}

impl MemoryUsageStats {
    fn new(total_application_usage: Byte) -> Self {
        Self {
            total_application_usage_bytes: total_application_usage.as_u64() as usize,
            total_blocks: 0,
            total_lines: 0,
            active_block_stats: Default::default(),
            inactive_5m_stats: Default::default(),
            inactive_1h_stats: Default::default(),
            inactive_24h_stats: Default::default(),
        }
    }

    fn add_blocks<'a>(
        &mut self,
        now: DateTime<Local>,
        blocks: impl Iterator<Item = &'a crate::terminal::model::block::Block>,
    ) {
        // We compute block-related memory stats across various intervals.
        // "Activity" refers to how recently the block was painted.
        const DURATION_5M: chrono::Duration = chrono::Duration::minutes(5);
        const DURATION_1H: chrono::Duration = chrono::Duration::hours(1);
        const DURATION_24H: chrono::Duration = chrono::Duration::hours(24);

        for block in blocks {
            let num_lines: usize = block.all_grids_iter().map(|grid| grid.len()).sum();

            self.total_blocks += 1;
            self.total_lines += num_lines;

            let last_painted_at = block
                .last_painted_at()
                .unwrap_or(DateTime::UNIX_EPOCH.into());
            let stats = match now - last_painted_at {
                duration if duration < DURATION_5M => &mut self.active_block_stats,
                duration if duration < DURATION_1H => &mut self.inactive_5m_stats,
                duration if duration < DURATION_24H => &mut self.inactive_1h_stats,
                _ => &mut self.inactive_24h_stats,
            };

            stats.num_blocks += 1;
            stats.num_lines += num_lines;
            stats.estimated_memory_usage_bytes += block.estimated_memory_usage_bytes();
        }
    }
}

impl From<MemoryUsageStats> for event_metadata::MemoryUsageStats {
    fn from(value: MemoryUsageStats) -> Self {
        Self {
            total_application_usage_bytes: value.total_application_usage_bytes,
            total_blocks: value.total_blocks,
            total_lines: value.total_lines,
            active_block_stats: value.active_block_stats.into(),
            inactive_5m_stats: value.inactive_5m_stats.into(),
            inactive_1h_stats: value.inactive_1h_stats.into(),
            inactive_24h_stats: value.inactive_24h_stats.into(),
        }
    }
}

#[derive(Copy, Clone, Default, Serialize, PartialEq)]
struct BlockMemoryStats {
    num_blocks: usize,
    num_lines: usize,
    estimated_memory_usage_bytes: usize,
}

impl std::fmt::Debug for BlockMemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockMemoryStats")
            .field("num_blocks", &self.num_blocks)
            .field("num_lines", &self.num_lines)
            .field(
                "estimated_memory_usage_bytes",
                &byte_unit::Byte::from(self.estimated_memory_usage_bytes)
                    .get_adjusted_unit(byte_unit::Unit::MB),
            )
            .finish()
    }
}

impl From<BlockMemoryStats> for event_metadata::BlockMemoryUsageStats {
    fn from(value: BlockMemoryStats) -> Self {
        Self {
            num_blocks: value.num_blocks,
            num_lines: value.num_lines,
            estimated_memory_usage_bytes: value.estimated_memory_usage_bytes,
        }
    }
}

#[cfg(test)]
#[path = "info_tests.rs"]
mod tests;
