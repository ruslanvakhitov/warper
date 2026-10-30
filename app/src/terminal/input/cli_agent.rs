use super::{
    common::{add_input_suggestions_overlays, wrap_input_with_terminal_padding_and_focus_handler},
    Input, InputDropTargetData, CLI_AGENT_RICH_INPUT_EDITOR_BOTTOM_PADDING,
    CLI_AGENT_RICH_INPUT_EDITOR_MAX_HEIGHT, CLI_AGENT_RICH_INPUT_EDITOR_TOP_PADDING,
    TERMINAL_VIEW_PADDING_LEFT,
};
use crate::{
    appearance::Appearance,
    context_chips::spacing,
    editor::TextColors,
    terminal::{cli_agent_sessions::CLIAgentSessionsModel, view::TerminalAction},
};
use warp_core::ui::{
    color::{contrast::MinimumAllowedContrast, ContrastingColor},
    theme::color::internal_colors,
};
use warpui::{
    elements::{
        Border, Clipped, ConstrainedBox, Container, DispatchEventResult, DropTarget, Element,
        EventHandler, Flex, Hoverable, ParentElement, SavePosition, Stack,
    },
    presenter::ChildView,
    AppContext, SingletonEntity as _, ViewContext,
};

impl Input {
    /// Renders the CLI rich input (editor + CLI agent footer).
    pub(super) fn render_cli_agent_input(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let menu_positioning = self.menu_positioning(app);

        let mut stack = Stack::new().with_constrain_absolute_children();

        let input_box = Container::new(
            ConstrainedBox::new(Clipped::new(ChildView::new(&self.editor).finish()).finish())
                .with_max_height(CLI_AGENT_RICH_INPUT_EDITOR_MAX_HEIGHT)
                .finish(),
        )
        .with_padding_top(CLI_AGENT_RICH_INPUT_EDITOR_TOP_PADDING)
        .with_padding_right(*TERMINAL_VIEW_PADDING_LEFT)
        .with_padding_bottom(CLI_AGENT_RICH_INPUT_EDITOR_BOTTOM_PADDING)
        .finish();

        let input_editor_save_position_id = self.editor_save_position_id();
        let editor_element = SavePosition::new(
            EventHandler::new(input_box)
                .on_right_mouse_down(move |ctx, _, position| {
                    let input_rect = ctx
                        .element_position_by_id(input_editor_save_position_id.clone())
                        .expect("input editor position id should be saved");
                    let offset_position = position - input_rect.origin();
                    ctx.dispatch_typed_action(TerminalAction::OpenInputContextMenu {
                        position: offset_position,
                    });
                    DispatchEventResult::StopPropagation
                })
                .finish(),
            &self.editor_save_position_id(),
        )
        .finish();

        let mut column = Flex::column();

        if let Some(chips) = self.render_cli_image_chips(appearance) {
            column.add_child(
                Container::new(chips)
                    .with_margin_top(spacing::UDI_CHIP_MARGIN)
                    .finish(),
            );
        }
        column.add_child(editor_element);
        column.add_child(
            SavePosition::new(
                Container::new(ChildView::new(&self.agent_input_footer).finish())
                    .with_padding_right(*TERMINAL_VIEW_PADDING_LEFT)
                    .finish(),
                &self.prompt_save_position_id(),
            )
            .finish(),
        );

        stack.add_child(wrap_input_with_terminal_padding_and_focus_handler(
            self.is_active_session(app),
            column.finish(),
            false,
        ));

        if self.is_pane_focused(app) {
            add_input_suggestions_overlays(self, &mut stack, appearance, menu_positioning, app);
        }

        let mut input_container = Container::new(stack.finish()).with_border(
            Border::top(1.0).with_border_fill(internal_colors::fg_overlay_2(appearance.theme())),
        );

        // When an alt screen CLI agent (e.g. OpenCode) is running, match
        // the rich input background to the alt screen so it blends in.
        {
            let terminal_model = self.model.lock();
            if terminal_model.is_alt_screen_active() {
                if let Some(bg_color) = terminal_model.alt_screen().inferred_bg_color() {
                    input_container = input_container.with_background(bg_color);
                }
            }
        }

        let drop_target = DropTarget::new(
            input_container.finish(),
            InputDropTargetData::new(self.weak_view_handle.clone()),
        )
        .finish();

        let input = SavePosition::new(
            Hoverable::new(self.hoverable_handle.clone(), |_| drop_target)
                .on_middle_click(|ctx, _app, _position| {
                    ctx.dispatch_typed_action(TerminalAction::MiddleClickOnInput)
                })
                .finish(),
            &self.status_free_input_save_position_id(),
        )
        .finish();

        SavePosition::new(input, &self.save_position_id()).finish()
    }

    /// Keep the rich input editor's text colors legible when it's rendered on
    /// top of an alt-screen CLI agent's inferred background (e.g. OpenCode),
    /// which does not respect the Warp theme. When no alt-screen-backed CLI
    /// agent rich input is active, restores the theme default text colors.
    ///
    /// This mirrors the contrast-adjustment pattern used for the use-agent
    /// toolbar button text (see `AgentFooterButtonTheme::text_color`) and the
    /// CLI agent brand icon in `AgentInputFooter::render_cli_mode_footer`.
    pub(super) fn update_cli_agent_editor_text_colors(&mut self, ctx: &mut ViewContext<Self>) {
        let appearance = Appearance::as_ref(ctx);
        let default_colors = TextColors::from_appearance(appearance);

        // The terminal and CLI rich input share the editor. Apply the override
        // while CLI rich input is open.
        let rich_input_open =
            CLIAgentSessionsModel::as_ref(ctx).is_input_open(self.terminal_view_id);

        let alt_screen_bg = if rich_input_open {
            let terminal_model = self.model.lock();
            terminal_model
                .is_alt_screen_active()
                .then(|| terminal_model.alt_screen().inferred_bg_color())
                .flatten()
        } else {
            None
        };

        let text_colors = match alt_screen_bg {
            Some(bg) => TextColors {
                default_color: default_colors
                    .default_color
                    .on_background(bg.into(), MinimumAllowedContrast::Text),
                disabled_color: default_colors
                    .disabled_color
                    .on_background(bg.into(), MinimumAllowedContrast::Text),
                hint_color: default_colors
                    .hint_color
                    .on_background(bg.into(), MinimumAllowedContrast::Text),
            },
            None => default_colors,
        };

        self.editor.update(ctx, |editor, ctx| {
            editor.set_text_colors(text_colors, ctx);
        });
    }
}
