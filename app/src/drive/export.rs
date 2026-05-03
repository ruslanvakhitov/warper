use std::path::PathBuf;

use crate::{cloud_object::Space, drive::CloudObjectTypeAndId};
use warpui::{Entity, ModelContext, SingletonEntity, WindowId};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ExportId(pub CloudObjectTypeAndId, pub Space);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportEvent {
    Canceled(ExportId),
    Failed { id: ExportId },
    Completed { id: ExportId, path: PathBuf },
}

pub struct ExportManager;

impl ExportManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self
    }

    pub fn export(
        &mut self,
        _window_id: WindowId,
        _objects: &[CloudObjectTypeAndId],
        _ctx: &mut ModelContext<Self>,
    ) {
        log::debug!("Ignoring removed drive export request after WARPER-001 amputation");
    }
}

impl Entity for ExportManager {
    type Event = ExportEvent;
}

impl SingletonEntity for ExportManager {}

pub fn safe_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed
    }
}
