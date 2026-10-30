use serde::{Deserialize, Serialize};

use crate::context_chips::{available_chips, ContextChipKind};
use crate::features::FeatureFlag;
use crate::ui_components::icons::Icon;

/// A configurable item
///
/// This unifies context-chip data displays with interactive control buttons so
/// they can all be arranged through the same drag-and-drop editor.
#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "An item that can appear in the agent toolbar.",
    rename_all = "snake_case"
)]
pub enum AgentToolbarItemKind {
    #[schemars(description = "A prompt context chip.")]
    ContextChip(ContextChipKind),

    // CLI agent only
    FileExplorer,
    RichInput,

    // Renamed from ImageAttach; alias preserves existing user toolbar configs.
    #[serde(alias = "ImageAttach")]
    FileAttach,

    // CLI agent only – opens settings to the Coding Agents section.
    Settings,
}

impl AgentToolbarItemKind {
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::ContextChip(_) => "Context Chip",

            Self::FileAttach => "Attach File",

            Self::FileExplorer => "File Explorer",
            Self::RichInput => "Rich Input",
            Self::Settings => "Settings",
        }
    }

    pub fn icon(&self) -> Option<Icon> {
        match self {
            Self::ContextChip(kind) => kind.udi_icon(),

            Self::FileAttach => Some(Icon::Plus),

            Self::FileExplorer => Some(Icon::FileCopy),
            Self::RichInput => Some(Icon::TextInput),
            Self::Settings => Some(Icon::Settings),
        }
    }

    pub fn is_context_chip(&self) -> bool {
        matches!(self, Self::ContextChip(_))
    }

    pub fn context_chip_kind(&self) -> Option<&ContextChipKind> {
        match self {
            Self::ContextChip(kind) => Some(kind),
            _ => None,
        }
    }

    /// Default left-side items for the CLI agent footer.
    pub fn cli_default_left() -> Vec<Self> {
        let mut items = vec![
            Self::FileAttach,
            Self::ContextChip(ContextChipKind::GitDiffStats),
        ];
        items.push(Self::FileExplorer);
        if FeatureFlag::CLIAgentRichInput.is_enabled() {
            items.push(Self::RichInput);
        }
        items
    }

    /// Default right-side items for the CLI agent footer.
    pub fn cli_default_right() -> Vec<Self> {
        vec![
            Self::ContextChip(ContextChipKind::WorkingDirectory),
            Self::ContextChip(ContextChipKind::ShellGitBranch),
            Self::Settings,
        ]
    }

    /// All items available for the CLI agent footer configurator.
    pub fn all_available_for_cli_input() -> Vec<Self> {
        let mut items: Vec<Self> = available_chips()
            .into_iter()
            .map(Self::ContextChip)
            .collect();
        items.extend([
            Self::FileExplorer,
            Self::RichInput,
            Self::FileAttach,
            Self::Settings,
        ]);
        items
    }

    /// Default layout and available items for the CLI agent toolbar.
    pub fn cli_defaults() -> (Vec<Self>, Vec<Self>, Vec<Self>) {
        (
            Self::cli_default_left(),
            Self::cli_default_right(),
            Self::all_available_for_cli_input(),
        )
    }
}

impl From<ContextChipKind> for AgentToolbarItemKind {
    fn from(kind: ContextChipKind) -> Self {
        Self::ContextChip(kind)
    }
}
