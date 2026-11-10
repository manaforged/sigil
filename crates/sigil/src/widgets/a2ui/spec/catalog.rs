use serde_json::{Map, Value};

use super::names::pick;
use super::{Dynamic, Event, Kind, event, number};

#[derive(Clone, Copy)]
pub(in crate::widgets::a2ui) enum Plot {
    Bar,
    Line,
    Area,
    Pie,
}

#[derive(Clone, Copy)]
pub(in crate::widgets::a2ui) enum Mood {
    Neutral,
    Success,
    Warning,
    Danger,
}

const PLOTS: &[(&str, Plot)] = &[
    ("bar", Plot::Bar),
    ("line", Plot::Line),
    ("area", Plot::Area),
    ("pie", Plot::Pie),
];

const MOODS: &[(&str, Mood)] = &[
    ("neutral", Mood::Neutral),
    ("success", Mood::Success),
    ("warning", Mood::Warning),
    ("danger", Mood::Danger),
];

pub(in crate::widgets::a2ui) enum Piece {
    Chart {
        plot: Plot,
        labels: Dynamic,
        series: Dynamic,
    },
    Stat {
        label: Dynamic,
        value: Dynamic,
        tone: Mood,
    },
    Table {
        columns: Dynamic,
        rows: Dynamic,
    },
    Markdown {
        text: Dynamic,
    },
    Mermaid {
        source: Dynamic,
    },
    Diff {
        patch: Dynamic,
    },
    Item(Item),
}

pub(in crate::widgets::a2ui) struct Item {
    pub(in crate::widgets::a2ui) title: Dynamic,
    pub(in crate::widgets::a2ui) status: Dynamic,
    pub(in crate::widgets::a2ui) detail: Dynamic,
    pub(in crate::widgets::a2ui) marker: Dynamic,
    pub(in crate::widgets::a2ui) tone: Mood,
    pub(in crate::widgets::a2ui) depth: u16,
    pub(in crate::widgets::a2ui) event: Option<Event>,
}

pub(super) fn parse(name: &str, props: &Map<String, Value>) -> Option<Kind> {
    let tone = || pick(props.get("tone"), MOODS, Mood::Neutral);
    let piece = match name {
        "Chart" => Piece::Chart {
            plot: pick(props.get("variant"), PLOTS, Plot::Bar),
            labels: Dynamic::of(props, "labels"),
            series: Dynamic::of(props, "series"),
        },
        "Stat" => Piece::Stat {
            label: Dynamic::of(props, "label"),
            value: Dynamic::of(props, "value"),
            tone: tone(),
        },
        "Table" => Piece::Table {
            columns: Dynamic::of(props, "columns"),
            rows: Dynamic::of(props, "rows"),
        },
        "Markdown" => Piece::Markdown {
            text: Dynamic::of(props, "text"),
        },
        "Mermaid" => Piece::Mermaid {
            source: Dynamic::of(props, "source"),
        },
        "Diff" => Piece::Diff {
            patch: Dynamic::of(props, "patch"),
        },
        "Item" => Piece::Item(item(props, tone())),
        _ => return None,
    };
    Some(Kind::Piece(piece))
}

fn item(props: &Map<String, Value>, tone: Mood) -> Item {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "depth is clamped to u16"
    )]
    let depth = number(props, "depth", 0.0).clamp(0.0, f32::from(u16::MAX)) as u16;
    Item {
        title: Dynamic::of(props, "title"),
        status: Dynamic::of(props, "status"),
        detail: Dynamic::of(props, "detail"),
        marker: Dynamic::of(props, "marker"),
        tone,
        depth,
        event: event(props),
    }
}
