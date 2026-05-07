use crate::editor::{
    EditorView, Event as EditorEvent, PropagateAndNoOpNavigationKeys, SingleLineEditorOptions,
    TextOptions,
};
use crate::search_bar::SearchBar;
use crate::settings::{AISettings, AISettingsChangedEvent};
use crate::ui_components::icons::Icon;
use ai::project_context::model::{ProjectContextModel, ProjectContextModelEvent};
use markdown_parser::{
    weight::CustomWeight, FormattedText, FormattedTextFragment, FormattedTextLine,
};
use std::fmt::Debug;
use std::path::PathBuf;
use warp_core::ui::{
    appearance::{Appearance, AppearanceEvent},
    theme::color::internal_colors,
};
use warpui::elements::Shrinkable;
use warpui::{
    elements::{
        Align, Border, ChildView, ConstrainedBox, Container, CornerRadius, CrossAxisAlignment,
        Expanded, Flex, FormattedTextElement, HighlightedHyperlink, MainAxisAlignment,
        MainAxisSize, ParentElement,
    },
    ui_components::components::UiComponent,
    AppContext, Element, Entity, FocusContext, SingletonEntity, TypedActionView, View, ViewContext,
    ViewHandle,
};

use super::style;

pub const HEADER_TEXT: &str = "Rules";
const DESCRIPTION_TEXT: &str =
    "Project rules enhance the agent with local WARP.md guidance for this codebase.";

const SEARCH_PLACEHOLDER_TEXT: &str = "Search rules";
const ZERO_STATE_TEXT: &str = "No project rules found.";

const DISABLED_BANNER_TEXT: &str =
    "Your rules are disabled and won't be used as context in sessions. You can ";
const DISABLED_BANNER_LINK_TEXT: &str = "turn it back on";
const DISABLED_BANNER_TEXT_2: &str = " anytime.";

#[derive(Debug, Clone)]
struct ProjectScopedRow {
    file_path: PathBuf,
}

#[derive(Debug, Clone)]
enum RuleRow {
    ProjectScoped(ProjectScopedRow),
}

impl RuleRow {
    fn matches_search_term(&self, search_term: &str) -> bool {
        match self {
            RuleRow::ProjectScoped(row) => row
                .file_path
                .to_str()
                .map(|s| s.to_lowercase().contains(search_term))
                .unwrap_or(false),
        }
    }

    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (RuleRow::ProjectScoped(a), RuleRow::ProjectScoped(b)) => a.file_path.cmp(&b.file_path),
        }
    }
}

pub struct RuleView {
    project_rules: Vec<ProjectScopedRow>,
    search_editor: ViewHandle<EditorView>,
    search_bar: ViewHandle<SearchBar>,
    disabled_banner_highlight_index: HighlightedHyperlink,
}

impl RuleView {
    pub fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&AISettings::handle(ctx), |_, _, event, ctx| {
            if matches!(
                event,
                AISettingsChangedEvent::MemoryEnabled { .. }
                    | AISettingsChangedEvent::IsAnyAIEnabled { .. }
            ) {
                ctx.notify();
            }
        });

        let project_context = ProjectContextModel::handle(ctx);
        let project_rules = project_context
            .as_ref(ctx)
            .indexed_rules()
            .map(|p| ProjectScopedRow { file_path: p })
            .collect();

        ctx.subscribe_to_model(&project_context, |me, context_model, event, ctx| {
            if matches!(event, ProjectContextModelEvent::PathIndexed) {
                me.project_rules = context_model
                    .as_ref(ctx)
                    .indexed_rules()
                    .map(|p| ProjectScopedRow { file_path: p })
                    .collect();

                ctx.notify();
            }
        });

        let appearance = Appearance::handle(ctx);
        ctx.subscribe_to_model(&appearance, move |me, _, event, ctx| {
            if let AppearanceEvent::ThemeChanged = event {
                let appearance = Appearance::as_ref(ctx);
                let search_bar_styles = style::search_bar(appearance);
                me.search_bar.update(ctx, |search_bar, _| {
                    search_bar.with_style(search_bar_styles)
                });
            }
        });

        let search_editor_text = TextOptions::ui_text(None, appearance.as_ref(ctx));
        let search_editor = {
            let options = SingleLineEditorOptions {
                text: search_editor_text,
                propagate_and_no_op_vertical_navigation_keys:
                    PropagateAndNoOpNavigationKeys::Always,
                ..Default::default()
            };
            ctx.add_typed_action_view(|ctx| EditorView::single_line(options, ctx))
        };
        ctx.subscribe_to_view(&search_editor, move |me, _, event, ctx| {
            me.handle_search_editor_event(event, ctx);
        });

        search_editor.update(ctx, |editor, ctx| {
            editor.clear_buffer_and_reset_undo_stack(ctx);
            editor.set_placeholder_text(SEARCH_PLACEHOLDER_TEXT, ctx);
        });
        let search_bar = ctx.add_typed_action_view(|_| SearchBar::new(search_editor.clone()));

        Self {
            project_rules,
            search_editor,
            search_bar,
            disabled_banner_highlight_index: Default::default(),
        }
    }

    fn handle_search_editor_event(&mut self, _event: &EditorEvent, ctx: &mut ViewContext<Self>) {
        ctx.notify();
    }

    fn get_filtered_rules(&self) -> Vec<RuleRow> {
        self.project_rules
            .iter()
            .cloned()
            .map(RuleRow::ProjectScoped)
            .collect()
    }

    fn render_header(&self, appearance: &Appearance) -> Box<dyn Element> {
        Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(
                Container::new(
                    ConstrainedBox::new(
                        warpui::elements::Icon::new(
                            Icon::BookOpen.into(),
                            appearance
                                .theme()
                                .main_text_color(appearance.theme().background()),
                        )
                        .finish(),
                    )
                    .with_width(style::ICON_SIZE)
                    .with_height(style::ICON_SIZE)
                    .finish(),
                )
                .with_margin_right(style::ICON_MARGIN)
                .finish(),
            )
            .with_child(
                appearance
                    .ui_builder()
                    .wrappable_text(HEADER_TEXT, true)
                    .with_style(style::header_text())
                    .build()
                    .finish(),
            )
            .finish()
    }

    fn render_description(&self, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            appearance
                .ui_builder()
                .wrappable_text(DESCRIPTION_TEXT, true)
                .with_style(style::description_text(appearance))
                .build()
                .finish(),
        )
        .with_vertical_margin(style::ITEM_BOTTOM_MARGIN)
        .finish()
    }

    fn render_disabled_banner(&self, appearance: &Appearance) -> Box<dyn Element> {
        let mut link = FormattedTextFragment::hyperlink(DISABLED_BANNER_LINK_TEXT, "Settings > AI");
        link.styles.weight = Some(CustomWeight::Bold);

        let formatted_text = FormattedTextElement::new(
            FormattedText::new([FormattedTextLine::Line(vec![
                FormattedTextFragment::bold(DISABLED_BANNER_TEXT),
                link,
                FormattedTextFragment::bold(DISABLED_BANNER_TEXT_2),
            ])]),
            style::SUBTEXT_FONT_SIZE,
            appearance.ui_font_family(),
            appearance.ui_font_family(),
            appearance
                .theme()
                .sub_text_color(appearance.theme().background())
                .into(),
            self.disabled_banner_highlight_index.clone(),
        )
        .with_hyperlink_font_color(internal_colors::accent_fg_strong(appearance.theme()).into())
        .register_default_click_handlers(|_, _ctx, _| {});

        Container::new(
            Flex::row()
                .with_cross_axis_alignment(CrossAxisAlignment::Center)
                .with_child(
                    Container::new(
                        ConstrainedBox::new(
                            Icon::Info
                                .to_warpui_icon(
                                    appearance
                                        .theme()
                                        .sub_text_color(appearance.theme().background()),
                                )
                                .finish(),
                        )
                        .with_width(style::BANNER_ICON_SIZE)
                        .with_height(style::BANNER_ICON_SIZE)
                        .finish(),
                    )
                    .with_margin_right(style::ROW_ICON_MARGIN)
                    .finish(),
                )
                .with_child(Expanded::new(1., formatted_text.finish()).finish())
                .finish(),
        )
        .with_background(appearance.theme().accent_overlay())
        .with_corner_radius(CornerRadius::with_all(warpui::elements::Radius::Pixels(4.)))
        .with_uniform_padding(style::BANNER_PADDING)
        .with_margin_bottom(style::ITEM_BOTTOM_MARGIN)
        .finish()
    }

    fn render_search_bar_row(&self) -> Box<dyn Element> {
        let row = Flex::row()
            .with_cross_axis_alignment(CrossAxisAlignment::Center)
            .with_child(Expanded::new(1., ChildView::new(&self.search_bar).finish()).finish());
        Container::new(row.finish())
            .with_margin_bottom(style::SECTION_MARGIN)
            .finish()
    }

    fn render_project_based_row(
        &self,
        project_row: ProjectScopedRow,
        appearance: &Appearance,
    ) -> Option<Box<dyn Element>> {
        let row_name = project_row.file_path.to_str().map(|s| s.to_string())?;
        let row = Shrinkable::new(
            1.,
            appearance
                .ui_builder()
                .wrappable_text(row_name, true)
                .with_style(style::fact_project_based_row_text(appearance))
                .build()
                .finish(),
        )
        .finish();

        Some(
            Container::new(row)
                .with_background(internal_colors::neutral_1(appearance.theme()))
                .with_corner_radius(CornerRadius::with_all(warpui::elements::Radius::Pixels(4.)))
                .with_border(
                    Border::all(1.)
                        .with_border_color(internal_colors::neutral_2(appearance.theme())),
                )
                .with_horizontal_padding(style::ROW_HORIZONTAL_PADDING)
                .with_vertical_padding(style::RULE_VERTICAL_PADDING)
                .with_margin_bottom(style::ITEM_BOTTOM_MARGIN)
                .finish(),
        )
    }

    fn render_items(
        &self,
        appearance: &Appearance,
        mut filtered_rules: Vec<RuleRow>,
        app: &AppContext,
    ) -> Box<dyn Element> {
        let mut col = Flex::column();

        // Filter the rows based on the search query
        let search_term = self.search_editor.as_ref(app).buffer_text(app);
        if !search_term.is_empty() {
            filtered_rules = filtered_rules
                .iter()
                .filter(|row| row.matches_search_term(search_term.as_str()))
                .cloned()
                .collect();
        }
        // Sort the rows by the last modified timestamp
        filtered_rules.sort_by(|a, b| a.cmp(b));

        for row in filtered_rules {
            let row = match row {
                RuleRow::ProjectScoped(project_row) => {
                    self.render_project_based_row(project_row, appearance)
                }
            };

            if let Some(row) = row {
                col.add_child(row);
            }
        }
        col.finish()
    }

    fn render_zero_state(&self, appearance: &Appearance) -> Box<dyn Element> {
        Container::new(
            ConstrainedBox::new(
                Align::new(
                    Flex::column()
                        .with_main_axis_size(MainAxisSize::Max)
                        .with_main_axis_alignment(MainAxisAlignment::Center)
                        .with_cross_axis_alignment(CrossAxisAlignment::Center)
                        .with_child(
                            appearance
                                .ui_builder()
                                .wrappable_text(ZERO_STATE_TEXT, true)
                                .with_style(style::description_text(appearance))
                                .build()
                                .finish(),
                        )
                        .finish(),
                )
                .finish(),
            )
            .with_height(style::ZERO_STATE_HEIGHT)
            .finish(),
        )
        .with_border(
            Border::all(1.).with_border_color(internal_colors::neutral_2(appearance.theme())),
        )
        .with_margin_bottom(style::SECTION_MARGIN)
        .finish()
    }

    fn render_body(
        &self,
        appearance: &Appearance,
        filtered_rules: Vec<RuleRow>,
        app: &AppContext,
    ) -> Box<dyn Element> {
        Flex::column()
            .with_child(self.render_search_bar_row())
            .with_child(self.render_items(appearance, filtered_rules, app))
            .finish()
    }
}

impl Entity for RuleView {
    type Event = ();
}

impl View for RuleView {
    fn ui_name() -> &'static str {
        "RuleView"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            ctx.focus(&self.search_editor);
        }
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let mut col = Flex::column()
            .with_child(self.render_header(appearance))
            .with_child(self.render_description(appearance));

        let ai_settings = AISettings::as_ref(app);
        if !ai_settings.is_memory_enabled(app) {
            col.add_child(self.render_disabled_banner(appearance));
        }

        let filtered_rules = self.get_filtered_rules();
        if filtered_rules.is_empty() {
            col.add_child(self.render_zero_state(appearance));
        } else {
            col.add_child(self.render_body(appearance, filtered_rules, app));
        };
        col.finish()
    }
}

impl TypedActionView for RuleView {
    type Action = ();

    fn handle_action(&mut self, _action: &(), _ctx: &mut ViewContext<Self>) {}
}
