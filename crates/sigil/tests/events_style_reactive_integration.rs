#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used)]

#[path = "layout_stub/mod.rs"]
mod stub;
use sigil_kittest::FakeMeasure;
use stub::first_child;

use std::cell::Cell;
use std::rc::Rc;

use sigil::events::*;
use sigil::geometry::{Point, Size};
use sigil::kernel::*;
use sigil::layout::*;
use sigil::prelude::Widget;
use sigil::reactive::{Owner, signal};
use sigil::style::{ColorAtom, Paint, PaintAtom, StyleAtom, StyleBinding, Token};

static HOVER_OFF: &[StyleAtom] = &[];
static HOVER_ON: &[StyleAtom] = &[
    StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent)),
    StyleAtom::Paint(PaintAtom::NoGradient),
];

#[test]
fn pointer_move_drives_style_reactive_without_allocating() {
    Owner::scope("sigil_events_style_reactive", || {
        let hover = signal(false);
        let flips = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(100.0, 100.0))
                .pointer_move_capture({
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
                        .background(Token::Surface)
                        .paint_when(hover, |s| s.background(Token::Accent)),
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
    let mut layout = LayoutEngine::with_text_measure(Box::new(FakeMeasure::default()));
    layout
        .compute(cx, root, Size::fixed(100.0, 100.0))
        .expect("layout computes");
    layout
}

fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}

const fn pointer_move(x: f32, y: f32) -> Event {
    Event::pointer(PointerPhase::Move, Point::new(x, y))
}
