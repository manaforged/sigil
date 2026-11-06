use std::rc::Rc;

use crate::kernel::{IntoNode, NodeBuilder, TextContent, fragment, show, text};
use crate::layout::{column, row};
use crate::style::{ClassSource, Paint};
use crate::widgets::{Widget, button, card, checkbox, field, scroll, spacer, text_field};

use super::parse::{Attrs, Item, Variant};
use super::vocab::Kind;

pub(super) fn build(item: &Rc<Item>) -> Box<dyn IntoNode> {
    let Some(flag) = item.attrs.show else {
        return shown(item);
    };
    let item = Rc::clone(item);
    Box::new(show(flag, move || shown(&item), fragment))
}

fn styled<W: Widget>(mut widget: W, attrs: &Attrs) -> W {
    if let Some(padding) = attrs.padding {
        widget = widget.padding(padding);
    }
    if let Some(layer) = attrs.layer {
        widget = widget.map_node(move |node| node.layer(layer));
    }
    if let Some(class) = attrs.class.clone() {
        widget = widget.map_node(move |node| node.class(ClassSource::Owned(Box::new(class))));
    }
    widget
}

fn stack(mut parent: NodeBuilder, item: &Item) -> NodeBuilder {
    if let Some(gap) = item.attrs.gap {
        parent = parent.gap(gap);
    }
    for child in &item.children {
        parent = parent.child(build(child));
    }
    parent
}

fn label(item: &Item) -> TextContent {
    item.label
        .as_ref()
        .map_or_else(|| TextContent::from(""), TextContent::from)
}

fn shown(item: &Rc<Item>) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    match item.kind {
        Kind::Column => Box::new(styled(stack(column(), item), attrs)),
        Kind::Row => Box::new(styled(stack(row(), item), attrs)),
        Kind::Card => Box::new(styled(
            NodeBuilder::from(card(stack(column(), item))),
            attrs,
        )),
        Kind::Scroll => Box::new(styled(
            NodeBuilder::from(scroll(stack(column(), item))),
            attrs,
        )),
        Kind::Field => field_of(item),
        Kind::Text => text_of(item),
        Kind::Button => button_of(item),
        Kind::TextField => text_field_of(item),
        Kind::Checkbox => checkbox_of(item),
        Kind::Spacer => Box::new(styled(spacer(), attrs)),
        Kind::Slot => match &item.slot {
            Some(slot) => Box::new(styled(slot(), attrs)),
            None => Box::new(fragment()),
        },
    }
}

fn field_of(item: &Item) -> Box<dyn IntoNode> {
    let mut control = fragment();
    for child in &item.children {
        control = control.child(build(child));
    }
    Box::new(styled(field(label(item), control), &item.attrs))
}

fn text_of(item: &Item) -> Box<dyn IntoNode> {
    let mut node = text(label(item));
    if let Some(role) = item.attrs.role {
        node = node.text_role(role);
    }
    Box::new(styled(node, &item.attrs))
}

fn button_of(item: &Item) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    let mut node = button(label(item));
    node = match attrs.variant {
        Some(Variant::Primary) => node.primary(),
        Some(Variant::Outline) => node.outline(),
        Some(Variant::Ghost) => node.ghost(),
        None => node,
    };
    if let Some(flag) = attrs.disabled {
        node = node.disabled(flag);
    }
    if let Some(action) = attrs.activate.clone() {
        node = node.activate(move || action());
    }
    Box::new(styled(node, attrs))
}

fn text_field_of(item: &Item) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    let Some(value) = attrs.value else {
        return Box::new(fragment());
    };
    let mut node = text_field(value);
    if let Some(placeholder) = &attrs.placeholder {
        node = node.placeholder(TextContent::from(placeholder));
    }
    if let Some(flag) = attrs.disabled {
        node = node.disabled(flag);
    }
    Box::new(styled(node, attrs))
}

fn checkbox_of(item: &Item) -> Box<dyn IntoNode> {
    let attrs = &item.attrs;
    let Some(checked) = attrs.checked else {
        return Box::new(fragment());
    };
    let mut node = checkbox(checked);
    if let Some(words) = item.label.as_ref().map(TextContent::from) {
        node = node.label(words);
    }
    if let Some(flag) = attrs.disabled {
        node = node.disabled(flag);
    }
    Box::new(styled(node, attrs))
}
