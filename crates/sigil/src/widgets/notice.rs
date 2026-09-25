use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::prelude::{LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::layout::snap::CellForm;
use crate::style::{BORDER_WIDTH, NodeStyleExt, Radius, StyleAtom, Tone, space};

static NOTICE_BAND: &[StyleAtom] = &[
    Radius::Md.atom(),
    StyleAtom::BgToneStep(3),
    StyleAtom::BorderWidth(BORDER_WIDTH),
    StyleAtom::BorderToneStep(6),
];
static NOTICE_CELL: &[StyleAtom] = &[StyleAtom::BgToneStep(3)];
static NOTICE_TEXT: &[StyleAtom] = &[StyleAtom::FgToneStep(11)];

#[must_use]
pub fn notice_band(body: impl Into<TextContent>, tone: Tone) -> NodeBuilder {
    let (role, name) = match tone {
        Tone::Error => (Role::Alert, "Error"),
        Tone::Warn => (Role::Alert, "Warning"),
        Tone::Success => (Role::Status, "Success"),
        Tone::Info | Tone::Primary | Tone::Neutral => (Role::Status, "Notice"),
    };
    let band = container().role(role).label(name).tone(tone);
    let band = CellForm::<fn(NodeBuilder, Tone) -> NodeBuilder>::new(window_band, cell_band).pick()(
        band, tone,
    );
    band.child(text(body).ellipsize().tone(tone).class(NOTICE_TEXT))
}

fn window_band(band: NodeBuilder, _tone: Tone) -> NodeBuilder {
    band.layout(
        LayoutStyle::flex_row()
            .fill_width()
            .padding_x(space(3))
            .padding_y(space(2)),
    )
    .class(NOTICE_BAND)
}

fn cell_band(band: NodeBuilder, tone: Tone) -> NodeBuilder {
    let marker = match tone {
        Tone::Error => "✗",
        Tone::Warn => "!",
        Tone::Success => "✓",
        Tone::Info | Tone::Primary | Tone::Neutral => "▌",
    };
    band.layout(
        LayoutStyle::flex_row()
            .fill_width()
            .height(rows(1.0))
            .gap(cols(1.0)),
    )
    .class(NOTICE_CELL)
    .child(text(marker).tone(tone).class(NOTICE_TEXT))
}
