//! Utilities for terminal keybindings.
use crate::util::bindings::{custom_tag_to_keystroke, CustomAction};

/// The keybinding label to display for a custom terminal action.
pub fn custom_action_to_display(action: CustomAction) -> Option<String> {
    custom_tag_to_keystroke(action.into()).map(|keystroke| keystroke.displayed())
}
