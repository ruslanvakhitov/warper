#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Because it's hard to identify the contents of rich content blocks,
/// we register unique identifiers to make it easier to identify them.
pub enum RichContentType {
    WarpifySuccessBlock,

    TerminalViewZeroState,
    PluginInstructionsBlock,
}

impl RichContentType {}
