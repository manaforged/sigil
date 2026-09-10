#![allow(clippy::print_stdout)]

use std::error::Error;
use std::rc::Rc;

use sigil_kernel::prelude::*;
use sigil_layout::TextEngineMeasure;
use sigil_layout::prelude::*;
use sigil_text::{OwnedTextEngine, TextConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .role(Role::Main)
            .label("Hello layout")
            .layout(
                LayoutStyle::flex_row()
                    .fixed_size(320.0, 80.0)
                    .gap(12.0)
                    .align_items(AlignItems::FlexStart),
            )
            .child(text("Hello"))
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .padding(4.0)
                            .align_items(AlignItems::FlexStart),
                    )
                    .child(text("from layout")),
            ),
    );

    let engine = Rc::new(OwnedTextEngine::new(TextConfig::default()));
    let measurer = TextEngineMeasure::new(engine);
    let mut layout = LayoutEngine::with_text_measure(Box::new(measurer));
    layout.compute(&mut cx, root, Size::fixed(320.0, 80.0))?;

    cx.visit_preorder(root, |id, node| {
        if let Some(result) = node.get_extension::<LayoutResult>() {
            println!(
                "{id:?} kind={} rect=({:.1},{:.1},{:.1},{:.1})",
                kind_name(&node.kind),
                result.rect().origin.x,
                result.rect().origin.y,
                result.rect().width,
                result.rect().height
            );
        }
    });

    Ok(())
}

const fn kind_name(kind: &NodeType) -> &'static str {
    match kind {
        NodeType::Container => "container",
        NodeType::Text { .. } => "text",
        NodeType::Image { .. } => "image",
        NodeType::Fragment => "fragment",
    }
}
