#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use sigil::geometry::Size;
use sigil::kernel::{Cx, container, text};
use sigil::layout::LayoutEngine;
use sigil::reactive::{Runtime, signal};
use sigil::scene::emit_scene;
use sigil::widgets::virtual_list_bind;
use sigil_kittest::stub_text_measure;

#[test]
fn items_bind_tick_updates_reactive_text() {
    Runtime::scope("sigil-vlist-tick-text", || {
        let items = signal(vec!["alpha".to_string(), "beta".to_string()]);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, move |index, _| {
                container().child(text(move || {
                    items.get().get(index).cloned().unwrap_or_default()
                }))
            })
            .row_height(20.0)
            .viewport_height(80.0)
            .viewport_width(200.0),
        );
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        let avail = Size::fixed(200.0, 80.0);
        layout.compute(&mut cx, root, avail).expect("layout");
        assert!(emit_scene(&cx, root).find_text("alpha").is_some());
        items.update(|rows| *rows.first_mut().expect("one row") = "omega".to_string());
        layout.compute(&mut cx, root, avail).expect("tick");
        assert!(
            emit_scene(&cx, root).find_text("omega").is_some(),
            "reactive cell must paint the new value"
        );
    });
}
