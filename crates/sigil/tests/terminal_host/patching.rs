use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{NodeBuilder, container, fragment, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::LayoutBuilderExt;
use sigil::profile::{self, BackendStatus, Counter};
use sigil::reactive::{Runtime, Signal, signal};
use sigil::terminal::Screen;
use sigil_kittest::input::tab;

const SIZE: (u16, u16) = (40, 12);

#[derive(Clone, Copy, PartialEq)]
enum Row {
    Text,
    Boxed,
    Focusable,
    Nested,
}

fn row(kind: Row, word: &str) -> NodeBuilder {
    let label = text(word.to_string());
    match kind {
        Row::Text => fragment().child(label),
        Row::Boxed => container().child(label),
        Row::Focusable => container().focusable(true).child(label),
        Row::Nested => {
            let word = word.to_string();
            container().children_reactive(move || vec![text(word.clone())])
        }
    }
}

fn mounted(rows: Signal<Vec<(Row, String)>>) -> (Host, EventRouter, Screen) {
    let mut host = Host::cells();
    host.mount(move || {
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .children_reactive(move || {
                rows.with(|rows| {
                    rows.iter()
                        .map(|(kind, word)| row(*kind, word))
                        .collect::<Vec<_>>()
                })
            })
    });
    let mut router = EventRouter::new();
    let mut screen = Screen::new(SIZE.0, SIZE.1);
    for _ in 0..2 {
        screen
            .frame(&mut host, &mut router, [], SIZE)
            .expect("frame");
    }
    (host, router, screen)
}

fn rebuilt(from: Row, to: Row) -> (u64, u64) {
    Runtime::scope("patching-counts", || {
        let rows = signal((0..5).map(|at| (from, format!("row {at}"))).collect());
        let (mut host, mut router, mut screen) = mounted(rows);
        let _profiled = crate::profiled();
        profile::enable();
        profile::reset();
        profile::begin_frame(BackendStatus::TERMINAL);
        rows.set((0..5).map(|at| (to, format!("new {at}"))).collect());
        screen
            .frame(&mut host, &mut router, [], SIZE)
            .expect("frame");
        profile::tick_frame();
        let counted = profile::frames().last().map_or((0, 0), |frame| {
            (
                frame.counter(Counter::KernelPatched),
                frame.counter(Counter::KernelMounted),
            )
        });
        profile::disable();
        counted
    })
}

#[test]
fn a_positional_rebuild_patches_rows_in_place() {
    let (patched, mounted) = rebuilt(Row::Boxed, Row::Boxed);
    assert!(patched >= 5, "patched {patched} of 5 rows");
    assert_eq!(mounted, 0, "a same-kind rebuild mounted {mounted} nodes");
}

#[test]
fn nested_reactive_rows_are_patched_not_remounted() {
    let (patched, mounted) = rebuilt(Row::Nested, Row::Nested);
    assert!(patched >= 10, "patched {patched} of 10 rows and labels");
    assert_eq!(mounted, 0, "a nested rebuild mounted {mounted} nodes");
}

#[test]
fn a_rebuild_that_changes_kind_mounts_new_rows() {
    let (_, mounted) = rebuilt(Row::Text, Row::Boxed);
    assert!(mounted >= 5, "a kind change mounted only {mounted} nodes");
}

#[test]
fn a_row_patched_into_a_focusable_one_takes_tab() {
    Runtime::scope("patching-focus", || {
        let rows = signal((0..3).map(|at| (Row::Boxed, format!("row {at}"))).collect());
        let (mut host, mut router, mut screen) = mounted(rows);
        rows.update(|rows| rows[1].0 = Row::Focusable);
        screen
            .frame(&mut host, &mut router, [], SIZE)
            .expect("frame");
        screen
            .frame(&mut host, &mut router, [tab(false)], SIZE)
            .expect("frame");
        assert!(
            router.focus().is_some(),
            "Tab found no focusable row after the patch"
        );
    });
}
