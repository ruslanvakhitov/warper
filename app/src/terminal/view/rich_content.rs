use warpui::{prelude::ChildView, Element, EntityId, View, ViewContext, ViewHandle};

use crate::{
    env_vars::env_var_collection_block::EnvVarCollectionBlock,
    terminal::{
        block_list_viewport::ScrollPositionUpdate,
        model::{
            blocks::RichContentItem, rich_content::RichContentType, terminal_model::BlockIndex,
        },
        ssh::{error::SshErrorBlock, install_tmux::SshInstallTmuxBlock, warpify::SshWarpifyBlock},
        warpify::success_block::WarpifySuccessBlock,
        TerminalView,
    },
};

/// Specifies where to insert rich content in the blocklist.
#[derive(Clone, Copy, Debug)]
pub enum RichContentInsertionPosition {
    /// Append to the end of the blocklist. If `insert_below_long_running_block` is true
    /// and there is a long-running block, the content is inserted after that block.
    Append {
        insert_below_long_running_block: bool,
    },
    /// Insert before the block at the given index.
    BeforeBlockIndex(BlockIndex),
    /// Pin to the bottom of the blocklist. The BlockList will automatically
    /// keep this item at the end by reordering it after any subsequent insertions.
    /// Only one item can be pinned at a time.
    PinToBottom,
}

/// Wrapper type to hold rich content views and allow generating typed `ChildView` instances
/// on-demand. The `ChildView`s are then passed to the `BlockListElement` to be used when
/// displaying rich content.
pub struct RichContent {
    view_id: EntityId,
    element_builder: Box<dyn Fn() -> Box<dyn Element>>,

    /// Optional rich content view-specific metadata to be passed to the `BlocklistElement` for
    /// rendering.
    metadata: Option<RichContentMetadata>,
}

impl RichContent {
    /// Create a new `RichContent` using a ViewHandle. The RichContent type will continue to own
    /// the ViewHandle for its lifetime, ensuring that the underlying View remains active.
    ///
    pub fn new<V: View>(handle: ViewHandle<V>) -> Self {
        let view_id = handle.id();
        // By `move`ing the handle into the closure, the closure will own the handle and keep it
        // alive for the duration. This also allows us to generate any number of necessary
        // `ChildView` instances
        let element_builder = Box::new(move || ChildView::new(&handle).finish());

        Self {
            view_id,
            element_builder,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: RichContentMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build a new `ChildView` element for this rich content
    fn element(&self) -> Box<dyn Element> {
        (self.element_builder)()
    }

    pub fn view_id(&self) -> EntityId {
        self.view_id
    }

    /// Returns a reference to the metadata, if present.
    pub fn metadata(&self) -> Option<&RichContentMetadata> {
        self.metadata.as_ref()
    }

    pub fn metadata_mut(&mut self) -> Option<&mut RichContentMetadata> {
        self.metadata.as_mut()
    }

    pub(super) fn to_block_list_element_render_params(
        &self,
    ) -> (EntityId, Box<dyn Element>, Option<RichContentMetadata>) {
        (self.view_id(), self.element(), self.metadata.clone())
    }
}

/// `RichContent` view-specific metadata required for rendering in the `BlocklistElement`.
#[derive(Clone, Debug)]
pub enum RichContentMetadata {
    EnvVarCollectionBlock {
        env_var_collection_block_handle: ViewHandle<EnvVarCollectionBlock>,
    },
    SshWarpifyBlock {
        ssh_warpify_block_handle: ViewHandle<SshWarpifyBlock>,
    },
    SshInstallTmuxBlock {
        ssh_install_tmux_block_handle: ViewHandle<SshInstallTmuxBlock>,
    },
    SshErrorBlock {
        ssh_error_block_handle: ViewHandle<SshErrorBlock>,
    },
    WarpifySuccessBlock {
        bootstrap_success_block_handle: ViewHandle<WarpifySuccessBlock>,
    },

    TerminalViewZeroState,
    PluginInstructionsBlock,
}

impl TerminalView {
    /// Add a rich content `View` to the block list. This view can contain any content
    /// we want to display, however it must be exactly `height_px` tall. It will take up that much
    /// space in the block list and when it is laid out in the scene, it will be passed that height
    /// as a strict constraint to the `Element::layout` method.
    ///
    /// The `position` parameter controls where the content is inserted:
    /// - `Append`: Adds to the end; if `insert_below_long_running_block` is true and there's a
    ///   long-running block, the content is inserted after that block.
    /// - `BeforeBlockIndex`: Inserts before the specified block index.
    pub fn insert_rich_content<V: View>(
        &mut self,
        content_type: Option<RichContentType>,
        handle: ViewHandle<V>,
        metadata: Option<RichContentMetadata>,
        position: RichContentInsertionPosition,
        ctx: &mut ViewContext<Self>,
    ) {
        let item = RichContentItem::new(content_type, handle.id());

        match position {
            RichContentInsertionPosition::Append {
                insert_below_long_running_block,
            } => {
                self.model
                    .lock()
                    .block_list_mut()
                    .append_rich_content(item, insert_below_long_running_block);
            }
            RichContentInsertionPosition::BeforeBlockIndex(block_index) => {
                self.model
                    .lock()
                    .block_list_mut()
                    .insert_rich_content_before_block_index(item, block_index);
            }
            RichContentInsertionPosition::PinToBottom => {
                self.model
                    .lock()
                    .block_list_mut()
                    .append_rich_content_pinned_to_bottom(item);
            }
        }

        let mut rich_content = RichContent::new(handle);
        if let Some(metadata) = metadata {
            rich_content = rich_content.with_metadata(metadata);
        }
        self.rich_content_views.push(rich_content);

        // Scroll to bottom
        self.update_scroll_position_locking(ScrollPositionUpdate::AfterRichBlockInserted, ctx);

        ctx.notify();
    }
}
