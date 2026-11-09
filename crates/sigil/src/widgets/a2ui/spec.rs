use serde_json::Value;

use crate::kernel::ObjectFit;
use crate::layout::{AlignItems, JustifyContent};

mod parse;

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
    Unsupported {
        name: String,
    },
}

pub(super) struct Component {
    pub(super) kind: Kind,
    pub(super) weight: Option<f32>,
}
