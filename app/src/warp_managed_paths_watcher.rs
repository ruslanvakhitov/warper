use notify_debouncer_full::notify::{RecursiveMode, WatchFilter};
use repo_metadata::RepositoryUpdate;
use repo_metadata::TargetFile;
use std::path::{Path, PathBuf};
use std::{fs, sync::Arc, time::Duration};
use warpui::ModelHandle;
use warpui::{Entity, ModelContext, SingletonEntity};
use watcher::{BulkFilesystemWatcher, BulkFilesystemWatcherEvent};

/// Duration between filesystem watch events for the Warp managed paths watcher, in milliseconds.
const WARP_MANAGED_PATHS_WATCHER_DEBOUNCE_MILLI_SECS: u64 = 500;

pub(crate) fn warp_data_dir() -> PathBuf {
    warp_core::paths::data_dir()
}
pub(crate) fn ensure_warp_watch_roots_exist() {
    let data_dir = warp_data_dir();
    if let Err(err) = fs::create_dir_all(&data_dir) {
        log::warn!(
            "Failed to create Warper data directory {}: {err}",
            data_dir.display()
        );
    }

    let config_local_dir = warp_core::paths::config_local_dir();
    if config_local_dir != data_dir {
        if let Err(err) = fs::create_dir_all(&config_local_dir) {
            log::warn!(
                "Failed to create Warper config directory {}: {err}",
                config_local_dir.display()
            );
        }
    }
}

pub(crate) fn repository_update_touches_path(update: &RepositoryUpdate, path: &Path) -> bool {
    repository_update_paths(update).any(|candidate| candidate == path)
}
pub(crate) fn repository_update_touches_prefix(update: &RepositoryUpdate, prefix: &Path) -> bool {
    repository_update_paths(update).any(|candidate| candidate.starts_with(prefix))
}

fn repository_update_paths(update: &RepositoryUpdate) -> impl Iterator<Item = &Path> {
    update
        .added
        .iter()
        .map(|target| target.path.as_path())
        .chain(update.modified.iter().map(|target| target.path.as_path()))
        .chain(update.deleted.iter().map(|target| target.path.as_path()))
        .chain(update.moved.iter().flat_map(|(to_target, from_target)| {
            [to_target.path.as_path(), from_target.path.as_path()]
        }))
}

fn filesystem_event_to_repository_update(event: &BulkFilesystemWatcherEvent) -> RepositoryUpdate {
    RepositoryUpdate {
        added: event
            .added
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        modified: event
            .modified
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        deleted: event
            .deleted
            .iter()
            .cloned()
            .map(|path| TargetFile::new(path, false))
            .collect(),
        moved: event
            .moved
            .iter()
            .map(|(to_path, from_path)| {
                (
                    TargetFile::new(to_path.clone(), false),
                    TargetFile::new(from_path.clone(), false),
                )
            })
            .collect(),
        commit_updated: false,
        index_lock_detected: false,
    }
}
pub(crate) enum WarpManagedPathsWatcherEvent {
    FilesChanged(RepositoryUpdate),
}
pub(crate) struct WarpManagedPathsWatcher {
    _watcher: ModelHandle<BulkFilesystemWatcher>,
}
impl WarpManagedPathsWatcher {
    pub(crate) fn new(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_internal(ctx, true)
    }

    #[cfg(test)]
    pub(crate) fn new_for_testing(ctx: &mut ModelContext<Self>) -> Self {
        Self::new_internal(ctx, false)
    }

    fn new_internal(ctx: &mut ModelContext<Self>, should_register_watcher: bool) -> Self {
        let watcher = if should_register_watcher {
            ctx.add_model(|ctx| {
                BulkFilesystemWatcher::new(
                    Duration::from_millis(WARP_MANAGED_PATHS_WATCHER_DEBOUNCE_MILLI_SECS),
                    ctx,
                )
            })
        } else {
            ctx.add_model(|_| BulkFilesystemWatcher::new_for_test())
        };
        ctx.subscribe_to_model(&watcher, Self::handle_fs_event);

        if should_register_watcher {
            let data_dir = warp_data_dir();
            let config_local_dir = warp_core::paths::config_local_dir();
            let should_register_config_local_dir = config_local_dir != data_dir;
            let worktrees_dir = data_dir.join("worktrees");
            Self::register_path(
                ctx,
                &watcher,
                data_dir.clone(),
                WatchFilter::with_filter(Arc::new(move |path| !path.starts_with(&worktrees_dir))),
                RecursiveMode::Recursive,
                "Warper data directory",
            );
            if should_register_config_local_dir {
                Self::register_path(
                    ctx,
                    &watcher,
                    config_local_dir.clone(),
                    WatchFilter::accept_all(),
                    RecursiveMode::Recursive,
                    "Warper config directory",
                );
            }
        }

        Self { _watcher: watcher }
    }

    fn register_path(
        ctx: &mut ModelContext<Self>,
        watcher: &ModelHandle<BulkFilesystemWatcher>,
        directory_path: PathBuf,
        watch_filter: WatchFilter,
        recursive_mode: RecursiveMode,
        description: &'static str,
    ) {
        let registration_path = directory_path.clone();
        let registration = watcher.update(ctx, |watcher, _ctx| {
            watcher.register_path(&registration_path, watch_filter, recursive_mode)
        });

        ctx.spawn(registration, move |_, result, _ctx| {
            if let Err(err) = result {
                log::warn!(
                    "Failed to start watching {description} {}: {err}",
                    directory_path.display()
                );
            }
        });
    }

    fn handle_fs_event(
        &mut self,
        event: &BulkFilesystemWatcherEvent,
        ctx: &mut ModelContext<Self>,
    ) {
        let update = filesystem_event_to_repository_update(event);
        if !update.is_empty() {
            ctx.emit(WarpManagedPathsWatcherEvent::FilesChanged(update));
        }
    }
}
impl Entity for WarpManagedPathsWatcher {
    type Event = WarpManagedPathsWatcherEvent;
}

impl SingletonEntity for WarpManagedPathsWatcher {}
