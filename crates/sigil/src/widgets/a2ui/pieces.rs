use serde_json::Value;

use crate::kernel::{NodeBuilder, TextContent, container, text};
use crate::layout::{AlignItems, LayoutStyle, Space};
use crate::style::{Token, Tone};
use crate::widgets::{Widget, chart, diff, list_row, markdown, mermaid, stat};

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
            } => {
                let (render, plot, names, data) =
                    (self.clone(), *plot, labels.clone(), series.clone());
                self.reactive(&[labels, series], move || {
                    Self::chart(plot, &render.tracked(&names), &render.tracked(&data))
                })
            }
            Piece::Stat {
                label,
                value,
                tone: mood,
            } => {
                let tile = stat(self.live(label), self.live(value));
                match tone(*mood) {
                    Some(shade) => tile.tone(shade).into(),
                    None => tile.into(),
                }
            }
            Piece::Table { columns, rows } => {
                let (render, heads, cells) = (self.clone(), columns.clone(), rows.clone());
                self.reactive(&[columns, rows], move || {
                    render.table(&render.tracked(&heads), &render.tracked(&cells))
                })
            }
            Piece::Markdown { text: body } => markdown(self.source(body)),
            Piece::Mermaid { source } => mermaid(self.source(source)),
            Piece::Diff { patch } => diff(self.source(patch)).into(),
            Piece::Item(item) => self.item(item),
        }
    }

    fn reactive(
        &self,
        values: &[&Dynamic],
        build: impl Fn() -> NodeBuilder + 'static,
    ) -> NodeBuilder {
        if values.iter().all(|value| self.bound(value).is_none()) {
            return build();
        }
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .children_reactive(move || [build()])
    }

    fn chart(plot: Plot, labels: &Value, series: &Value) -> NodeBuilder {
        let mut rows = items(series).iter();
        let Some(first) = rows.next() else {
            return super::render::note("No data");
        };
        let labels: Vec<String> = items(labels).iter().map(display).collect();
        let name = |row: &Value| row.get("name").map(display).unwrap_or_default();
        let values = |row: &Value| numbers(row.get("values").unwrap_or(&Value::Null));
        let mut drawn = chart(values(first)).labels(labels).tooltip();
        for (index, row) in rows.enumerate() {
            let shade = SERIES_TONES
                .get((index + 1) % SERIES_TONES.len())
                .copied()
                .unwrap_or(Tone::Primary);
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

    fn table(&self, columns: &Value, rows: &Value) -> NodeBuilder {
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
        items(rows).iter().fold(
            container()
                .layout(LayoutStyle::flex_column().fill_width().gap(Space::Sm))
                .child(line(items(columns), true)),
            |table, cells| table.child(line(items(cells), false)),
        )
    }

    fn item(&self, item: &Item) -> NodeBuilder {
        let optional = |value: &Dynamic| -> Option<TextContent> {
            let shown = self.shown(value);
            (!shown.is_empty()).then(|| self.live(value))
        };
        let mut row = list_row(self.live(&item.title)).depth(item.depth);
        if let Some(marker) = optional(&item.marker) {
            row = row.marker(marker);
        }
        if let Some(status) = optional(&item.status) {
            row = row.status(status);
        }
        if let Some(tone) = tone(item.tone) {
            row = row.status_tone(tone);
        }
        if let Some(detail) = optional(&item.detail) {
            row = row.detail(detail);
        }
        if let Some(run) = item.event.as_ref().and_then(|event| self.trigger(event)) {
            row = row.activate(move || run());
        }
        row.into_node_builder()
    }
}
