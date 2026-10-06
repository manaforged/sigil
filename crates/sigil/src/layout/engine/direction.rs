use crate::layout::style_data::{
    AlignItems, Display, FlexDirection, JustifyContent, Position, Rect, StyleData,
};
use crate::style::locale::Direction;

pub(super) fn apply_direction(style: &mut StyleData, direction: Direction) {
    if direction.is_rtl() {
        mirror(style);
    }
}

fn mirror(style: &mut StyleData) {
    style.direction = Direction::Rtl;
    swap_sides(&mut style.padding);
    swap_sides(&mut style.margin);
    swap_sides(&mut style.border);
    if !matches!(style.position, Position::Absolute) {
        swap_sides(&mut style.inset);
    }
    if !matches!(style.display, Display::Flex) {
        return;
    }
    match style.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => {
            style.flex_direction = reverse_row(style.flex_direction);
            style.justify_content = Some(flip_justify(style.justify_content));
        }
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            style.align_items = style.align_items.map(flip_align);
        }
    }
}

const fn swap_sides<T: Copy>(rect: &mut Rect<T>) {
    core::mem::swap(&mut rect.left, &mut rect.right);
}

const fn reverse_row(direction: FlexDirection) -> FlexDirection {
    match direction {
        FlexDirection::Row => FlexDirection::RowReverse,
        FlexDirection::RowReverse => FlexDirection::Row,
        other => other,
    }
}

const fn flip_justify(justify: Option<JustifyContent>) -> JustifyContent {
    match justify {
        None | Some(JustifyContent::Start | JustifyContent::Stretch) => JustifyContent::End,
        Some(JustifyContent::End) => JustifyContent::Start,
        Some(other) => other,
    }
}

const fn flip_align(align: AlignItems) -> AlignItems {
    match align {
        AlignItems::Start => AlignItems::End,
        AlignItems::End => AlignItems::Start,
        other => other,
    }
}
