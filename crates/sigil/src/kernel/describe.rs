use core::fmt::Write as _;

use crate::kernel::{AccessibleName, Cx, Node, NodeId, NodeType, TextContent};

const TEXT_CHARS: usize = 24;
const ANCESTOR_HOPS: usize = 4;

impl Cx {
    #[must_use]
    pub fn describe(&self, id: NodeId) -> String {
        let Some(node) = self.get(id) else {
            return format!("{id:?}");
        };
        let mut out = format!("{:?}", node.a11y.role);
        if let Some(name) = accessible_name(node) {
            let _ = write!(out, " {name:?}");
            return out;
        }
        if let NodeType::Text { content, .. } = &node.kind {
            let text = text_prefix(content);
            if !text.is_empty() {
                let _ = write!(out, " {text:?}");
                return out;
            }
        }
        let mut current = self.parent(id);
        for _ in 0..ANCESTOR_HOPS {
            let Some(parent_id) = current else {
                break;
            };
            let Some(parent) = self.get(parent_id) else {
                break;
            };
            if let Some(name) = accessible_name(parent) {
                let _ = write!(out, " [in {:?} {name:?}]", parent.a11y.role);
                return out;
            }
            current = self.parent(parent_id);
        }
        out
    }
}

fn accessible_name(node: &Node) -> Option<String> {
    let name = match &node.a11y.name {
        AccessibleName::None => return None,
        AccessibleName::Static(value) => value.clone(),
        AccessibleName::Derived(signal) => signal.try_peek()?,
    };
    (!name.is_empty()).then(|| truncate(name))
}

fn text_prefix(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.chars().take(TEXT_CHARS).collect(),
        TextContent::Reactive(_) | TextContent::Compute(_) => truncate(content.resolve()),
    }
}

fn truncate(mut value: String) -> String {
    if let Some((index, _)) = value.char_indices().nth(TEXT_CHARS) {
        value.truncate(index);
    }
    value
}

#[cfg(test)]
#[path = "describe_tests.rs"]
mod tests;
