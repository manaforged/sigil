#![allow(clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use crate::kernel::{Cx, NodeId, container};
use crate::reactive::{Owner, signal};
use crate::style::{ColorAtom, Rgba8, StyleAtom, StyleBinding, Token};

static INACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundColor(ColorAtom::Token(Token::Surface))];
static ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent))];
static PADDING_ON: &[StyleAtom] = &[StyleAtom::PaddingAll(16)];
static PADDING_OFF: &[StyleAtom] = &[StyleAtom::PaddingAll(4)];

#[test]
fn reactive_reapplies_on_signal_change() {
    Owner::scope("sigil_style_reactive_reapplies", || {
        let active = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(
            move || {
                if active.get() { ACTIVE } else { INACTIVE }
            },
        ));

        assert_eq!(reactive_slice(&cx, id), INACTIVE);

        active.set(true);
        assert_eq!(reactive_slice(&cx, id), ACTIVE);
    });
}

#[test]
fn dispose_drops_effect() {
    let runs = Rc::new(Cell::new(0_u32));

    Owner::scope("sigil_style_reactive_dispose", || {
        let active = signal(false);
        let runs_for_effect = Rc::clone(&runs);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(move || {
            runs_for_effect.set(runs_for_effect.get() + 1);
            if active.get() { ACTIVE } else { INACTIVE }
        }));

        assert_eq!(runs.get(), 1);
        active.set(true);
        assert_eq!(runs.get(), 2);
        assert!(cx.remove(id));

        active.set(false);
        assert_eq!(runs.get(), 2);
    });
}

#[test]
fn reactive_toggle_is_alloc_free() {
    Owner::scope("sigil_style_reactive_alloc_free", || {
        let active = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(move || {
            if active.get() {
                PADDING_ON
            } else {
                PADDING_OFF
            }
        }));

        active.set(true);
        active.set(false);

        let info = allocation_counter::measure(|| {
            for index in 0..100 {
                active.set(index % 2 == 0);
            }
        });

        assert_eq!(info.count_total, 0, "{info:?}");
        assert_eq!(reactive_slice(&cx, id), PADDING_OFF);
    });
}

#[test]
fn pillar_call_site_compiles() {
    Owner::scope("sigil_style_pillar_call_site", || {
        let danger = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(move || {
            if danger.get() {
                &[StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 255,
                }))]
            } else {
                INACTIVE
            }
        }));

        assert_eq!(reactive_slice(&cx, id), INACTIVE);
    });
}

fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}

#[test]
fn a_later_axis_drive_wins_over_a_uniform_scale_on_that_axis_only() {
    use crate::animation::Property;
    use crate::style::{AnimatorOverlay, NumericAtomKind};

    Owner::scope("sigil_style_driven_scale", || {
        let mut uniform_first = StyleBinding::default();
        uniform_first.bind_animation(Property::Scale, signal(2.0_f32));
        uniform_first.bind_animation(Property::ScaleY, signal(3.0_f32));
        assert_eq!(uniform_first.value_for(NumericAtomKind::ScaleX), Some(2.0));
        assert_eq!(uniform_first.value_for(NumericAtomKind::ScaleY), Some(3.0));

        let mut axis_x = StyleBinding::default();
        axis_x.bind_animation(Property::Scale, signal(2.0_f32));
        axis_x.bind_animation(Property::ScaleX, signal(4.0_f32));
        assert_eq!(axis_x.value_for(NumericAtomKind::ScaleX), Some(4.0));
        assert_eq!(axis_x.value_for(NumericAtomKind::ScaleY), Some(2.0));
    });
}
