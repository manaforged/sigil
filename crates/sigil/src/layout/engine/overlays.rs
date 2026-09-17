use crate::style::{NumericAtomKind, StyleBinding, StyleEngine, TextStyle};

use crate::layout::LayoutStyle;
use crate::layout::style_adapter::apply_style_atoms;
use crate::layout::style_data::Dimension;
use crate::layout::text_style::{apply_text_binding, theme_for};

pub(super) fn apply_reactive_position(
    node: &crate::kernel::Node,
    intent: LayoutStyle,
) -> LayoutStyle {
    let Some(pos) = node.get_extension::<crate::layout::ReactivePosition>() else {
        return intent;
    };

    let (x, y) = pos.get();

    intent
        .position(crate::layout::style_data::Position::Absolute)
        .inset_left(x)
        .inset_top(y)
}

pub(super) fn apply_style_overlay(
    node: &crate::kernel::Node,

    intent: &mut LayoutStyle,

    text_style: Option<&mut TextStyle>,
) {
    let Some(binding) = node.get_extension::<StyleBinding>() else {
        return;
    };

    let pad = intent.data().padding;

    let keep_pad = pad
        != crate::layout::style_data::Rect::splat(crate::layout::style_data::LengthPercentage::Px(
            0.0,
        ));

    apply_style_atoms(binding.static_slice(), intent);

    apply_style_atoms(binding.dynamic_slice(), intent);

    for reactive in binding.reactive_channels() {
        apply_style_atoms(reactive, intent);
    }

    if keep_pad {
        intent.data_mut().padding = pad;
    }

    apply_style_atoms(binding.state_overlay_atoms(), intent);
    if let Some(overlay) = binding.animator_overlay() {
        if let Some(value) = overlay.value_for(NumericAtomKind::Gap) {
            *intent = intent.clone().gap(value.max(0.0));
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::PaddingX) {
            *intent = intent.clone().padding_x(value.max(0.0));
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::PaddingY) {
            *intent = intent.clone().padding_y(value.max(0.0));
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::Width) {
            *intent = intent.clone().width(value.max(0.0));
        }
        if let Some(value) = overlay.value_for(NumericAtomKind::Height) {
            *intent = intent.clone().height(value.max(0.0));
        }
    }

    if let Some(style) = text_style {
        let engine = StyleEngine::new(theme_for(node));

        apply_text_binding(engine, binding, style);
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
pub(super) fn apply_intrinsic_image_size(
    intent: LayoutStyle,
    width: u32,
    height: u32,
) -> LayoutStyle {
    let mut intent = intent;

    if intent.data().size.width == Dimension::Auto {
        intent = intent.width(Dimension::Px(width as f32));
    }

    if intent.data().size.height == Dimension::Auto {
        intent = intent.height(Dimension::Px(height as f32));
    }

    intent
}
