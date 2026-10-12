use crate::animation_runtime::presets;
use crate::kernel::{AccessibleName, TextBuilder, TextContent, text};
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::{
    BORDER_WIDTH, Elevation, GHOST_VARIANT, NodeStyleExt, SOFT_VARIANT, SOLID_VARIANT, StyleAtom,
    StyleVariant, Tone, TransitionId,
};

const BUTTON_TRANSITION: TransitionId = presets::SMOOTH;

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

pub(super) static BUTTON_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];

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

pub(super) fn label_node(
    label: TextContent,
    tone: Tone,
    atoms: &'static [StyleAtom],
    compact: bool,
) -> TextBuilder {
    let node = text(label).nowrap().tone(tone).class(atoms);
    if compact { node.class("text-xs") } else { node }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ButtonFill {
    Solid,
    Soft,
    Ghost,
    Outline,
}

impl ButtonFill {
    pub(super) const fn variant(self) -> StyleVariant {
        match self {
            Self::Solid => SOLID_VARIANT,
            Self::Soft => SOFT_VARIANT,
            Self::Ghost | Self::Outline => GHOST_VARIANT,
        }
    }

    pub(super) const fn label_atoms(self) -> &'static [StyleAtom] {
        match self {
            Self::Solid => LABEL_TEXT_SOLID,
            Self::Soft | Self::Ghost | Self::Outline => LABEL_TEXT_SOFT,
        }
    }
}
