use sigil_animation_runtime::presets;
use sigil_kernel::{AccessibleName, TextContent};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutStyle};
use sigil_style::{Elevation, Px, StyleAtom, TransitionId};

pub(super) const BUTTON_TRANSITION: TransitionId = presets::SMOOTH;

#[derive(Clone, Copy)]
pub(super) enum ButtonWidth {
    Hug,
    Fixed(f32),
    Fill,
}

pub(super) fn accessible_label(label: &TextContent) -> AccessibleName {
    match label {
        TextContent::Static(value) => AccessibleName::Static(value.clone()),
        TextContent::Reactive(value) => AccessibleName::Derived(*value),
        TextContent::Compute(_) => AccessibleName::Static(label.resolve()),
    }
}

pub(super) fn button_layout(width: ButtonWidth, height: f32, padding_x: f32) -> LayoutStyle {
    let layout = LayoutStyle::flex_row()
        .height(height)
        .padding_x(padding_x)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center);
    match width {
        ButtonWidth::Hug => layout.shrink(0.0),
        ButtonWidth::Fixed(px) => layout.width(px).shrink(0.0),
        ButtonWidth::Fill => layout.fill_width(),
    }
}

pub(super) static BUTTON_TRANSITION_ATOMS: &[StyleAtom] =
    &[StyleAtom::Transition(BUTTON_TRANSITION)];

pub(super) static BUTTON_FRAME: &[StyleAtom] =
    &[StyleAtom::BorderWidth(Px(1)), StyleAtom::BorderToneStep(6)];

pub(super) static SOLID_FRAME: &[StyleAtom] = &[StyleAtom::Shadow(Elevation::Sm)];

pub(super) static LABEL_TEXT_SOLID: &[StyleAtom] = &[
    StyleAtom::FgOnSolid,
    StyleAtom::FontWeight(500),
    StyleAtom::LetterSpacing(-10),
];
pub(super) static LABEL_TEXT_SOFT: &[StyleAtom] = &[
    StyleAtom::FgToneStep(11),
    StyleAtom::FontWeight(500),
    StyleAtom::LetterSpacing(-10),
];
