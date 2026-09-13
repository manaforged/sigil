#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss
)]

use sigil::prelude::*;

const VIEWPORT: Size = Size::fixed(1440.0, 900.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        let text = content.as_static_str().unwrap_or("");
        let lines = text.lines().count().max(1) as f32;
        sigil::hal::LogicalSize::new(120.0, lines * 16.8)
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn deep_reactive_scroll_switch_stays_positioned() {
    sigil::reactive::Runtime::scope("incremental-scroll-position", || {
        let tall = signal(false);
        let pane = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .children_reactive(move || {
                let lines = if tall.get() { 200 } else { 2 };
                let body = (0..lines)
                    .map(|i| format!("line {i} of source"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let extent = (lines as f32).mul_add(16.8, 20.0);
                let code = container()
                    .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
                    .child(text(body));
                vec![scroll(code, extent).width(580.0).height(720.0)]
            });
        let mut filler = container().layout(LayoutStyle::flex_column());
        for i in 0..120 {
            filler = filler.child(label(format!("filler row {i}")));
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .child(filler)
            .child(
                container()
                    .layout(LayoutStyle::flex_row().grow(1.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column().width(248.0))
                            .child(label("SIDEBAR".to_owned())),
                    )
                    .child(pane),
            );

        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("initial layout");

        tall.set(true);
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("layout after switch");

        let mut escaped = Vec::new();
        cx.visit_preorder(root, |id, _node| {
            if let Some(res) = layout.result(id) {
                let r = res.rect();
                if r.height > 900.0 && r.width > 1500.0 && r.origin.x < 60.0 && r.origin.y < 60.0 {
                    escaped.push((r.origin.x, r.origin.y, r.width, r.height));
                }
            }
        });
        assert!(
            escaped.is_empty(),
            "tall scroll content escaped to the top-left (incremental containment-root bug): {escaped:?}"
        );
    });
}
