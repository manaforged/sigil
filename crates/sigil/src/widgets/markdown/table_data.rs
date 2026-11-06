use std::cmp::Ordering;

use pulldown_cmark::Alignment;

use crate::kernel::SortDirection;

use super::parse::Inline;

pub(crate) type Cell = Vec<Inline>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Align {
    #[default]
    Start,
    Center,
    End,
}

impl Align {
    pub(super) const fn from_delimiter(align: Alignment) -> Self {
        match align {
            Alignment::None | Alignment::Left => Self::Start,
            Alignment::Center => Self::Center,
            Alignment::Right => Self::End,
        }
    }

    const fn marker(self) -> &'static str {
        match self {
            Self::Start => "---",
            Self::Center => ":---:",
            Self::End => "---:",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Table {
    pub(crate) align: Vec<Align>,
    pub(crate) rows: Vec<Vec<Cell>>,
}

impl FromIterator<Vec<Cell>> for Table {
    fn from_iter<I: IntoIterator<Item = Vec<Cell>>>(rows: I) -> Self {
        Self {
            align: Vec::new(),
            rows: rows.into_iter().collect(),
        }
    }
}

impl Table {
    pub(super) fn align_at(&self, column: usize) -> Align {
        self.align.get(column).copied().unwrap_or_default()
    }

    pub(super) fn columns(&self) -> usize {
        self.rows.iter().map(Vec::len).max().unwrap_or(0)
    }

    pub(super) fn order(&self, sort: Option<(usize, SortDirection)>) -> Vec<usize> {
        let mut order: Vec<usize> = (1..self.rows.len()).collect();
        let Some((column, direction)) = sort else {
            return order;
        };
        let keys: Vec<String> = self
            .rows
            .iter()
            .map(|row| row.get(column).map(|cell| Inline::plain(cell)).unwrap_or_default())
            .collect();
        let numeric = order
            .iter()
            .filter_map(|&row| keys.get(row))
            .filter(|key| !key.trim().is_empty())
            .all(|key| number(key).is_some());
        order.sort_by(|&a, &b| {
            let (a, b) = (
                keys.get(a).map_or("", String::as_str),
                keys.get(b).map_or("", String::as_str),
            );
            let ordering = if numeric {
                compare_numbers(a, b)
            } else {
                a.to_lowercase().cmp(&b.to_lowercase())
            };
            match direction {
                SortDirection::Descending => ordering.reverse(),
                SortDirection::Ascending => ordering,
            }
        });
        order
    }

    fn shown<'a>(&'a self, order: &'a [usize]) -> impl Iterator<Item = &'a Vec<Cell>> + 'a {
        let body = order.iter().filter_map(|&at| self.rows.get(at));
        self.rows.first().into_iter().chain(body)
    }

    pub(super) fn markdown(&self, order: &[usize]) -> String {
        let columns = self.columns();
        let line = |cells: Vec<String>| format!("| {} |\n", cells.join(" | "));
        let mut out = String::new();
        for (index, row) in self.shown(order).enumerate() {
            out.push_str(&line(
                (0..columns)
                    .map(|at| row.get(at).map(|cell| markdown_cell(cell)).unwrap_or_default())
                    .collect(),
            ));
            if index == 0 {
                out.push_str(&line(
                    (0..columns)
                        .map(|at| self.align_at(at).marker().to_string())
                        .collect(),
                ));
            }
        }
        out
    }

    pub(super) fn csv(&self, order: &[usize]) -> String {
        let columns = self.columns();
        let mut out = String::new();
        for row in self.shown(order) {
            let cells: Vec<String> = (0..columns)
                .map(|at| {
                    row.get(at)
                        .map(|cell| csv_field(&Inline::plain(cell)))
                        .unwrap_or_default()
                })
                .collect();
            out.push_str(&cells.join(","));
            out.push('\n');
        }
        out
    }
}

fn number(key: &str) -> Option<f64> {
    let cleaned: String = key
        .trim()
        .chars()
        .filter(|ch| !matches!(ch, ',' | '_' | '$' | '%' | '€' | '£' | ' '))
        .collect();
    cleaned.parse::<f64>().ok().filter(|value| value.is_finite())
}

fn compare_numbers(a: &str, b: &str) -> Ordering {
    match (number(a), number(b)) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn markdown_cell(cell: &[Inline]) -> String {
    let mut out = String::new();
    write_markdown(&mut out, cell);
    out.replace('|', "\\|").replace('\n', " ")
}

fn write_markdown(out: &mut String, body: &[Inline]) {
    for inline in body {
        match inline {
            Inline::Text(text) => out.push_str(text),
            Inline::Code(code) => out.push_str(&format!("`{code}`")),
            Inline::Math(expr) => out.push_str(&format!("${expr}$")),
            Inline::Note(label) => out.push_str(&format!("[^{label}]")),
            Inline::Image { alt, src } => out.push_str(&format!("![{alt}]({src})")),
            Inline::Strong(inner) => wrap(out, "**", inner),
            Inline::Em(inner) => wrap(out, "*", inner),
            Inline::Strike(inner) => wrap(out, "~~", inner),
            Inline::Link { href, body, .. } => {
                out.push('[');
                write_markdown(out, body);
                out.push_str(&format!("]({href})"));
            }
        }
    }
}

fn wrap(out: &mut String, marker: &str, inner: &[Inline]) {
    out.push_str(marker);
    write_markdown(out, inner);
    out.push_str(marker);
}
