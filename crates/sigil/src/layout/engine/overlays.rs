use crate::layout::Space;
use crate::style::{NumericAtomKind, StyleBinding, StyleEngine, TextStyle};

use crate::layout::LayoutStyle;
use crate::layout::style_adapter::apply_style_atoms;
use crate::layout::style_data::Length;
use crate::layout::text_style::{apply_text_binding, theme_for};

type Overlay = (NumericAtomKind, fn(LayoutStyle, f32) -> LayoutStyle);

const OVERLAYS: [Overlay; 5] = [
    (NumericAtomKind::Gap, |s, v| s.gap(Space::px(v))),
    (NumericAtomKind::PaddingX, |s, v| s.padding_x(Space::px(v))),
    (NumericAtomKind::PaddingY, |s, v| s.padding_y(Space::px(v))),
    (NumericAtomKind::Width, |s, v| s.width(v)),
    (NumericAtomKind::Height, |s, v| s.height(v)),
];

pub(super) fn apply_reactive_position(
    node: &crate::kernel::Node,
    intent: LayoutStyle,
) -> LayoutStyle {
    if node
        .extension::<crate::layout::ReactivePosition>()
        .is_none()
    {
        return intent;
    }
    intent
        .position(crate::layout::style_data::Position::Absolute)
        .inset_left(0.0)
        .inset_top(0.0)
}

pub(super) fn apply_style_overlay(
    node: &crate::kernel::Node,
    intent: &mut LayoutStyle,
    text_style: Option<&mut TextStyle>,
) {
    let Some(binding) = node.extension::<StyleBinding>() else {
        return;
    };
    for atom in binding.cascade() {
        apply_style_atoms(&[atom], intent);
    }
    if let Some(overlay) = binding.animator_overlay() {
        for (kind, apply) in OVERLAYS {
            if let Some(value) = overlay.value_for(kind) {
                *intent = apply(intent.clone(), value.max(0.0));
            }
        }
    }
    if let Some(style) = text_style {
        apply_text_binding(StyleEngine::new(theme_for(node)), binding, style);
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn apply_intrinsic_image_size(
    mut intent: LayoutStyle,
    width: u32,
    height: u32,
) -> LayoutStyle {
    if intent.data().size.width == Length::Auto {
        intent = intent.width(Length::Px(width as f32));
    }
    if intent.data().size.height == Length::Auto {
        intent = intent.height(Length::Px(height as f32));
    }
    intent
}
