use super::{
    settings_page::{
        build_sub_header, build_toggle_element, render_body_item_label, LocalOnlyIconState,
        MatchData, PageType, SettingsPageMeta, SettingsPageViewHandle, SettingsWidget, ToggleState,
        HEADER_PADDING,
    },
    SettingsSection,
};
use crate::terminal::cli_agent_toolbar::editor::AgentToolbarInlineEditor;
use crate::{
    appearance::Appearance,
    menu::{MenuItem, MenuItemFields},
    report_if_error,
    settings::{CLIAgentSettings, CLIAgentSettingsChangedEvent, ShouldRenderCLIAgentToolbar},
    terminal::CLIAgent,
    view_components::{
        dropdown::DropdownAction, Dropdown, SubmittableTextInput, SubmittableTextInputEvent,
    },
};
use enum_iterator::all;
use markdown_parser::{FormattedText, FormattedTextFragment, FormattedTextLine};
use regex::Regex;
use settings::{Setting, ToggleableSetting};
use std::{cell::RefCell, collections::HashMap};
use warp_core::{features::FeatureFlag, ui::theme::color::internal_colors};
use warpui::{
    elements::{
        ChildView, Container, CornerRadius, CrossAxisAlignment, Fill, Flex, FormattedTextElement,
        HighlightedHyperlink, MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement,
        Radius, Shrinkable,
    },
    ui_components::{
        components::{Coords, UiComponent, UiComponentStyles},
        switch::SwitchStateHandle,
    },
    AppContext, Element, Entity, SingletonEntity, TypedActionView, View, ViewContext, ViewHandle,
};

const CONTENT_FONT_SIZE: f32 = 12.;

pub struct CLIAgentSettingsPageView {
    page: PageType<Self>,
    local_only_icon_tooltip_states: RefCell<HashMap<String, MouseStateHandle>>,
    cli_agent_footer_command_editor: ViewHandle<SubmittableTextInput>,
    cli_agent_footer_command_mouse_state_handles: Vec<MouseStateHandle>,
    cli_agent_footer_command_agent_dropdowns: Vec<ViewHandle<Dropdown<CLIAgentSettingsPageAction>>>,
    cli_agent_toolbar_inline_editor: ViewHandle<AgentToolbarInlineEditor>,
}

pub enum CLIAgentSettingsPageEvent {
    FocusModal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CLIAgentSettingsPageAction {
    ToggleCLIAgentToolbar,
    ToggleAutoToggleRichInput,
    ToggleAutoOpenRichInputOnCLIAgentStart,
    ToggleAutoDismissRichInputAfterSubmit,
    RemoveCLIAgentToolbarEnabledCommand(String),
    SetCLIAgentForCommand {
        pattern: String,
        agent: Option<CLIAgent>,
    },
}

impl CLIAgentSettingsPageView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        let cli_agent_footer_command_editor = ctx.add_typed_action_view(|ctx| {
            let mut input =
                SubmittableTextInput::new(ctx).validate_on_edit(|s| Regex::new(s).is_ok());
            input.set_placeholder_text("command (supports regex)", ctx);
            input
        });
        ctx.subscribe_to_view(
            &cli_agent_footer_command_editor,
            |_, _, event, ctx| match event {
                SubmittableTextInputEvent::Submit(command) => CLIAgentSettings::handle(ctx)
                    .update(ctx, |settings, ctx| {
                        settings.add_cli_agent_footer_enabled_command(command, ctx)
                    }),
                SubmittableTextInputEvent::Escape => {
                    ctx.emit(CLIAgentSettingsPageEvent::FocusModal)
                }
            },
        );
        ctx.subscribe_to_model(&CLIAgentSettings::handle(ctx), |me, _, event, ctx| {
            if matches!(
                event,
                CLIAgentSettingsChangedEvent::CLIAgentToolbarEnabledCommands { .. }
            ) {
                me.cli_agent_footer_command_mouse_state_handles = CLIAgentSettings::as_ref(ctx)
                    .cli_agent_footer_enabled_commands
                    .keys()
                    .map(|_| MouseStateHandle::default())
                    .collect();
                me.cli_agent_footer_command_agent_dropdowns = Self::create_cli_agent_dropdowns(ctx);
            }
            ctx.notify();
        });
        let cli_agent_footer_command_mouse_state_handles = CLIAgentSettings::as_ref(ctx)
            .cli_agent_footer_enabled_commands
            .keys()
            .map(|_| MouseStateHandle::default())
            .collect();
        let cli_agent_toolbar_inline_editor =
            ctx.add_typed_action_view(AgentToolbarInlineEditor::new);
        Self {
            page: PageType::new_uncategorized(
                vec![Box::new(CLIAgentWidget::default())],
                Some("Third party CLI agents"),
            ),
            local_only_icon_tooltip_states: RefCell::default(),
            cli_agent_footer_command_editor,
            cli_agent_footer_command_mouse_state_handles,
            cli_agent_footer_command_agent_dropdowns: Self::create_cli_agent_dropdowns(ctx),
            cli_agent_toolbar_inline_editor,
        }
    }
    fn create_cli_agent_dropdowns(
        ctx: &mut ViewContext<Self>,
    ) -> Vec<ViewHandle<Dropdown<CLIAgentSettingsPageAction>>> {
        let entries: Vec<(String, CLIAgent)> = CLIAgentSettings::as_ref(ctx)
            .cli_agent_footer_enabled_commands
            .value()
            .iter()
            .map(|(pattern, agent_value)| {
                (pattern.clone(), CLIAgent::from_serialized_name(agent_value))
            })
            .collect();

        entries
            .into_iter()
            .map(|(pattern_clone, current_agent)| {
                ctx.add_typed_action_view(move |ctx| {
                    let mut dropdown = Dropdown::new(ctx);
                    dropdown.set_top_bar_max_width(160.);
                    dropdown.set_menu_width(180., ctx);
                    dropdown.set_main_axis_size(MainAxisSize::Min, ctx);

                    let mut items: Vec<MenuItem<DropdownAction<CLIAgentSettingsPageAction>>> =
                        Vec::new();

                    for agent in all::<CLIAgent>() {
                        if matches!(agent, CLIAgent::Unknown) {
                            continue;
                        }
                        let icon = agent.icon();
                        let mut fields = MenuItemFields::new(agent.display_name())
                            .with_on_select_action(DropdownAction::SelectActionAndClose(
                                CLIAgentSettingsPageAction::SetCLIAgentForCommand {
                                    pattern: pattern_clone.clone(),
                                    agent: Some(agent),
                                },
                            ));
                        if let Some(icon) = icon {
                            fields = fields.with_icon(icon);
                        }
                        items.push(fields.into_item());
                    }

                    items.push(
                        MenuItemFields::new("Other")
                            .with_on_select_action(DropdownAction::SelectActionAndClose(
                                CLIAgentSettingsPageAction::SetCLIAgentForCommand {
                                    pattern: pattern_clone.clone(),
                                    agent: None,
                                },
                            ))
                            .into_item(),
                    );

                    dropdown.set_rich_items(items, ctx);

                    dropdown.set_menu_header_text_override(|label| {
                        if label == "Other" {
                            "Select coding agent".to_string()
                        } else {
                            label.to_string()
                        }
                    });

                    let selected_name = if matches!(current_agent, CLIAgent::Unknown) {
                        "Other"
                    } else {
                        current_agent.display_name()
                    };
                    dropdown.set_selected_by_name(selected_name, ctx);

                    dropdown
                })
            })
            .collect()
    }
}

impl Entity for CLIAgentSettingsPageView {
    type Event = CLIAgentSettingsPageEvent;
}
impl View for CLIAgentSettingsPageView {
    fn ui_name() -> &'static str {
        "CLIAgentSettingsPage"
    }
    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        self.page.render(self, app)
    }
}
impl TypedActionView for CLIAgentSettingsPageView {
    type Action = CLIAgentSettingsPageAction;
    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        CLIAgentSettings::handle(ctx).update(ctx, |settings, ctx| match action {
            CLIAgentSettingsPageAction::ToggleCLIAgentToolbar => report_if_error!(settings
                .should_render_cli_agent_footer
                .toggle_and_save_value(ctx)),
            CLIAgentSettingsPageAction::ToggleAutoToggleRichInput => {
                report_if_error!(settings.auto_toggle_rich_input.toggle_and_save_value(ctx))
            }
            CLIAgentSettingsPageAction::ToggleAutoOpenRichInputOnCLIAgentStart => {
                report_if_error!(settings
                    .auto_open_rich_input_on_cli_agent_start
                    .toggle_and_save_value(ctx))
            }
            CLIAgentSettingsPageAction::ToggleAutoDismissRichInputAfterSubmit => {
                report_if_error!(settings
                    .auto_dismiss_rich_input_after_submit
                    .toggle_and_save_value(ctx))
            }
            CLIAgentSettingsPageAction::RemoveCLIAgentToolbarEnabledCommand(command) => {
                settings.remove_cli_agent_footer_enabled_command(command, ctx)
            }
            CLIAgentSettingsPageAction::SetCLIAgentForCommand { pattern, agent } => {
                settings.set_cli_agent_for_command(pattern, *agent, ctx)
            }
        });
        ctx.notify();
    }
}
impl SettingsPageMeta for CLIAgentSettingsPageView {
    fn section() -> SettingsSection {
        SettingsSection::ThirdPartyCLIAgents
    }

    fn should_render(&self, _ctx: &AppContext) -> bool {
        true
    }

    fn update_filter(&mut self, query: &str, ctx: &mut ViewContext<Self>) -> MatchData {
        self.page.update_filter(query, ctx)
    }

    fn scroll_to_widget(&mut self, widget_id: &'static str) {
        self.page.scroll_to_widget(widget_id)
    }

    fn clear_highlighted_widget(&mut self) {
        self.page.clear_highlighted_widget();
    }
}

impl From<ViewHandle<CLIAgentSettingsPageView>> for SettingsPageViewHandle {
    fn from(view_handle: ViewHandle<CLIAgentSettingsPageView>) -> Self {
        SettingsPageViewHandle::CLIAgents(view_handle)
    }
}

fn render_setting_toggle<S: Setting>(
    label: impl Into<String>,
    action: CLIAgentSettingsPageAction,
    is_setting_enabled: bool,
    is_setting_toggleable: bool,
    switch_state: SwitchStateHandle,
    tooltip_states: &RefCell<HashMap<String, MouseStateHandle>>,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    build_toggle_element(
        render_body_item_label::<CLIAgentSettingsPageAction>(
            label.into(),
            Some(styles::header_font_color(is_setting_toggleable, app)),
            None,
            LocalOnlyIconState::for_setting(
                S::storage_key(),
                S::sync_to_cloud(),
                &mut tooltip_states.borrow_mut(),
                app,
            ),
            ToggleState::Enabled,
            appearance,
        ),
        render_feature_switch(
            switch_state,
            is_setting_enabled,
            is_setting_toggleable,
            action,
            app,
        ),
        appearance,
        None,
    )
}

fn render_feature_switch(
    state_handle: SwitchStateHandle,
    is_setting_enabled: bool,
    is_setting_toggleable: bool,
    toggle_action: CLIAgentSettingsPageAction,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let ui_builder = appearance.ui_builder();
    ui_builder
        .switch(state_handle)
        .check(is_setting_enabled)
        .with_disabled(!is_setting_toggleable)
        .with_disabled_styles(UiComponentStyles {
            background: Some(Fill::Solid(internal_colors::neutral_4(appearance.theme()))),
            foreground: Some(Fill::Solid(internal_colors::neutral_5(appearance.theme()))),
            ..Default::default()
        })
        .build()
        .on_click(move |ctx, _, _| {
            if !is_setting_toggleable {
                return;
            }
            ctx.dispatch_typed_action(toggle_action.clone());
        })
        .finish()
}

fn render_toolbar_layout_editor(
    editor: &ViewHandle<AgentToolbarInlineEditor>,
    appearance: &Appearance,
) -> Box<dyn Element> {
    let label = Container::new(
        appearance
            .ui_builder()
            .span("Toolbar layout".to_string())
            .with_style(UiComponentStyles {
                font_size: Some(CONTENT_FONT_SIZE),
                ..Default::default()
            })
            .build()
            .finish(),
    )
    .with_margin_bottom(4.)
    .finish();
    let editor = Container::new(ChildView::new(editor).finish())
        .with_margin_bottom(16.)
        .finish();

    Flex::column().with_child(label).with_child(editor).finish()
}

pub(crate) fn cli_agent_settings_widget_id() -> &'static str {
    CLIAgentWidget::static_widget_id()
}

#[derive(Default)]
struct CLIAgentWidget {
    cli_agent_footer_toggle: SwitchStateHandle,
    auto_toggle_rich_input_toggle: SwitchStateHandle,
    auto_toggle_rich_input_info_tooltip: MouseStateHandle,
    auto_open_rich_input_on_cli_agent_start_toggle: SwitchStateHandle,
    auto_dismiss_rich_input_toggle: SwitchStateHandle,
}

impl SettingsWidget for CLIAgentWidget {
    type View = CLIAgentSettingsPageView;

    fn search_terms(&self) -> &str {
        "third party cli coding agent claude codex gemini toolbar footer layout chip chips rearrange re-arrange bar command regex auto show rich input dismiss"
    }

    fn render(
        &self,
        view: &Self::View,
        appearance: &Appearance,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let cli_agent_settings = CLIAgentSettings::as_ref(app);

        let cli_agent_footer_toggle = render_setting_toggle::<ShouldRenderCLIAgentToolbar>(
            "Show coding agent toolbar",
            CLIAgentSettingsPageAction::ToggleCLIAgentToolbar,
            *cli_agent_settings.should_render_cli_agent_footer,
            true,
            self.cli_agent_footer_toggle.clone(),
            &view.local_only_icon_tooltip_states,
            app,
        );

        let description_fragments = vec![
            FormattedTextFragment::plain_text(
                "Show a toolbar with quick actions when running coding agents like ",
            ),
            FormattedTextFragment::inline_code("claude"),
            FormattedTextFragment::plain_text(", "),
            FormattedTextFragment::inline_code("codex"),
            FormattedTextFragment::plain_text(", or "),
            FormattedTextFragment::inline_code("gemini"),
            FormattedTextFragment::plain_text("."),
        ];

        let description = FormattedTextElement::new(
            FormattedText::new([FormattedTextLine::Line(description_fragments)]),
            appearance.ui_font_size(),
            appearance.ui_font_family(),
            appearance.monospace_font_family(),
            styles::description_font_color(true, app).into(),
            HighlightedHyperlink::default(),
        );

        let is_footer_enabled = *cli_agent_settings.should_render_cli_agent_footer;

        let mut column = Flex::column()
            .with_child(
                build_sub_header(
                    appearance,
                    "Third party CLI agents",
                    Some(styles::header_font_color(true, app)),
                )
                .with_padding_bottom(HEADER_PADDING)
                .finish(),
            )
            .with_child(cli_agent_footer_toggle)
            .with_child(
                Container::new(description.finish())
                    .with_margin_top(styles::DESCRIPTION_NEGATIVE_MARGIN_OFFSET)
                    .with_margin_bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                    .with_margin_right(styles::TOGGLE_WIDTH_MARGIN)
                    .finish(),
            );

        if is_footer_enabled {
            use super::settings_page::AdditionalInfo;
            use crate::settings::{
                AutoDismissRichInputAfterSubmit, AutoOpenRichInputOnCLIAgentStart,
                AutoToggleRichInput,
            };

            if FeatureFlag::CLIAgentRichInput.is_enabled() {
                // Setting 1: Auto show/hide rich input based on agent status
                let auto_show_toggle_label = render_body_item_label::<CLIAgentSettingsPageAction>(
                    "Auto show/hide Rich Input based on agent status".into(),
                    Some(styles::header_font_color(true, app)),
                    Some(AdditionalInfo {
                        mouse_state: self.auto_toggle_rich_input_info_tooltip.clone(),
                        on_click_action: None,
                        secondary_text: None,
                        tooltip_override_text: Some(
                            "Requires the Warper plugin for your coding agent".to_owned(),
                        ),
                    }),
                    LocalOnlyIconState::for_setting(
                        AutoToggleRichInput::storage_key(),
                        AutoToggleRichInput::sync_to_cloud(),
                        &mut view.local_only_icon_tooltip_states.borrow_mut(),
                        app,
                    ),
                    ToggleState::Enabled,
                    appearance,
                );
                column.add_child(build_toggle_element(
                    auto_show_toggle_label,
                    render_feature_switch(
                        self.auto_toggle_rich_input_toggle.clone(),
                        *cli_agent_settings.auto_toggle_rich_input,
                        true,
                        CLIAgentSettingsPageAction::ToggleAutoToggleRichInput,
                        app,
                    ),
                    appearance,
                    None,
                ));

                column.add_child(render_setting_toggle::<AutoOpenRichInputOnCLIAgentStart>(
                    "Auto open Rich Input when a coding agent session starts",
                    CLIAgentSettingsPageAction::ToggleAutoOpenRichInputOnCLIAgentStart,
                    *cli_agent_settings.auto_open_rich_input_on_cli_agent_start,
                    true,
                    self.auto_open_rich_input_on_cli_agent_start_toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ));

                // Setting 2: Auto dismiss rich input after prompt submission
                column.add_child(render_setting_toggle::<AutoDismissRichInputAfterSubmit>(
                    "Auto dismiss Rich Input after prompt submission",
                    CLIAgentSettingsPageAction::ToggleAutoDismissRichInputAfterSubmit,
                    *cli_agent_settings.auto_dismiss_rich_input_after_submit,
                    true,
                    self.auto_dismiss_rich_input_toggle.clone(),
                    &view.local_only_icon_tooltip_states,
                    app,
                ));
            }

            let command_list = {
                let mut list_column = Flex::column();

                list_column.add_child(
                    appearance
                        .ui_builder()
                        .span("Commands that enable the toolbar".to_string())
                        .with_style(UiComponentStyles {
                            font_size: Some(CONTENT_FONT_SIZE),
                            ..Default::default()
                        })
                        .build()
                        .finish(),
                );

                list_column
                    .add_child(ChildView::new(&view.cli_agent_footer_command_editor).finish());

                let background = appearance.theme().surface_1();
                let font_color = appearance.theme().foreground();
                let items: Vec<_> = cli_agent_settings
                    .cli_agent_footer_enabled_commands
                    .value()
                    .keys()
                    .cloned()
                    .collect();
                let len = items.len();
                for (rev_i, pattern) in items.iter().rev().enumerate() {
                    let original_i = len - 1 - rev_i;
                    let remove_action =
                        CLIAgentSettingsPageAction::RemoveCLIAgentToolbarEnabledCommand(
                            pattern.clone(),
                        );
                    let mouse_state = view
                        .cli_agent_footer_command_mouse_state_handles
                        .get(original_i)
                        .cloned()
                        .unwrap_or_default();

                    let remove_button = appearance
                        .ui_builder()
                        .close_button(16., mouse_state)
                        .build()
                        .on_click(move |ctx, _, _| {
                            ctx.dispatch_typed_action(remove_action.clone());
                        })
                        .finish();

                    let label = appearance
                        .ui_builder()
                        .wrappable_text(pattern.clone(), true)
                        .with_style(UiComponentStyles {
                            font_color: Some(font_color.into_solid()),
                            font_family_id: Some(appearance.monospace_font_family()),
                            font_size: Some(appearance.ui_font_size()),
                            ..Default::default()
                        })
                        .build()
                        .finish();

                    let mut right_side =
                        Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);
                    if let Some(dropdown_handle) = view
                        .cli_agent_footer_command_agent_dropdowns
                        .get(original_i)
                    {
                        right_side.add_child(
                            Container::new(ChildView::new(dropdown_handle).finish())
                                .with_margin_right(8.)
                                .finish(),
                        );
                    }
                    right_side.add_child(remove_button);

                    let row = Container::new(
                        Flex::row()
                            .with_cross_axis_alignment(CrossAxisAlignment::Center)
                            .with_main_axis_size(MainAxisSize::Max)
                            .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                            .with_children([
                                Shrinkable::new(1., label).finish(),
                                right_side.finish(),
                            ])
                            .finish(),
                    )
                    .with_background(background)
                    .with_horizontal_padding(8.)
                    .with_vertical_padding(4.)
                    .with_corner_radius(CornerRadius::with_all(Radius::Pixels(4.)))
                    .with_margin_bottom(4.)
                    .finish();

                    list_column.add_child(row);
                }

                list_column.finish()
            };
            let command_list_description = appearance
                .ui_builder()
                .paragraph(
                    "Add regex patterns to show the coding agent toolbar for matching commands.",
                )
                .with_style(UiComponentStyles {
                    font_size: Some(appearance.ui_font_size()),
                    font_color: Some(styles::description_font_color(true, app).into()),
                    margin: Some(
                        Coords::default()
                            .top(4.)
                            .bottom(styles::DESCRIPTION_MARGIN_BOTTOM)
                            .right(styles::TOGGLE_WIDTH_MARGIN),
                    ),
                    ..Default::default()
                })
                .build()
                .finish();

            column.add_child(command_list);
            column.add_child(command_list_description);

            if FeatureFlag::AgentToolbarEditor.is_enabled() {
                column.add_child(render_toolbar_layout_editor(
                    &view.cli_agent_toolbar_inline_editor,
                    appearance,
                ));
            }
        }

        column.finish()
    }
}

mod styles {
    use warp_core::ui::{appearance::Appearance, theme::Fill};
    use warpui::{AppContext, SingletonEntity};

    // Apply a negative margin to the description text so it appears closer to the main
    // settings option text.
    pub const DESCRIPTION_NEGATIVE_MARGIN_OFFSET: f32 = -12.;

    /// The space between a description and the next toggle.
    pub const DESCRIPTION_MARGIN_BOTTOM: f32 = 12.;

    /// Margin to leave for switch toggle to the right of the description subtext.
    pub const TOGGLE_WIDTH_MARGIN: f32 = 48.;

    pub fn header_font_color(is_enabled_setting: bool, app: &AppContext) -> Fill {
        let appearance = Appearance::as_ref(app);
        if is_enabled_setting {
            appearance
                .theme()
                .main_text_color(appearance.theme().surface_2())
        } else {
            appearance.theme().disabled_ui_text_color()
        }
    }

    pub fn description_font_color(is_enabled_setting: bool, app: &AppContext) -> Fill {
        let appearance = Appearance::as_ref(app);
        if is_enabled_setting {
            appearance
                .theme()
                .sub_text_color(appearance.theme().surface_1())
        } else {
            appearance.theme().disabled_ui_text_color()
        }
    }
}
