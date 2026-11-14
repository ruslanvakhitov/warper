use pathfinder_geometry::vector::vec2f;
use warpui::{
    elements::{
        ChildAnchor, ConstrainedBox, Container, Empty, OffsetPositioning, ParentAnchor,
        ParentElement as _, ParentOffsetBounds, Stack,
    },
    ui_components::components::UiComponentStyles,
    Element,
};

pub struct RedNotificationDot {}

impl RedNotificationDot {
    fn render_internal(styles: &UiComponentStyles) -> Box<dyn Element> {
        let width = styles.width.expect("RedNotificationDot requires width");
        let height = styles.height.expect("RedNotificationDot requires height");

        let status_constrained_box = ConstrainedBox::new(Empty::new().finish())
            .with_height(height)
            .with_width(width)
            .finish();

        let mut status_element = Container::new(status_constrained_box);

        if let Some(corner) = styles.border_radius {
            status_element = status_element.with_corner_radius(corner);
        }

        if let Some(background) = styles.background {
            status_element = status_element.with_background(background);
        }
        status_element.finish()
    }

    pub fn render_with_offset(
        element: Box<dyn Element>,
        styles: &UiComponentStyles,
        (x_delta, y_delta): (f32, f32),
    ) -> Box<dyn Element> {
        let width = styles.width.expect("RedNotificationDot requires width");
        let height = styles.height.expect("RedNotificationDot requires height");

        let x_axis_offset = width / 2.;
        let y_axis_offset = -(height / 2.);

        let mut stack = Stack::new().with_child(element);

        stack.add_positioned_child(
            RedNotificationDot::render_internal(styles),
            OffsetPositioning::offset_from_parent(
                vec2f(x_axis_offset + x_delta, y_axis_offset + y_delta),
                ParentOffsetBounds::Unbounded,
                ParentAnchor::TopRight,
                ChildAnchor::TopRight,
            ),
        );
        stack.finish()
    }
}
