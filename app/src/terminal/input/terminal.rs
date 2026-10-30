use super::{
    common::{
        add_command_xray_overlay, add_input_suggestions_overlays, add_voltron_overlay,
        add_workflow_info_overlay, wrap_input_with_terminal_padding_and_focus_handler,
    },
    Input, InputDropTargetData,
};

use crate::{
    appearance::Appearance,
    context_chips::spacing,
    settings::AppEditorSettings,
    terminal::{
        block_list_settings::BlockListSettings, settings::TerminalSettings, view::TerminalAction,
    },
};
use warp_core::settings::Setting;
use warpui::{
    elements::{
        Border, Container, DropTarget, Element, Flex, Hoverable, ParentElement, SavePosition, Stack,
    },
    AppContext, SingletonEntity,
};

impl Input {
    /// Renders the terminal input using context chips for the prompt.
    pub(super) fn render_terminal_input(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let menu_positioning = self.menu_positioning(app);

        let model = self.model.lock();

        // We should likely rework this stack to not need to use `with_constrain_absolute_children`,
        // by reworking the positioning of the children to not depend on this.
        let mut stack = Stack::new().with_constrain_absolute_children();

        let vim_state = self.editor.as_ref(app).vim_state(app);
        let app_editor_settings = AppEditorSettings::as_ref(app);
        let show_vim_status = vim_state.is_some() && *app_editor_settings.vim_status_bar.value();

        let mut column = Flex::column();

        let prompt_elements = self
            .prompt_render_helper
            .render_universal_developer_input_prompt(&model, appearance, app);

        column.add_child(prompt_elements);

        let terminal_spacing = TerminalSettings::as_ref(app)
            .terminal_input_spacing(appearance.line_height_ratio(), app);
        column.add_child(
            Container::new(self.render_input_box(show_vim_status, appearance, app))
                .with_margin_top(
                    terminal_spacing.prompt_to_editor_padding
                        * spacing::UDI_PROMPT_BOTTOM_PADDING_FACTOR,
                )
                .finish(),
        );

        column.add_child(
            Container::new(Flex::row().finish())
                .with_margin_bottom(4.)
                .finish(),
        );

        stack.add_child(wrap_input_with_terminal_padding_and_focus_handler(
            self.focus_handle
                .as_ref()
                .is_some_and(|h| h.is_active_session(app)),
            column.finish(),
            false,
        ));

        if let Some(selected_workflow_state) = self.workflows_state.selected_workflow_state.as_ref()
        {
            if selected_workflow_state.should_show_more_info_view {
                add_workflow_info_overlay(
                    &mut stack,
                    selected_workflow_state,
                    self.size_info(app).pane_height_px().as_f32(),
                    menu_positioning,
                );
            }
        }

        let is_focused = self.focus_handle.as_ref().is_none_or(|h| h.is_focused(app));
        if self.is_voltron_open && is_focused {
            add_voltron_overlay(&mut stack, &self.voltron_view, menu_positioning);
        }

        if is_focused {
            add_input_suggestions_overlays(self, &mut stack, appearance, menu_positioning, app);
        }

        if let Some(token_description) = &self.command_x_ray_description {
            add_command_xray_overlay(
                self,
                &mut stack,
                token_description,
                appearance,
                menu_positioning,
                app,
            );
        }

        let drop_target = DropTarget::new(
            SavePosition::new(stack.finish(), &self.status_free_input_save_position_id()).finish(),
            InputDropTargetData::new(self.weak_view_handle.clone()),
        )
        .finish();

        let hoverable_input = Hoverable::new(self.hoverable_handle.clone(), |_| drop_target)
            .on_middle_click(|ctx, _app, _position| {
                ctx.dispatch_typed_action(TerminalAction::MiddleClickOnInput)
            })
            .finish();

        let show_block_dividers = *BlockListSettings::as_ref(app).show_block_dividers.value();

        let input = if show_block_dividers {
            Container::new(hoverable_input)
                .with_border(
                    Border::top(1.)
                        .with_border_color(styles::default_border_color(appearance.theme())),
                )
                .finish()
        } else {
            hoverable_input
        };

        SavePosition::new(input, &self.save_position_id()).finish()
    }
}

pub mod styles {
    use pathfinder_color::ColorU;
    use warp_core::ui::theme::WarpTheme;
    pub fn default_border_color(theme: &WarpTheme) -> ColorU {
        theme.outline().into()
    }
}
