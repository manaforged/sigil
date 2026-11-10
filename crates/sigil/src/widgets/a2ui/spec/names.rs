use serde_json::Value;

use super::{Axis, ButtonLook, FieldKind, ImageSize, TextStyle};
use crate::kernel::ObjectFit;
use crate::layout::{AlignItems, JustifyContent};

pub(super) const TEXT_STYLES: &[(&str, TextStyle)] = &[
    ("h1", TextStyle::H1),
    ("h2", TextStyle::H2),
    ("h3", TextStyle::H3),
    ("h4", TextStyle::H4),
    ("h5", TextStyle::H5),
    ("body", TextStyle::Body),
    ("caption", TextStyle::Caption),
];

pub(super) const IMAGE_SIZES: &[(&str, ImageSize)] = &[
    ("icon", ImageSize::Icon),
    ("avatar", ImageSize::Avatar),
    ("smallFeature", ImageSize::Small),
    ("mediumFeature", ImageSize::Medium),
    ("largeFeature", ImageSize::Large),
    ("header", ImageSize::Header),
];

pub(super) const FITS: &[(&str, ObjectFit)] = &[
    ("contain", ObjectFit::Contain),
    ("scaleDown", ObjectFit::Contain),
    ("none", ObjectFit::Contain),
    ("cover", ObjectFit::Cover),
    ("fill", ObjectFit::Fill),
];

pub(super) const LOOKS: &[(&str, ButtonLook)] = &[
    ("default", ButtonLook::Default),
    ("primary", ButtonLook::Primary),
    ("borderless", ButtonLook::Borderless),
];

pub(super) const FIELD_KINDS: &[(&str, FieldKind)] = &[
    ("shortText", FieldKind::Short),
    ("longText", FieldKind::Long),
    ("number", FieldKind::Number),
    ("obscured", FieldKind::Obscured),
];

pub(super) const AXES: &[(&str, Axis)] = &[
    ("horizontal", Axis::Horizontal),
    ("vertical", Axis::Vertical),
];

pub(super) const JUSTIFY: &[(&str, JustifyContent)] = &[
    ("start", JustifyContent::Start),
    ("center", JustifyContent::Center),
    ("end", JustifyContent::End),
    ("spaceBetween", JustifyContent::SpaceBetween),
    ("spaceAround", JustifyContent::SpaceAround),
    ("spaceEvenly", JustifyContent::SpaceEvenly),
    ("stretch", JustifyContent::Stretch),
];

pub(super) const ALIGN: &[(&str, AlignItems)] = &[
    ("start", AlignItems::Start),
    ("center", AlignItems::Center),
    ("end", AlignItems::End),
    ("stretch", AlignItems::Stretch),
];

pub(super) fn pick<T: Copy>(value: Option<&Value>, table: &[(&str, T)], default: T) -> T {
    value
        .and_then(Value::as_str)
        .and_then(|name| table.iter().find(|(known, _)| *known == name))
        .map_or(default, |(_, picked)| *picked)
}
