use sigil::Host;
use sigil::kernel::{container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::LayoutBuilderExt;
use sigil::profile::{self, BackendStatus, Counter};
use sigil::reactive::{Runtime, signal};
use sigil::scene::CommandScene;

const VIEW: (f32, f32) = (320.0, 160.0);

fn counted(work: impl FnOnce()) -> (u64, u64) {
    let _profiled = crate::profiled();
    profile::enable();
    profile::reset();
    profile::begin_frame(BackendStatus::TERMINAL);
    work();
    profile::tick_frame();
    let frame = profile::frames().last().cloned();
    profile::disable();
    frame.map_or((0, 0), |frame| {
        (
            frame.counter(Counter::EmitNodes),
            frame.counter(Counter::EmitCopied),
        )
    })
}

#[test]
fn an_unchanged_subtree_is_copied() {
    Runtime::scope("emit-copy", || {
        let word = signal(String::from("one"));
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child((0..12).fold(
                    container().layout(LayoutStyle::flex_column()),
                    |column, row| column.child(text(format!("steady row {row}"))),
                ))
                .child(text(word))
        });
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        word.set(String::from("two"));
        host.layout(VIEW).expect("layout");
        let changed = host.cx().take_delta();
        let mut second = CommandScene::default();
        let (walked, copied) = counted(|| {
            let (scene, diff) = host.emit_since(VIEW, &changed, &first);
            assert!(!diff.unchanged());
            second = scene;
        });
        assert_eq!(
            second,
            host.emit(VIEW),
            "the incremental emit differs from a fresh one"
        );
        assert!(copied > 0, "nothing was copied (walked {walked})");
        host.layout(VIEW).expect("layout");
        let idle = host.cx().take_delta();
        let (idle_walked, _) = counted(|| {
            let (third, diff) = host.emit_since(VIEW, &idle, &second);
            assert!(diff.unchanged());
            assert_eq!(third, second);
        });
        assert_eq!(idle_walked, 0, "an idle frame walked {idle_walked} nodes");
    });
}

#[test]
fn a_keyed_swap_still_copies_its_static_sibling() {
    Runtime::scope("emit-keyed-sibling", || {
        let word = signal(String::from("one"));
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child((0..12).fold(
                    container().layout(LayoutStyle::flex_column()),
                    |column, row| column.child(text(format!("still {row}"))),
                ))
                .child(
                    container()
                        .layout(LayoutStyle::flex_row())
                        .children_keyed(move || {
                            let shown = word.get();
                            vec![(shown.clone(), text(shown))]
                        }),
                )
        });
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        word.set(String::from("two"));
        host.layout(VIEW).expect("layout");
        let changed = host.cx().take_delta();
        let mut second = CommandScene::default();
        let (walked, copied) = counted(|| {
            let (scene, _) = host.emit_since(VIEW, &changed, &first);
            second = scene;
        });
        assert_eq!(
            second,
            host.emit(VIEW),
            "the incremental emit differs from a fresh one"
        );
        assert!(
            copied >= 12,
            "a keyed swap re-emitted its static sibling: walked {walked}, copied {copied}"
        );
    });
}

#[test]
fn an_idle_frame_keeps_the_copies_for_the_next_change() {
    Runtime::scope("emit-after-idle", || {
        let word = signal(String::from("one"));
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child((0..12).fold(
                    container().layout(LayoutStyle::flex_column()),
                    |column, row| column.child(text(format!("still {row}"))),
                ))
                .child(text(word))
        });
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        host.layout(VIEW).expect("layout");
        let idle = host.cx().take_delta();
        let (second, diff) = host.emit_since(VIEW, &idle, &first);
        assert!(diff.unchanged());
        word.set(String::from("two"));
        host.layout(VIEW).expect("layout");
        let changed = host.cx().take_delta();
        let mut third = CommandScene::default();
        let (walked, copied) = counted(|| {
            let (scene, _) = host.emit_since(VIEW, &changed, &second);
            third = scene;
        });
        assert_eq!(
            third,
            host.emit(VIEW),
            "the incremental emit differs from a fresh one"
        );
        assert!(
            copied >= 12,
            "a change after an idle frame re-emitted the unchanged column: walked {walked}, copied {copied}"
        );
    });
}

#[test]
fn a_reactive_atoms_write_reaches_the_next_emit() {
    use sigil::style::{ColorAtom, ReactiveStyleAtoms, Rgba8, StyleAtom, StyleBinding};
    use std::cell::Cell;
    use std::rc::Rc;
    const fn fill(r: u8) -> [StyleAtom; 1] {
        [StyleAtom::BackgroundColor(ColorAtom::Rgba8(Rgba8 {
            r,
            g: 40,
            b: 40,
            a: 255,
        }))]
    }
    static FIRST: [StyleAtom; 1] = fill(200);
    static SECOND: [StyleAtom; 1] = fill(20);
    Runtime::scope("emit-reactive-atoms", || {
        let atoms: ReactiveStyleAtoms = Rc::new(Cell::new(&FIRST));
        let shared = Rc::clone(&atoms);
        let mut host = Host::cells();
        host.mount(move || {
            let attached = Rc::clone(&shared);
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(text("steady"))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(80.0, 16.0))
                        .on_added(move |cx, id| {
                            if let Some(node) = cx.get_mut(id) {
                                node.attach(StyleBinding::reactive_atoms(attached));
                            }
                        }),
                )
        });
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        host.layout(VIEW).expect("layout");
        let idle = host.cx().take_delta();
        let (second, _) = host.emit_since(VIEW, &idle, &first);
        atoms.set(&SECOND);
        host.layout(VIEW).expect("layout");
        let after = host.cx().take_delta();
        let (third, _) = host.emit_since(VIEW, &after, &second);
        assert_eq!(
            third,
            host.emit(VIEW),
            "a reactive atoms write kept the old paint"
        );
    });
}

#[test]
fn a_host_with_nothing_mounted_reports_the_last_scene_gone() {
    Runtime::scope("emit-rootless", || {
        let mut shown = Host::cells();
        shown.mount(|| container().child(text("before")));
        let before = shown.scene(VIEW).expect("scene");
        assert!(!before.commands.is_empty());
        let mut empty = Host::cells();
        let delta = empty.cx().take_delta();
        let (scene, diff) = empty.emit_since(VIEW, &delta, &before);
        assert!(scene.commands.is_empty());
        assert!(!diff.unchanged(), "an emptied scene reported no change");
    });
}

#[test]
fn a_scale_change_emits_fresh_geometry() {
    Runtime::scope("emit-scale", || {
        let mut host = Host::cells();
        host.mount(|| container().child(text("scaled")));
        host.layout(VIEW).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(VIEW, &delta, &CommandScene::default());
        host.cx().set_scale(sigil::geometry::Scale::new(2.0));
        host.layout(VIEW).expect("layout");
        let rescaled = host.cx().take_delta();
        let (second, _) = host.emit_since(VIEW, &rescaled, &first);
        assert_eq!(second, host.emit(VIEW), "a scale change reused old paint");
    });
}
