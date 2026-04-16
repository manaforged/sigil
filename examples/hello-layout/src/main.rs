//! Headless sigil-layout proof example.

// Example intentionally prints layout results to stdout.
#![allow(clippy::print_stdout)]

use std::error::Error;
use std::rc::Rc;

use sigil_core::prelude::*;
use sigil_layout::TextEngineMeasure;
use sigil_layout::prelude::*;
use sigil_text::{ParleyTextEngine, TextConfig};

fn main() -> Result<(), Box<dyn Error>> {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .role(Role::Main)
            .aria_label("Hello layout")
            .child(text("Hello"))
            .child(container().child(text("from layout"))),
    );

    attach(
        &mut cx,
        root,
        LayoutIntent::flex_row()
            .fixed_size(320.0, 80.0)
            .gap_px(12.0)
            .align_items(AlignItems::FlexStart),
    )?;

    let second = cx
        .get(root)
        .and_then(|node| node.children.get(1))
        .copied()
        .ok_or_else(|| std::io::Error::other("second child exists"))?;
    attach(
        &mut cx,
        second,
        LayoutIntent::flex_column()
            .padding_px(4.0)
            .align_items(AlignItems::FlexStart),
    )?;

    let engine = Rc::new(ParleyTextEngine::new(TextConfig::default()));
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

fn attach(cx: &mut Cx, id: NodeId, intent: LayoutIntent) -> Result<(), Box<dyn Error>> {
    let node = cx
        .get_mut(id)
        .ok_or_else(|| std::io::Error::other("node exists"))?;
    let _ = node.attach(intent);
    Ok(())
}

const fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Container => "container",
        NodeKind::Text { .. } => "text",
        NodeKind::Fragment => "fragment",
    }
}
