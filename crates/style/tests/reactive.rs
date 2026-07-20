#![allow(missing_docs)]
#![allow(clippy::panic, clippy::unwrap_used)]

use std::cell::Cell;
use std::rc::Rc;

use allocation_counter as _;
use criterion as _;
use phf as _;
use sigil as _;
use sigil_hal as _;
use sigil_reactive as _;
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;
use tracing as _;

use sigil_kernel::{Cx, container};
use sigil_reactive::{Runtime, signal};
use sigil_style::{NodeStyleExt, Px, Rgba8, StyleAtom, StyleBinding};

static INACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static PADDING_ON: &[StyleAtom] = &[StyleAtom::PaddingAll(Px(16))];
static PADDING_OFF: &[StyleAtom] = &[StyleAtom::PaddingAll(Px(4))];

#[test]
fn reactive_reapplies_on_signal_change() {
    Runtime::scope("sigil_style_reactive_reapplies", || {
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

    Runtime::scope("sigil_style_reactive_dispose", || {
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
    Runtime::scope("sigil_style_reactive_alloc_free", || {
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
    Runtime::scope("sigil_style_pillar_call_site", || {
        let danger = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(move || {
            if danger.get() {
                &[StyleAtom::BackgroundColor(sigil_style::ColorAtom::Rgba8(
                    Rgba8 {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 255,
                    },
                ))]
            } else {
                INACTIVE
            }
        }));

        assert_eq!(reactive_slice(&cx, id), INACTIVE);
    });
}

#[test]
fn class_when_toggles_on_signal_change() {
    Runtime::scope("sigil_style_class_when_toggle", || {
        let highlighted = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().class_when(highlighted, "bg-accent"));

        let initial = reactive_slice(&cx, id);
        assert!(
            initial.is_empty(),
            "class_when starts with empty slice while signal is false, got {initial:?}",
        );

        highlighted.set(true);
        let live = reactive_slice(&cx, id);
        assert_eq!(
            live,
            &[StyleAtom::BackgroundThemeRef("accent")],
            "class_when must install `bg-accent` atoms when signal flips true, got {live:?}",
        );

        highlighted.set(false);
        assert!(
            reactive_slice(&cx, id).is_empty(),
            "class_when must clear atoms when signal flips false",
        );
    });
}

fn reactive_slice(cx: &Cx, id: sigil_kernel::NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();
    // T0 composition: concatenate every reactive channel in install order.
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}
