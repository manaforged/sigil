#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use accesskit as _;
use criterion as _;
use smallvec as _;
use taffy as _;

use sigil_core::prelude::*;
use sigil_events::prelude::*;
use sigil_hal::{ModifiersState, MouseButtons, Point, PointerEvent, PointerEventInit, PointerKind};
use sigil_layout::prelude::*;
use sigil_reactive::{Runtime, signal};
use sigil_style::{NodeStyleExt, StyleAtom, StyleBinding};

static HOVER_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static HOVER_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

#[test]
fn pointer_move_drives_style_reactive_without_allocating() {
    Runtime::scope("sigil_events_style_reactive", || {
        let hover = signal(false);
        let flips = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .on_pointer_move_capture({
                    let flips = Rc::clone(&flips);
                    move |event| {
                        let inside = event.position.x < 50.0 && event.position.y < 50.0;
                        if hover.peek() != inside {
                            flips.set(flips.get() + 1);
                            hover.set(inside);
                        }
                    }
                })
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(50.0, 50.0))
                        .style_reactive(move || if hover.get() { HOVER_ON } else { HOVER_OFF }),
                ),
        );
        let child = first_child(&cx, root);
        let layout = compute_layout(&mut cx, root);
        let mut router = EventRouter::new();

        router.route(&mut cx, &layout, pointer_move(75.0, 75.0));
        router.route(&mut cx, &layout, pointer_move(10.0, 10.0));
        router.route(&mut cx, &layout, pointer_move(75.0, 75.0));
        flips.set(0);
        assert_eq!(reactive_slice(&cx, child), HOVER_OFF);

        let enter = allocation_counter::measure(|| {
            router.route(&mut cx, &layout, pointer_move(10.0, 10.0));
        });
        assert_eq!(enter.count_total, 0, "{enter:?}");
        assert_eq!(flips.get(), 1);
        assert_eq!(reactive_slice(&cx, child), HOVER_ON);

        let leave = allocation_counter::measure(|| {
            router.route(&mut cx, &layout, pointer_move(75.0, 75.0));
        });
        assert_eq!(leave.count_total, 0, "{leave:?}");
        assert_eq!(flips.get(), 2);
        assert_eq!(reactive_slice(&cx, child), HOVER_OFF);
    });
}

fn compute_layout(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(ZeroTextMeasure));
    layout
        .compute(cx, root, Size::fixed(100.0, 100.0))
        .expect("layout computes");
    layout
}

fn first_child(cx: &Cx, root: NodeId) -> NodeId {
    cx.get(root).unwrap().children.first().copied().unwrap()
}

fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    cx.get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .map(StyleBinding::reactive_slice)
        .unwrap()
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
        phase: PointerPhase::Move,
    }))
}

struct ZeroTextMeasure;

impl TextMeasure for ZeroTextMeasure {
    fn measure(
        &self,
        _content: &TextContent,
        _space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    ) -> taffy::prelude::Size<f32> {
        taffy::prelude::Size::ZERO
    }
}
