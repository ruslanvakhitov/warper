use warpui::fonts::FamilyId;
use warpui::{AppContext, SingletonEntity};

use crate::appearance::Appearance;
use crate::search::async_snapshot_data_source::AsyncSnapshotDataSource;
use crate::search::data_source::{Query, QueryResult};
use crate::search::mixer::{BoxFuture, DataSourceRunErrorWrapper};
use crate::server::ids::SyncId;

use super::AcceptSlashCommandOrSavedPrompt;

pub(super) struct SavedPromptCandidate {
    pub(super) id: SyncId,
    pub(super) name: String,
    pub(super) breadcrumbs: String,
}

pub(crate) struct SavedPromptsSnapshot {
    candidates: Vec<SavedPromptCandidate>,
    query_text: String,
    font_family: FamilyId,
    ai_enabled: bool,
}

pub(crate) fn saved_prompts_data_source(
) -> AsyncSnapshotDataSource<SavedPromptsSnapshot, AcceptSlashCommandOrSavedPrompt> {
    AsyncSnapshotDataSource::new(
        |query: &Query, app: &AppContext| SavedPromptsSnapshot {
            candidates: Vec::new(),
            query_text: query.text.trim().to_owned(),
            font_family: Appearance::as_ref(app).ui_font_family(),
            ai_enabled: false,
        },
        fuzzy_match_saved_prompts,
    )
}

pub(crate) fn fuzzy_match_saved_prompts(
    snapshot: SavedPromptsSnapshot,
) -> BoxFuture<
    'static,
    Result<Vec<QueryResult<AcceptSlashCommandOrSavedPrompt>>, DataSourceRunErrorWrapper>,
> {
    Box::pin(async move {
        let _ = snapshot;
        Ok(Vec::new())
    })
}
