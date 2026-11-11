use super::field::{FieldParser, ParsedField, parsed_controls};
use super::shared::disabled_look;
use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::Space;
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle, Length, cols};
use crate::reactive::Signal;
use crate::style::{
    BORDER_WIDTH, Color, ControlSize, Metrics, Paint, Radius, StyleAtom, Theme, Token,
};
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::{Edit, Layout, NodeEdit, Sealed, StatePaint, StateTarget, Widget};

/// A hex color input with a swatch; layout and `map_node` change the row, and style, state, and focus setters change the text field.
#[derive(Debug)]
#[must_use]
pub struct ColorFieldBuilder {
    value: Signal<Color>,
    field: ParsedField<HexParser>,
    layout_overrides: LayoutOverrides,
}

/// A color input.
pub fn color_field(value: Signal<Color>) -> ColorFieldBuilder {
    ColorFieldBuilder {
        value,
        field: ParsedField::new(value, HexParser),
        layout_overrides: LayoutOverrides::new(),
    }
}

parsed_controls!([] ColorFieldBuilder);

impl ColorFieldBuilder {
    fn into_node_builder(self) -> NodeBuilder {
        let disabled = self.field.disabled_flag();
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
            .child(disabled_look(swatch(self.value), disabled))
            .child(field)
    }
}

impl Sealed for ColorFieldBuilder {
    fn edit(mut self, NodeEdit(edit): NodeEdit) -> Self {
        match edit {
            Edit::Layout(edit) => self.layout_overrides.edit(edit),
            focus => self.field = self.field.map(move |field| field.edit(NodeEdit(focus))),
        }
        self
    }

    fn map_target(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.field = self.field.map(move |field| field.map_target(f));
        self
    }
}

impl StateTarget for ColorFieldBuilder {
    fn set_state(mut self, state: StatePaint) -> Self {
        self.field = self.field.map(move |field| field.set_state(state));
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

    fn parse(&self, raw: &str, _theme: &Theme) -> Option<Color> {
        let raw = raw.trim();
        (raw.starts_with('#') && raw.len() <= 7)
            .then(|| raw.parse().ok())
            .flatten()
    }

    fn render(&self, value: &Color, _theme: &Theme) -> String {
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
