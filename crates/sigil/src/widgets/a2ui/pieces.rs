use serde_json::Value;

use crate::kernel::{NodeBuilder, TextContent, container, text};
use crate::layout::{AlignItems, LayoutStyle, Space};
use crate::reactive::signal;
use crate::style::{Token, Tone};
use crate::widgets::{Layout, ListRowSlots, chart, diff, list_row, markdown, mermaid, stat};

use super::model::display;
use super::render::Render;
use super::spec::{Dynamic, Item, Mood, Piece, Plot};

const SERIES_TONES: [Tone; 4] = [Tone::Primary, Tone::Success, Tone::Warning, Tone::Error];

const fn tone(mood: Mood) -> Option<Tone> {
    match mood {
        Mood::Neutral => None,
        Mood::Success => Some(Tone::Success),
        Mood::Warning => Some(Tone::Warning),
        Mood::Danger => Some(Tone::Error),
    }
}

fn items(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn numbers(value: &Value) -> Vec<f32> {
    #[expect(clippy::cast_possible_truncation, reason = "chart values fit f32")]
    items(value)
        .iter()
        .filter_map(Value::as_f64)
        .map(|n| n as f32)
        .collect()
}

impl Render {
    pub(super) fn piece(&self, piece: &Piece) -> NodeBuilder {
        match piece {
            Piece::Chart {
                plot,
                labels,
                series,
            } => self.chart(*plot, labels, series),
            Piece::Stat { label, value, tone: mood } => {
                let tile = stat(self.live(label), self.live(value));
                match tone(*mood) {
                    Some(shade) => tile.tone(shade).into(),
                    None => tile.into(),
                }
            }
            Piece::Table { columns, rows } => self.table(columns, rows),
            Piece::Markdown { text: body } => markdown(signal(self.shown(body))),
            Piece::Mermaid { source } => mermaid(signal(self.shown(source))),
            Piece::Diff { patch } => diff(signal(self.shown(patch))).into(),
            Piece::Item(item) => self.item(item),
        }
    }

    fn chart(&self, plot: Plot, labels: &Dynamic, series: &Dynamic) -> NodeBuilder {
        let series = self.value(series);
        let mut rows = items(&series).iter();
        let Some(first) = rows.next() else {
            return super::render::note("No data");
        };
        let labels: Vec<String> = items(&self.value(labels)).iter().map(display).collect();
        let name = |row: &Value| row.get("name").map(display).unwrap_or_default();
        let values = |row: &Value| numbers(row.get("values").unwrap_or(&Value::Null));
        let mut drawn = chart(values(first)).labels(labels).tooltip();
        for (index, row) in rows.enumerate() {
            let shade = SERIES_TONES[(index + 1) % SERIES_TONES.len()];
            drawn = drawn.series(name(row), values(row), shade);
        }
        let drawn = match plot {
            Plot::Bar => drawn,
            Plot::Line => drawn.line(),
            Plot::Area => drawn.area(),
            Plot::Pie => drawn.pie(),
        };
        drawn.into()
    }

    fn table(&self, columns: &Dynamic, rows: &Dynamic) -> NodeBuilder {
        let line = |cells: &[Value], header: bool| {
            let (size, shade) = if header {
                ("text-xs", Token::OnSurfaceMuted)
            } else {
                ("text-sm", Token::OnSurface)
            };
            cells.iter().fold(
                container().layout(
                    LayoutStyle::flex_row()
                        .fill_width()
                        .gap(Space::Md)
                        .align_items(AlignItems::Center),
                ),
                |row, cell| {
                    let shown = display(cell);
                    self.budget.spend(shown.len());
                    row.child(
                        container()
                            .layout(LayoutStyle::block().grow(1.0).width(0.0).min_size(0.0, 0.0))
                            .child(text(shown).ellipsize().class(size).class(shade.text())),
                    )
                },
            )
        };
        let rows = self.value(rows);
        items(&rows).iter().fold(
            container()
                .layout(LayoutStyle::flex_column().fill_width().gap(Space::Sm))
                .child(line(items(&self.value(columns)), true)),
            |table, cells| table.child(line(items(cells), false)),
        )
    }

    fn item(&self, item: &Item) -> NodeBuilder {
        let optional = |value: &Dynamic| -> Option<TextContent> {
            let shown = self.shown(value);
            (!shown.is_empty()).then(|| self.live(value))
        };
        list_row(
            false,
            self.live(&item.title),
            ListRowSlots {
                depth: item.depth,
                marker: optional(&item.marker),
                status: optional(&item.status),
                status_tone: tone(item.tone),
                detail: optional(&item.detail),
                activate: item.event.as_ref().and_then(|event| self.trigger(event)),
                ..ListRowSlots::default()
            },
        )
        .into()
    }
}
