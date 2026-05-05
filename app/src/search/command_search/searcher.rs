use crate::search::mixer::SearchMixer;
use crate::terminal::history::LinkedWorkflowData;
use crate::workflows::{WorkflowSource, WorkflowType};

pub type CommandSearchMixer = SearchMixer<CommandSearchItemAction>;

#[derive(Clone, Debug)]
pub struct AcceptedHistoryItem {
    pub command: String,

    /// The workflow used to construct the command, if any.
    pub linked_workflow_data: Option<LinkedWorkflowData>,
}

/// Payload for `AcceptWorkflow`: identifies which workflow was selected.
///
/// Local/AI-generated workflows carry owned data since hosted workflow sources
/// were removed by WARPER-001.
#[derive(Clone, Debug)]
pub enum AcceptedWorkflow {
    Local {
        workflow: Box<WorkflowType>,
        source: WorkflowSource,
    },
}

/// The set of events that may be produced by accepting or executing a search
/// result.
#[derive(Clone, Debug)]
pub enum CommandSearchItemAction {
    /// The user accepted a history search item. The contained string is the
    /// command they accepted.
    AcceptHistory(AcceptedHistoryItem),

    /// The user requested the re-execution of a history search item. The
    /// contained string is the command they accepted.
    ExecuteHistory(String),

    /// The user accepted a workflow search item.
    AcceptWorkflow(AcceptedWorkflow),

    /// The user accepted the AI query search item with this query text.
    AcceptAIQuery(String),

    /// The user requested to run the AI query search item with this query text.
    RunAIQuery(String),

    /// The user accepted the search item to open Warp AI.
    OpenWarpAI,

    /// The user accepted the search item to translate the query to a command using Warp AI.
    TranslateUsingWarpAI,
}

#[cfg(test)]
#[path = "searcher_test.rs"]
mod tests;
