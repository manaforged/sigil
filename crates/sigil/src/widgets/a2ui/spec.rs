use serde_json::{Map, Value};

use crate::kernel::ObjectFit;
use crate::layout::{AlignItems, JustifyContent};

use super::glyph;

mod catalog;
mod names;

use names::{ALIGN, AXES, FIELD_KINDS, FITS, IMAGE_SIZES, JUSTIFY, LOOKS, TEXT_STYLES, pick};

const MAX_CHILDREN: usize = 256;
const MAX_OPTIONS: usize = 64;
const MAX_TABS: usize = 16;

pub(super) use catalog::{Item, Mood, Piece, Plot};

#[derive(Clone)]
pub(super) enum Dynamic {
    Literal(Value),
    Path(String),
}

#[derive(Clone, Copy)]
pub(super) enum TextStyle {
    H1,
    H2,
    H3,
    H4,
    H5,
    Body,
    Caption,
}

#[derive(Clone, Copy)]
pub(super) enum ImageSize {
    Icon,
    Avatar,
    Small,
    Medium,
    Large,
    Header,
}

#[derive(Clone, Copy)]
pub(super) enum ButtonLook {
    Default,
    Primary,
    Borderless,
}

#[derive(Clone, Copy)]
pub(super) enum FieldKind {
    Short,
    Long,
    Number,
    Obscured,
}

#[derive(Clone, Copy)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub(super) struct Flex {
    pub(super) justify: JustifyContent,
    pub(super) align: AlignItems,
}

pub(super) enum Children {
    Ids(Vec<String>),
    Template { id: String, path: String },
}

pub(super) struct Event {
    pub(super) name: String,
    pub(super) context: Vec<(String, Dynamic)>,
}

pub(super) struct Choice {
    pub(super) label: Dynamic,
    pub(super) value: String,
}

pub(super) struct Picker {
    pub(super) label: Dynamic,
    pub(super) value: Dynamic,
    pub(super) options: Vec<Choice>,
    pub(super) multiple: bool,
}

pub(super) struct Clock {
    pub(super) label: Dynamic,
    pub(super) value: Dynamic,
    pub(super) date: bool,
    pub(super) time: bool,
}

pub(super) enum Kind {
    Text {
        text: Dynamic,
        style: TextStyle,
    },
    Image {
        url: Dynamic,
        alt: Dynamic,
        size: ImageSize,
        fit: ObjectFit,
    },
    Icon {
        glyph: Option<usize>,
    },
    Row {
        children: Children,
        flex: Flex,
    },
    Column {
        children: Children,
        flex: Flex,
    },
    List {
        children: Children,
        axis: Axis,
        align: AlignItems,
    },
    Card {
        child: String,
    },
    Tabs {
        tabs: Vec<(Dynamic, String)>,
    },
    Divider {
        axis: Axis,
    },
    Button {
        child: String,
        look: ButtonLook,
        event: Option<Event>,
    },
    TextField {
        label: Dynamic,
        value: Dynamic,
        kind: FieldKind,
    },
    CheckBox {
        label: Dynamic,
        value: Dynamic,
    },
    ChoicePicker(Picker),
    Slider {
        label: Dynamic,
        value: Dynamic,
        min: f32,
        max: f32,
    },
    DateTime(Clock),
    Piece(Piece),
    Unsupported {
        name: String,
    },
}

pub(super) struct Component {
    pub(super) kind: Kind,
    pub(super) weight: Option<f32>,
}

impl Dynamic {
    fn of(props: &Map<String, Value>, key: &str) -> Self {
        match props.get(key) {
            Some(Value::Object(map)) => map
                .get("@path")
                .or_else(|| map.get("path"))
                .and_then(Value::as_str)
                .map_or(Self::Literal(Value::Null), |path| {
                    Self::Path(path.to_string())
                }),
            Some(other) => Self::Literal(other.clone()),
            None => Self::Literal(Value::Null),
        }
    }
}

fn id(props: &Map<String, Value>, key: &str) -> String {
    props
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number(props: &Map<String, Value>, key: &str, default: f32) -> f32 {
    #[expect(clippy::cast_possible_truncation, reason = "slider bounds fit f32")]
    props
        .get(key)
        .and_then(Value::as_f64)
        .map_or(default, |n| n as f32)
}

fn flag(props: &Map<String, Value>, key: &str) -> bool {
    props.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn children(props: &Map<String, Value>) -> Children {
    match props.get("children") {
        Some(Value::Array(ids)) => Children::Ids(
            ids.iter()
                .take(MAX_CHILDREN)
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        ),
        Some(Value::Object(template)) => Children::Template {
            id: id(template, "componentId"),
            path: id(template, "path"),
        },
        _ => Children::Ids(Vec::new()),
    }
}

fn flex(props: &Map<String, Value>) -> Flex {
    Flex {
        justify: pick(props.get("justify"), JUSTIFY, JustifyContent::Start),
        align: pick(props.get("align"), ALIGN, AlignItems::Stretch),
    }
}

fn event(props: &Map<String, Value>) -> Option<Event> {
    let event = props.get("action")?.get("event")?;
    let context = event
        .get("context")
        .and_then(Value::as_object)
        .map(|map| {
            map.keys()
                .take(MAX_CHILDREN)
                .map(|key| (key.clone(), Dynamic::of(map, key)))
                .collect()
        })
        .unwrap_or_default();
    Some(Event {
        name: event.get("name")?.as_str()?.to_string(),
        context,
    })
}

fn tabs(props: &Map<String, Value>) -> Kind {
    let tabs = props
        .get("tabs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_TABS)
        .filter_map(Value::as_object)
        .map(|tab| (Dynamic::of(tab, "title"), id(tab, "child")))
        .collect();
    Kind::Tabs { tabs }
}

fn picker(props: &Map<String, Value>) -> Kind {
    let options = props
        .get("options")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(MAX_OPTIONS)
        .filter_map(Value::as_object)
        .map(|option| Choice {
            label: Dynamic::of(option, "label"),
            value: id(option, "value"),
        })
        .collect();
    Kind::ChoicePicker(Picker {
        label: Dynamic::of(props, "label"),
        value: Dynamic::of(props, "value"),
        options,
        multiple: props.get("variant").and_then(Value::as_str) == Some("multipleSelection"),
    })
}

fn content(name: &str, props: &Map<String, Value>) -> Option<Kind> {
    Some(match name {
        "Text" => Kind::Text {
            text: Dynamic::of(props, "text"),
            style: pick(props.get("variant"), TEXT_STYLES, TextStyle::Body),
        },
        "Image" => Kind::Image {
            url: Dynamic::of(props, "url"),
            alt: Dynamic::of(props, "description"),
            size: pick(props.get("variant"), IMAGE_SIZES, ImageSize::Medium),
            fit: pick(props.get("fit"), FITS, ObjectFit::Contain),
        },
        "Icon" => Kind::Icon {
            glyph: props
                .get("name")
                .and_then(Value::as_str)
                .and_then(glyph::find),
        },
        "Divider" => Kind::Divider {
            axis: pick(props.get("axis"), AXES, Axis::Horizontal),
        },
        "Card" => Kind::Card {
            child: id(props, "child"),
        },
        "Tabs" => tabs(props),
        "Row" => Kind::Row {
            children: children(props),
            flex: flex(props),
        },
        "Column" => Kind::Column {
            children: children(props),
            flex: flex(props),
        },
        "List" => Kind::List {
            children: children(props),
            axis: pick(props.get("direction"), AXES, Axis::Vertical),
            align: pick(props.get("align"), ALIGN, AlignItems::Stretch),
        },
        _ => return None,
    })
}

fn input(name: &str, props: &Map<String, Value>) -> Option<Kind> {
    let label = Dynamic::of(props, "label");
    let value = Dynamic::of(props, "value");
    Some(match name {
        "Button" => Kind::Button {
            child: id(props, "child"),
            look: pick(props.get("variant"), LOOKS, ButtonLook::Default),
            event: event(props),
        },
        "TextField" => Kind::TextField {
            label,
            value,
            kind: pick(props.get("variant"), FIELD_KINDS, FieldKind::Short),
        },
        "CheckBox" => Kind::CheckBox { label, value },
        "ChoicePicker" => picker(props),
        "Slider" => Kind::Slider {
            label,
            value,
            min: number(props, "min", 0.0),
            max: number(props, "max", 100.0),
        },
        "DateTimeInput" => Kind::DateTime(Clock {
            label,
            value,
            date: flag(props, "enableDate"),
            time: flag(props, "enableTime"),
        }),
        _ => return None,
    })
}

impl Component {
    pub(super) fn parse(props: &Map<String, Value>) -> Self {
        let name = props
            .get("component")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let kind = content(name, props)
            .or_else(|| input(name, props))
            .or_else(|| catalog::parse(name, props))
            .unwrap_or_else(|| Kind::Unsupported {
                name: name.to_string(),
            });
        #[expect(clippy::cast_possible_truncation, reason = "flex weights fit f32")]
        let weight = props
            .get("weight")
            .and_then(Value::as_f64)
            .map(|w| w as f32);
        Self { kind, weight }
    }
}
