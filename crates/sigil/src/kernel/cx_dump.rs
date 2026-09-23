use core::fmt::Write as _;

use super::Cx;
use crate::kernel::{NodeId, NodeType, TextContent};

impl Cx {
    #[must_use]
    pub fn dump_tree(&self, root: NodeId) -> String {
        let mut out = String::new();
        self.write_dump(root, 0, &mut out);
        out
    }

    fn write_dump(&self, id: NodeId, depth: usize, out: &mut String) {
        let Some(node) = self.nodes.get(id.key()) else {
            return;
        };
        for _ in 0..depth {
            out.push_str("  ");
        }
        match &node.kind {
            NodeType::Container => {
                let _ = writeln!(out, "Container role={:?}", node.a11y.role);
            }
            NodeType::Fragment => {
                let _ = writeln!(out, "Fragment role={:?}", node.a11y.role);
            }
            NodeType::Text { content, .. } => {
                let _ = writeln!(
                    out,
                    "Text {} role={:?}",
                    dump_text_content(content),
                    node.a11y.role
                );
            }
            NodeType::Image { source, .. } => {
                let _ = writeln!(
                    out,
                    "Image {}x{} role={:?}",
                    source.width(),
                    source.height(),
                    node.a11y.role
                );
            }
        }
        for child in &node.children {
            self.write_dump(*child, depth + 1, out);
        }
    }
}

fn dump_text_content(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => format!("{value:?}"),
        TextContent::Reactive(_) => "<reactive>".to_owned(),
        TextContent::Compute(_) => "<compute>".to_owned(),
    }
}
