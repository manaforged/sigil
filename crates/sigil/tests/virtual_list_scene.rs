//! Virtual-list paint tests that need `emit_scene` (GPU crate).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_profile as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use sigil::widgets::virtual_list_bind;
use sigil_hal::Size;
use sigil_kernel::{Cx, TextContent, container, text_live};
use sigil_layout::{LayoutEngine, TextMeasure};
use sigil_reactive::{Runtime, signal};
use sigil_renderer::emit_scene;

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

#[test]
fn items_bind_tick_updates_reactive_text() {
    Runtime::scope("sigil-vlist-tick-text", || {
        let items = signal(vec!["alpha".to_string(), "beta".to_string()]);
        let mut cx = Cx::new_root();
        let root = cx.add(
            virtual_list_bind(items, move |index, _| {
                container().child(text_live(move || {
                    items.get().get(index).cloned().unwrap_or_default()
                }))
            })
            .row_height(20.0)
            .viewport_height(80.0)
            .viewport_width(200.0),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
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
