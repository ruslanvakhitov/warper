//! User-invoked command snippets for terminal setup and CLI-agent instructions.
use crate::{
    appearance::Appearance,
    ui_components::{blended_colors, buttons::icon_button, icons::Icon},
};
use std::iter;
use warpui::{
    elements::{
        Border, Container, CornerRadius, CrossAxisAlignment, Empty, Flex, HighlightedRange,
        MainAxisAlignment, MainAxisSize, MouseStateHandle, ParentElement, Radius, Text,
    },
    ui_components::components::UiComponent,
    AppContext, Element, EventContext, SingletonEntity,
};
const CODE_BLOCK_CORNER_RADIUS: f32 = 8.;
const INLINE_ACTION_HEADER_VERTICAL_PADDING: f32 = 10.;
const INLINE_ACTION_HORIZONTAL_PADDING: f32 = 16.;
#[derive(Default, Clone)]
pub struct CodeSnippetButtonHandles {
    pub copy_button: MouseStateHandle,
    pub insert_button: MouseStateHandle,
}
pub type HandleCode = Box<dyn FnMut(String, &mut EventContext)>;
pub struct CodeBlockOptions {
    pub on_execute: Option<HandleCode>,
    pub on_copy: Option<HandleCode>,
    pub footer_element: Option<Box<dyn Element>>,
    pub mouse_handles: Option<CodeSnippetButtonHandles>,
}
fn render_button<F>(
    appearance: &Appearance,
    icon: Icon,
    tooltip_text: &str,
    mouse_handle: MouseStateHandle,
    formatted_text: String,
    on_click: F,
) -> Container
where
    F: FnMut(String, &mut EventContext) + 'static,
{
    let ui_builder = appearance.ui_builder().clone();
    let tooltip_text = tooltip_text.to_owned();
    let mut on_click = on_click;
    let button_element = icon_button(appearance, icon, false, mouse_handle);

    Container::new(
        button_element
            .with_tooltip(move || ui_builder.tool_tip(tooltip_text.clone()).build().finish())
            .build()
            .on_click(move |ctx, _, _| {
                on_click(formatted_text.clone(), ctx);
            })
            .finish(),
    )
}
pub fn render_code_block_plain(
    code: &str,
    find_highlight_ranges: impl Iterator<Item = HighlightedRange>,
    options: CodeBlockOptions,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();

    let code_element = Text::new(
        code.to_owned(),
        appearance.monospace_font_family(),
        appearance.monospace_font_size(),
    )
    .with_color(blended_colors::text_main(theme, theme.surface_1()))
    .with_highlights(find_highlight_ranges)
    .with_selectable(true)
    .finish();

    let content = render_plain_code_block_internal(
        code,
        code_element,
        options.footer_element,
        options.on_execute,
        options.on_copy,
        options.mouse_handles,
        appearance,
        false,
    );
    Container::new(content.finish())
        .with_border(Border::all(1.).with_border_fill(theme.outline()))
        .with_corner_radius(CornerRadius::with_all(Radius::Pixels(
            CODE_BLOCK_CORNER_RADIUS,
        )))
        .finish()
}
pub fn render_runnable_code_snippet(
    code_snippet: &str,
    language: Option<&str>,
    on_execute: Option<HandleCode>,
    on_copy: Option<HandleCode>,
    mouse_handles: Option<CodeSnippetButtonHandles>,
    app: &AppContext,
) -> Box<dyn Element> {
    let appearance = Appearance::as_ref(app);
    let theme = appearance.theme();
    let language_text = language.map(|language| {
        Text::new_inline(
            language.to_owned(),
            appearance.monospace_font_family(),
            appearance.monospace_font_size(),
        )
        .with_color(blended_colors::text_sub(theme, theme.surface_3()))
        .finish()
    });
    render_code_block_plain(
        code_snippet,
        Box::new(iter::empty()),
        CodeBlockOptions {
            on_execute,
            on_copy,
            footer_element: language_text,
            mouse_handles,
        },
        app,
    )
}
#[allow(clippy::too_many_arguments)]
fn render_plain_code_block_internal(
    code: &str,
    code_element: Box<dyn Element>,
    footer_element: Option<Box<dyn Element>>,
    on_execute: Option<HandleCode>,
    on_copy: Option<HandleCode>,
    mouse_handles: Option<CodeSnippetButtonHandles>,
    appearance: &Appearance,
    without_extra_padding_between_code_and_footer: bool,
) -> Flex {
    let theme = appearance.theme();
    let mut content = Flex::column().with_cross_axis_alignment(CrossAxisAlignment::Stretch);

    let mut footer_row = Flex::row()
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
        .with_main_axis_size(MainAxisSize::Max)
        .with_child(footer_element.unwrap_or_else(|| Empty::new().finish()));

    if let Some(mouse_handles) = mouse_handles {
        let mut action_row = Flex::row().with_cross_axis_alignment(CrossAxisAlignment::Center);
        let code_clone = code.to_owned();

        if let Some(on_copy) = on_copy {
            let copy_button = render_button(
                appearance,
                Icon::Copy,
                "Copy",
                mouse_handles.copy_button,
                code_clone.clone(),
                on_copy,
            )
            .finish();

            action_row.add_child(copy_button);
        }

        if let Some(on_execute) = on_execute {
            let insert_button = render_button(
                appearance,
                Icon::TerminalInput,
                "Run in terminal",
                mouse_handles.insert_button,
                code_clone.clone(),
                on_execute,
            )
            .with_margin_left(8.)
            .finish();

            action_row.add_child(insert_button);
        }

        footer_row.add_child(action_row.finish());
    }

    let code_padding_bottom = if without_extra_padding_between_code_and_footer {
        0.
    } else {
        INLINE_ACTION_HEADER_VERTICAL_PADDING
    };
    let footer_padding_top = if without_extra_padding_between_code_and_footer {
        0.
    } else {
        6.
    };

    content.add_child(
        Container::new(code_element)
            .with_background(theme.surface_2())
            .with_padding_top(INLINE_ACTION_HEADER_VERTICAL_PADDING)
            .with_padding_bottom(code_padding_bottom)
            .with_horizontal_padding(INLINE_ACTION_HORIZONTAL_PADDING)
            .with_corner_radius(CornerRadius::with_top(Radius::Pixels(
                CODE_BLOCK_CORNER_RADIUS,
            )))
            .finish(),
    );

    content.add_child(
        Container::new(
            footer_row
                .with_main_axis_size(MainAxisSize::Max)
                .with_main_axis_alignment(MainAxisAlignment::SpaceBetween)
                .finish(),
        )
        .with_background(theme.surface_2())
        .with_padding_top(footer_padding_top)
        .with_padding_bottom(12.)
        .with_horizontal_padding(INLINE_ACTION_HORIZONTAL_PADDING)
        .with_corner_radius(CornerRadius::with_bottom(Radius::Pixels(
            CODE_BLOCK_CORNER_RADIUS,
        )))
        .finish(),
    );

    content
}
