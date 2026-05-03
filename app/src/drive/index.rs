use crate::{
    cloud_object::{CloudObjectLocation, Space},
    server::ids::SyncId,
    workflows::WorkflowViewMode,
};

use super::CloudObjectTypeAndId;

pub const AUTOSCROLL_DETECTION_DISTANCE: f32 = 30.0;
pub const AUTOSCROLL_SPEED_MULTIPLIER: f32 = 10.0;
pub const DRIVE_INDEX_VIEW_POSITION_ID: &str = "drive_index_view_id";
pub const FOLDER_DEPTH_INDENT: f32 = 16.0;
pub const INDEX_CONTENT_MARGIN_LEFT: f32 = 12.0;
pub const ITEM_FONT_SIZE: f32 = 14.0;
pub const ITEM_MARGIN_BOTTOM: f32 = 2.0;
pub const ITEM_PADDING_HORIZONTAL: f32 = 8.0;
pub const ITEM_PADDING_VERTICAL: f32 = 4.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriveIndexVariant {
    MainIndex,
    Trash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DriveIndexSection {
    Space(Space),
    Trash,
}

pub fn warp_drive_section_header_position_id(section: &DriveIndexSection) -> String {
    match section {
        DriveIndexSection::Space(space) => format!("WarpDriveSection_{space:?}"),
        DriveIndexSection::Trash => "WarpDriveSection_Trash".to_string(),
    }
}

#[derive(Clone, Debug)]
pub enum DriveIndexAction {
    OpenObject(CloudObjectTypeAndId),
    RunObject(CloudObjectTypeAndId),
    OpenWorkflowInPane {
        cloud_object_type_and_id: CloudObjectTypeAndId,
        open_mode: WorkflowViewMode,
    },
    ToggleFolderOpen(SyncId),
    OpenAIFactCollection,
    OpenMCPServerCollection,
    ToggleItemOverflowMenu {
        space: Space,
        warp_drive_item_id: super::items::WarpDriveItemId,
    },
    DropIndexItem {
        cloud_object_type_and_id: CloudObjectTypeAndId,
        drop_target_location: CloudObjectLocation,
    },
    UpdateCurrentDropTarget {
        drop_target_location: CloudObjectLocation,
    },
    ClearDropTarget,
    Autoscroll {
        delta: f32,
    },
}
