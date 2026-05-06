pub mod actions;
mod context_menu;
pub mod editor;
pub mod file;
pub mod link;
mod styles;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use warpui::AppContext;

use crate::server::ids::{ServerId, SyncId};

/// This is the notebook_id in the database associated with this notebook.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct NotebookId(ServerId);
crate::server_id_traits! { NotebookId, "Notebook" }

impl From<NotebookId> for SyncId {
    fn from(id: NotebookId) -> Self {
        Self::ServerId(id.into())
    }
}

/// A notebook location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum NotebookLocation {
    /// A notebook backed by a local file.
    LocalFile,
    /// A notebook backed by a remote file.
    RemoteFile,
}

/// Initialize notebooks-related keybindings.
pub fn init(app: &mut AppContext) {
    self::file::init(app);
    self::editor::view::init(app);
}

/// Post process a notebook's content read from an external system. This cleans up extra
/// whitespace, and, in the future, may filter out unsupported syntax extensions.
///
/// See CLD-944.
pub fn post_process_notebook(data: &str) -> String {
    // TODO(kevin): We should not strip out newlines in the code block.
    data.lines().filter(|line| !line.is_empty()).join("\n")
}

/// Translate a notebook's Markdown content into an external Markdown format.
///
/// This:
/// * Normalizes code block languages
/// * Includes extra context for embedded objects.
#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
pub fn export_notebook(data: &str, ctx: &AppContext) -> anyhow::Result<String> {
    use warp_editor::content::{buffer::Buffer, markdown::MarkdownStyle};

    // Parse the Markdown directly rather than using [`Buffer::from_markdown`] so that we can
    // report errors to the exporter.
    let parsed = markdown_parser::parse_markdown(data)?;
    Ok(Buffer::export_to_markdown(
        parsed,
        Some(editor::notebook_embedded_item_conversion),
        MarkdownStyle::Export {
            app_context: Some(ctx),
            should_not_escape_markdown_punctuation: false,
        },
    ))
}
