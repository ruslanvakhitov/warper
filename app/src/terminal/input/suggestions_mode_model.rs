use warpui::{Entity, ModelContext};

use super::{DynamicEnumSuggestionStatus, InputSuggestionsMode};

/// State for terminal history, completions, and workflow parameter suggestions.
pub struct InputSuggestionsModeModel {
    mode: InputSuggestionsMode,
}

impl Default for InputSuggestionsModeModel {
    fn default() -> Self {
        Self::new()
    }
}

impl InputSuggestionsModeModel {
    pub fn new() -> Self {
        Self {
            mode: InputSuggestionsMode::Closed,
        }
    }

    pub fn mode(&self) -> &InputSuggestionsMode {
        &self.mode
    }

    pub fn set_mode(&mut self, mode: InputSuggestionsMode, ctx: &mut ModelContext<Self>) {
        if self.mode != mode {
            self.mode = mode;
            ctx.emit(());
        }
    }

    pub fn set_dynamic_enum_status(
        &mut self,
        status: DynamicEnumSuggestionStatus,
        ctx: &mut ModelContext<Self>,
    ) {
        if let InputSuggestionsMode::DynamicWorkflowEnumSuggestions {
            dynamic_enum_status,
            ..
        } = &mut self.mode
        {
            *dynamic_enum_status = status;
            ctx.emit(());
        }
    }

    pub fn is_visible(&self) -> bool {
        self.mode.is_visible()
    }
    pub fn is_closed(&self) -> bool {
        matches!(self.mode, InputSuggestionsMode::Closed)
    }
    pub fn is_history_up(&self) -> bool {
        matches!(self.mode, InputSuggestionsMode::HistoryUp { .. })
    }
    pub fn is_completion_suggestions(&self) -> bool {
        matches!(
            self.mode,
            InputSuggestionsMode::CompletionSuggestions { .. }
        )
    }
    pub fn is_static_workflow_enum_suggestions(&self) -> bool {
        matches!(
            self.mode,
            InputSuggestionsMode::StaticWorkflowEnumSuggestions { .. }
        )
    }
    pub fn is_dynamic_workflow_enum_suggestions(&self) -> bool {
        matches!(
            self.mode,
            InputSuggestionsMode::DynamicWorkflowEnumSuggestions { .. }
        )
    }
}

impl Entity for InputSuggestionsModeModel {
    type Event = ();
}
