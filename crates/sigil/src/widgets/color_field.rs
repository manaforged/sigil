use super::field::{FieldParser, ParsedField};
use super::shared::disabled_look;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::Space;
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle, Length, cols};
use crate::reactive::Reactive;
use crate::reactive::Signal;
use crate::style::locale::Locale;
use crate::style::{
    BORDER_WIDTH, Color, ControlSize, Metrics, Paint, Radius, StateSlot, StyleAtom, Token,
};
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::{Edit, Layout, Sealed, StateTarget, Widget};
use smallvec::SmallVec;

/// A hex color input with a swatch; layout and `map_node` change the row, and style, state, and focus setters change the text field.
#[derive(Debug)]
pub struct ColorFieldBuilder {
    value: Signal<Color>,
    field: ParsedField<HexParser>,
    disabled: Option<Signal<bool>>,
    layout_overrides: LayoutOverrides,
}

/// A color input.
#[must_use]
pub fn color_field(value: Signal<Color>) -> ColorFieldBuilder {
    ColorFieldBuilder {
        value,
        field: ParsedField::new(value, HexParser),
        disabled: None,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl ColorFieldBuilder {
    /// Sets the disabled.
    #[must_use]
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        let disabled = disabled.into_signal();
        self.field = self.field.disabled(disabled);
        self.disabled = Some(disabled);
        self
    }

    /// Sets the invalid when.
    #[must_use]
    pub fn invalid_when(mut self, invalid: impl Reactive<bool>) -> Self {
        self.field = self.field.invalid_when(invalid);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let field = self.field.text_field().layout(
            LayoutStyle::flex_row()
                .grow(1.0)
                .min_size(0.0, Length::Auto)
                .align_items(AlignItems::Center),
        );
        container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .align_items(AlignItems::Center)
                    .gap(Space::Sm),
            )
            .child(disabled_look(swatch(self.value), self.disabled))
            .child(field)
    }
}

impl Sealed for ColorFieldBuilder {
    fn edit(mut self, edit: Edit) -> Self {
        match edit {
            Edit::Layout(edit) => self.layout_overrides.edit(edit),
            focus => self.field = self.field.map(move |field| field.edit(focus)),
        }
        self
    }

    fn map_target(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.field = self.field.map(move |field| field.map_target(f));
        self
    }
}

impl StateTarget for ColorFieldBuilder {
    fn set_state(mut self, slot: StateSlot, atoms: SmallVec<[StyleAtom; 16]>) -> Self {
        self.field = self.field.map(move |field| field.set_state(slot, atoms));
        self
    }
}

impl Widget for ColorFieldBuilder {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.layout_overrides.node_hook(f);
        self
    }
}

impl IntoNode for ColorFieldBuilder {
    fn build(mut self, cx: &mut Cx) -> NodeId {
        let overrides = std::mem::take(&mut self.layout_overrides);
        overrides.finish(self.into_node_builder()).build(cx)
    }
}

struct HexParser;

impl FieldParser for HexParser {
    type Value = Color;

    fn parse(&self, raw: &str, _locale: &Locale) -> Option<Color> {
        let raw = raw.trim();
        (raw.starts_with('#') && raw.len() <= 7)
            .then(|| raw.parse().ok())
            .flatten()
    }

    fn render(&self, value: &Color, _locale: &Locale) -> String {
        value.to_hex()
    }
}

fn swatch(value: Signal<Color>) -> NodeBuilder {
    let extent = Metrics::current().control_height(ControlSize::Default);
    let (width, chrome) =
        CellForm::new((Length::Px(extent), SWATCH_CHROME), (cols(2.0), NO_CHROME)).pick();
    container().children_reactive(move || {
        vec![
            container()
                .layout(LayoutStyle::block().width(width).height(extent))
                .class(chrome)
                .background(value.get()),
        ]
    })
}

static SWATCH_CHROME: &[StyleAtom] = &[
    StyleAtom::Radius(Radius::Md),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    Token::Gray6.border(),
];
static NO_CHROME: &[StyleAtom] = &[];

impl Color {
    /// Formats the color as `#rrggbb` hex text.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[cfg(test)]
#[path = "color_field_tests.rs"]
mod tests;
