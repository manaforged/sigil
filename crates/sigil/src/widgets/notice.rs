use crate::kernel::{AccessibleName, NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{LayoutBuilderExt, LayoutStyle, cols, rows};
use crate::reactive::derived;
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
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
        Tone::Error => (Role::Alert, StringKey::NoticeError),
        Tone::Warn => (Role::Alert, StringKey::NoticeWarning),
        Tone::Success => (Role::Status, StringKey::NoticeSuccess),
        Tone::Info | Tone::Primary | Tone::Neutral => (Role::Status, StringKey::Notice),
    };
    let body: TextContent = body.into();
    let band = container()
        .role(role)
        .label(announced(name, &body))
        .tone(tone);
    let band = CellForm::<fn(NodeBuilder, Tone) -> NodeBuilder>::new(
        |node, _| {
            node.layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .padding_x(space(3))
                    .padding_y(space(2)),
            )
            .class(NOTICE_BAND)
        },
        cell_band,
    )
    .pick()(band, tone);
    band.child(text(body).ellipsize().tone(tone).class(NOTICE_TEXT))
}

fn announced(key: StringKey, body: &TextContent) -> AccessibleName {
    let name = theme_label(key);
    match body {
        TextContent::Static(text) => {
            let text = text.clone();
            derived(move || format!("{}: {text}", name.get())).into()
        }
        TextContent::Reactive(text) => {
            let text = *text;
            derived(move || format!("{}: {}", name.get(), text.get())).into()
        }
        TextContent::Compute(text) => {
            let text = text.clone();
            derived(move || format!("{}: {}", name.get(), text())).into()
        }
    }
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

#[cfg(test)]
#[path = "notice_tests.rs"]
mod tests;
