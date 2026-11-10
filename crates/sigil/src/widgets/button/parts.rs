use crate::animation::runtime::presets;
use crate::kernel::{TextBuilder, TextContent, text};
use crate::layout::Space;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle};
use crate::style::{
    BORDER_WIDTH, ColorAtom, Elevation, GHOST_VARIANT, SOFT_VARIANT, SOLID_VARIANT, StyleAtom,
    StyleVariant, Tone, TransitionId,
};
use crate::widgets::Widget;

const BUTTON_TRANSITION: TransitionId = presets::SMOOTH;

#[derive(Clone, Copy)]
pub(super) enum ButtonWidth {
    Hug,
    Fixed(f32),
    Fill,
}

pub(super) fn button_layout(width: ButtonWidth, height: f32, padding_x: f32) -> LayoutStyle {
    let layout = LayoutStyle::flex_row()
        .height(height)
        .padding_x(Space::px(padding_x))
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

const CLEAR_BORDER: StyleAtom =
    StyleAtom::BorderColor(ColorAtom::Rgba8(crate::scene::command::fills::TRANSPARENT));

pub(super) static GHOST_FRAME: &[StyleAtom] = &[StyleAtom::BorderWidth(BORDER_WIDTH), CLEAR_BORDER];

pub(super) static SOLID_FRAME: &[StyleAtom] = &[
    StyleAtom::BorderWidth(BORDER_WIDTH),
    CLEAR_BORDER,
    StyleAtom::Shadow(Elevation::Sm),
];

pub(super) static LABEL_TEXT_SOLID: &[StyleAtom] = &[
    StyleAtom::FgOnSolid,
    StyleAtom::FontWeight(500),
    StyleAtom::LetterSpacing(-10),
];
static LABEL_TEXT_SOLID_CELL: &[StyleAtom] = &[StyleAtom::FgOnSolid, StyleAtom::FontWeight(700)];
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
    let node = text(label).nowrap().tone_scope(tone).class(atoms);
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

    pub(super) fn label_atoms(self) -> &'static [StyleAtom] {
        match self {
            Self::Solid => {
                crate::layout::snap::CellForm::new(LABEL_TEXT_SOLID, LABEL_TEXT_SOLID_CELL).pick()
            }
            Self::Soft | Self::Ghost | Self::Outline => LABEL_TEXT_SOFT,
        }
    }
}
