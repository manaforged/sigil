use crate::layout::Space;
use crate::style::{Layer, TextRole};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Column,
    Row,
    Card,
    Scroll,
    Field,
    Text,
    Button,
    TextField,
    Checkbox,
    Spacer,
    Slot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Prop {
    Gap,
    Padding,
    Layer,
    Role,
    Class,
    Show,
    Primary,
    Outline,
    Ghost,
    Activate,
    Value,
    Checked,
    Placeholder,
    Disabled,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Head {
    Nothing,
    Label,
    OptionalLabel,
    Name,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Body {
    Leaf,
    Many,
    One,
}

pub(super) struct Spec {
    pub(super) name: &'static str,
    pub(super) kind: Kind,
    pub(super) head: Head,
    pub(super) body: Body,
    pub(super) props: &'static [Prop],
    pub(super) required: Option<Prop>,
}

const fn spec(name: &'static str, kind: Kind, head: Head, body: Body) -> Spec {
    Spec {
        name,
        kind,
        head,
        body,
        props: &[],
        required: None,
    }
}

const COMMON: &[Prop] = &[Prop::Padding, Prop::Layer, Prop::Class, Prop::Show];
const STACK: &[Prop] = &[Prop::Gap];
const TEXT: &[Prop] = &[Prop::Role];
const BUTTON: &[Prop] = &[
    Prop::Primary,
    Prop::Outline,
    Prop::Ghost,
    Prop::Activate,
    Prop::Disabled,
];
const INPUT: &[Prop] = &[Prop::Value, Prop::Placeholder, Prop::Disabled];
const CHECK: &[Prop] = &[Prop::Checked, Prop::Disabled];

pub(super) const WIDGETS: &[Spec] = &[
    Spec {
        props: STACK,
        ..spec("column", Kind::Column, Head::Nothing, Body::Many)
    },
    Spec {
        props: STACK,
        ..spec("row", Kind::Row, Head::Nothing, Body::Many)
    },
    Spec {
        props: STACK,
        ..spec("card", Kind::Card, Head::Nothing, Body::Many)
    },
    Spec {
        props: STACK,
        ..spec("scroll", Kind::Scroll, Head::Nothing, Body::Many)
    },
    spec("field", Kind::Field, Head::Label, Body::One),
    Spec {
        props: TEXT,
        ..spec("text", Kind::Text, Head::Label, Body::Leaf)
    },
    Spec {
        props: BUTTON,
        ..spec("button", Kind::Button, Head::Label, Body::Leaf)
    },
    Spec {
        props: INPUT,
        required: Some(Prop::Value),
        ..spec("text_field", Kind::TextField, Head::Nothing, Body::Leaf)
    },
    Spec {
        props: CHECK,
        required: Some(Prop::Checked),
        ..spec("checkbox", Kind::Checkbox, Head::OptionalLabel, Body::Leaf)
    },
    spec("spacer", Kind::Spacer, Head::Nothing, Body::Leaf),
    spec("slot", Kind::Slot, Head::Name, Body::Leaf),
];

pub(super) const PROPS: &[(&str, Prop)] = &[
    ("gap", Prop::Gap),
    ("padding", Prop::Padding),
    ("layer", Prop::Layer),
    ("role", Prop::Role),
    ("class", Prop::Class),
    ("show", Prop::Show),
    ("primary", Prop::Primary),
    ("outline", Prop::Outline),
    ("ghost", Prop::Ghost),
    ("activate", Prop::Activate),
    ("value", Prop::Value),
    ("checked", Prop::Checked),
    ("placeholder", Prop::Placeholder),
    ("disabled", Prop::Disabled),
];

pub(super) const SPACES: &[(&str, Space)] = &[
    ("none", Space::None),
    ("xxs", Space::Xxs),
    ("xs", Space::Xs),
    ("sm", Space::Sm),
    ("md", Space::Md),
    ("lg", Space::Lg),
    ("xl", Space::Xl),
    ("xxl", Space::Xxl),
    ("xxxl", Space::Xxxl),
];

pub(super) const LAYERS: &[(&str, Layer)] = &[
    ("page", Layer::Page),
    ("sunken", Layer::Sunken),
    ("raised", Layer::Raised),
    ("overlay", Layer::Overlay),
];

pub(super) const ROLES: &[(&str, TextRole)] = &[
    ("caption", TextRole::Caption),
    ("body", TextRole::Body),
    ("label", TextRole::Label),
    ("subtitle", TextRole::Subtitle),
    ("title", TextRole::Title),
    ("heading", TextRole::Heading),
    ("display", TextRole::Display),
];

impl Spec {
    pub(super) fn allows(&self, prop: Prop) -> bool {
        COMMON.contains(&prop) || self.props.contains(&prop)
    }
}

pub(super) fn names<T>(table: &[(&'static str, T)]) -> String {
    let names: Vec<&str> = table.iter().map(|(name, _)| *name).collect();
    names.join(", ")
}

pub(super) fn widget_names() -> String {
    let names: Vec<&str> = WIDGETS.iter().map(|spec| spec.name).collect();
    names.join(", ")
}
