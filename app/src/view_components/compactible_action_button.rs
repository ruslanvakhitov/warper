use std::sync::Arc;

use crate::{
    ui_components::icons::Icon,
    view_components::action_button::{
        ActionButton, ActionButtonTheme, ButtonSize, KeystrokeSource,
    },
    view_components::status_icons::icon_size,
};
use warp_core::ui::appearance::Appearance;
use warpui::{
    elements::{
        ChildView, ConstrainedBox, Container, CrossAxisAlignment, Flex, MainAxisAlignment,
        MainAxisSize, ParentElement,
    },
    Action, AppContext, Element, TypedActionView, View, ViewContext, ViewHandle,
};
const BUTTON_MARGIN: f32 = 8.;

// Size switch thresholds for responsive button behavior
pub const SMALL_SIZE_SWITCH_THRESHOLD: f32 = 400.0;

/// Stores normal and compact (i.e. without a keybinding display) versions of action buttons
/// for use in views that need to display buttons in different modes.
#[derive(Clone)]
pub struct CompactibleActionButton {
    compact_button: ViewHandle<ActionButton>,
    expanded_button: ViewHandle<ActionButton>,
}

pub trait RenderCompactibleActionButton {
    fn render_expanded_button(&self) -> Box<dyn Element>;
    fn render_compact_button(&self) -> Box<dyn Element>;
}

impl CompactibleActionButton {
    /// Creates a new button pair with compact and regular variants.
    pub fn new<T, A>(
        label: String,
        keybinding: Option<KeystrokeSource>,
        size: ButtonSize,
        action: A,
        compact_icon: Icon,
        theme: Arc<dyn ActionButtonTheme>,
        ctx: &mut ViewContext<'_, T>,
    ) -> Self
    where
        T: TypedActionView<Action = A> + View,
        A: Action + Clone + 'static,
    {
        let action_for_compact = action.clone();
        let compact_button = ctx.add_typed_action_view(|ctx| {
            let mut compact_button =
                ActionButton::new_with_boxed_theme(String::new(), Arc::clone(&theme))
                    .with_size(size)
                    .with_icon(compact_icon)
                    .with_tooltip(label.clone())
                    .on_click(move |ctx| ctx.dispatch_typed_action(action_for_compact.clone()));

            if let Some(ref kb) = keybinding {
                if let Some(tooltip_sublabel) = kb.displayed(ctx) {
                    compact_button = compact_button.with_tooltip_sublabel(tooltip_sublabel);
                }
            }

            compact_button
        });

        let expanded_button = ctx.add_typed_action_view(move |ctx| {
            let mut button = ActionButton::new_with_boxed_theme(label.clone(), theme)
                .with_size(size)
                .on_click(move |ctx| ctx.dispatch_typed_action(action.clone()));

            if let Some(kb) = keybinding {
                button = button.with_keybinding(kb, ctx);
            }

            button
        });

        Self {
            compact_button,
            expanded_button,
        }
    }

    pub fn compact_button(&self) -> &ViewHandle<ActionButton> {
        &self.compact_button
    }

    pub fn expanded_button(&self) -> &ViewHandle<ActionButton> {
        &self.expanded_button
    }
}

impl RenderCompactibleActionButton for CompactibleActionButton {
    fn render_expanded_button(&self) -> Box<dyn Element> {
        ChildView::new(self.expanded_button()).finish()
    }
    fn render_compact_button(&self) -> Box<dyn Element> {
        ChildView::new(self.compact_button()).finish()
    }
}

/// Renders compact and expanded rows for a caller to switch between.
pub fn render_compact_and_regular_button_rows(
    buttons: Vec<&dyn RenderCompactibleActionButton>,
) -> (Box<dyn Element>, Box<dyn Element>) {
    let (full_buttons, compact_buttons) = buttons
        .iter()
        .map(|button| {
            (
                button.render_expanded_button(),
                button.render_compact_button(),
            )
        })
        .unzip();

    let full_row = render_button_row(full_buttons);
    let compact_row = render_button_row(compact_buttons);

    (full_row.finish(), compact_row.finish())
}

fn render_button_row(buttons: Vec<Box<dyn Element>>) -> Flex {
    let mut row = Flex::row()
        .with_main_axis_alignment(MainAxisAlignment::End)
        .with_cross_axis_alignment(CrossAxisAlignment::Center)
        .with_main_axis_size(MainAxisSize::Min);

    for (index, element) in buttons.into_iter().enumerate() {
        let mut container = Container::new(element);
        if index != 0 {
            container = container.with_margin_left(BUTTON_MARGIN);
        }
        row.add_child(container.finish());
    }

    row
}

pub fn render_expansion_icon(
    expanded: bool,
    appearance: &Appearance,
    app: &AppContext,
) -> Box<dyn Element> {
    ConstrainedBox::new(
        warpui::elements::Icon::new(
            if expanded {
                Icon::ChevronDown.into()
            } else {
                Icon::ChevronRight.into()
            },
            appearance.theme().foreground(),
        )
        .finish(),
    )
    .with_width(icon_size(app))
    .with_height(icon_size(app))
    .finish()
}
