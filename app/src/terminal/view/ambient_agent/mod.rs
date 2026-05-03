//! Local-only ambient agent compatibility surface.
//!
//! WARPER-001 amputates hosted Oz/cloud ambient agents. The surrounding terminal
//! and AI input code still carries model handles for layout decisions, so this
//! module keeps inert local types while deleting the hosted entrypoints and
//! server/session polling implementation.

mod model;

pub use model::{AmbientAgentViewModel, AmbientAgentViewModelEvent};

use std::sync::Arc;

use crate::terminal::input::MenuPositioningProvider;
use warpui::prelude::Empty;
use warpui::{AppContext, Element, Entity, ModelHandle, TypedActionView, View, ViewContext};

pub fn is_cloud_agent_pre_first_exchange(
    _ambient_agent_view_model: &ModelHandle<AmbientAgentViewModel>,
    _agent_view_controller: &ModelHandle<crate::ai::blocklist::agent_view::AgentViewController>,
    _app: &AppContext,
) -> bool {
    false
}

pub struct AmbientAgentEntryBlock;

impl View for AmbientAgentEntryBlock {
    fn ui_name() -> &'static str {
        "AmbientAgentEntryBlock"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

pub enum AmbientAgentEntryBlockEvent {}

impl Entity for AmbientAgentEntryBlock {
    type Event = AmbientAgentEntryBlockEvent;
}

#[derive(Debug)]
pub enum AmbientAgentEntryBlockAction {}

impl TypedActionView for AmbientAgentEntryBlock {
    type Action = AmbientAgentEntryBlockAction;
}

#[derive(Clone, Debug, PartialEq)]
pub enum HarnessSelectorAction {
    ToggleMenu,
}

pub enum HarnessSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct HarnessSelector;

impl HarnessSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _ambient_agent_model: ModelHandle<AmbientAgentViewModel>,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }

    pub fn set_button_theme<T>(&mut self, _theme: T, _ctx: &mut ViewContext<Self>) {}
}

impl View for HarnessSelector {
    fn ui_name() -> &'static str {
        "HarnessSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for HarnessSelector {
    type Event = HarnessSelectorEvent;
}

impl TypedActionView for HarnessSelector {
    type Action = HarnessSelectorAction;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Local,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostSelectorAction {
    ToggleMenu,
}

pub enum HostSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct HostSelector;

impl HostSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }
}

impl View for HostSelector {
    fn ui_name() -> &'static str {
        "HostSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for HostSelector {
    type Event = HostSelectorEvent;
}

impl TypedActionView for HostSelector {
    type Action = HostSelectorAction;
}

pub struct NakedHeaderButtonTheme;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSelectorAction {
    ToggleMenu,
}

pub enum ModelSelectorEvent {
    MenuVisibilityChanged { open: bool },
}

pub struct ModelSelector;

impl ModelSelector {
    pub fn new(
        _menu_positioning_provider: Arc<dyn MenuPositioningProvider>,
        _terminal_view_id: warpui::EntityId,
        _ctx: &mut ViewContext<Self>,
    ) -> Self {
        Self
    }

    pub fn is_menu_open(&self) -> bool {
        false
    }
}

impl View for ModelSelector {
    fn ui_name() -> &'static str {
        "ModelSelector"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        Empty::new().finish()
    }
}

impl Entity for ModelSelector {
    type Event = ModelSelectorEvent;
}

impl TypedActionView for ModelSelector {
    type Action = ModelSelectorAction;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetupCommandState {
    did_execute_a_setup_command: bool,
    should_expand_setup_commands: bool,
}

impl SetupCommandState {
    pub fn did_execute_a_setup_command(&self) -> bool {
        self.did_execute_a_setup_command
    }

    pub fn set_did_execute_a_setup_command(&mut self, value: bool) {
        self.did_execute_a_setup_command = value;
    }

    pub fn should_expand(&self) -> bool {
        self.should_expand_setup_commands
    }

    pub fn set_should_expand(&mut self, value: bool) {
        self.should_expand_setup_commands = value;
    }
}

pub fn render_loading_footer(
    _appearance: &warp_core::ui::appearance::Appearance,
) -> Box<dyn Element> {
    Empty::new().finish()
}

pub fn render_error_footer(
    _error_message: &str,
    _appearance: &warp_core::ui::appearance::Appearance,
) -> Box<dyn Element> {
    Empty::new().finish()
}

pub struct ProgressProps;
pub struct ProgressStep;
pub struct ProgressStepState;

pub fn render_progress(_props: ProgressProps, _app: &AppContext) -> Box<dyn Element> {
    Empty::new().finish()
}

impl Entity for AmbientAgentViewModel {
    type Event = AmbientAgentViewModelEvent;
}
