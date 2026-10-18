use sigil::Host;
use sigil::events::{EventRouter, NodeEventsExt};
use sigil::kernel::{NodeBuilder, container, fragment, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::{LayoutBuilderExt, Paint};
use sigil::profile::{self, BackendStatus, Counter};
use sigil::reactive::{Runtime, Signal, signal};
use sigil::terminal::Screen;
use sigil_kittest::input::{click, tab};

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

fn themed(dark: Signal<bool>) -> Host {
    use sigil::style::{NodeStyleExt, Theme, Token};
    let mut host = Host::cells();
    host.mount(move || {
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .child(text("outside"))
            .children_reactive(move || {
                let theme = if dark.get() {
                    Theme::dark()
                } else {
                    Theme::light()
                };
                vec![
                    container()
                        .theme(theme)
                        .bg(Token::Surface)
                        .child(text("inside")),
                ]
            })
    });
    host
}

#[test]
fn a_nested_theme_change_reaches_the_emitted_scene() {
    use sigil::scene::CommandScene;
    let view = (320.0, 96.0);
    Runtime::scope("patching-theme", || {
        let dark = signal(false);
        let mut host = themed(dark);
        host.layout(view).expect("layout");
        let delta = host.cx().take_delta();
        let (first, _) = host.emit_since(view, &delta, &CommandScene::default());
        host.layout(view).expect("layout");
        let idle = host.cx().take_delta();
        let (second, _) = host.emit_since(view, &idle, &first);
        dark.set(true);
        host.layout(view).expect("layout");
        let changed = host.cx().take_delta();
        let (third, diff) = host.emit_since(view, &changed, &second);
        assert!(!diff.unchanged(), "a theme change emitted no change");
        assert_eq!(
            third,
            host.emit(view),
            "a nested theme change copied stale paint"
        );
    });
}

#[test]
fn a_patched_row_drops_children_it_no_longer_has() {
    Runtime::scope("patching-orphans", || {
        let wide = signal(true);
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    let row = container().child(text("kept"));
                    vec![if wide.get() {
                        row.child(text("dropped"))
                    } else {
                        row
                    }]
                })
        });
        let mut router = EventRouter::new();
        let mut screen = Screen::new(SIZE.0, SIZE.1);
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], SIZE)
                .expect("frame");
        }
        wide.set(false);
        let shown = screen
            .frame(&mut host, &mut router, [], SIZE)
            .expect("frame")
            .text();
        assert!(
            shown.contains("kept") && !shown.contains("dropped"),
            "{shown}"
        );
    });
}

#[test]
fn tab_reaches_a_row_patched_into_focus_after_the_order_is_known() {
    Runtime::scope("patching-focus-order", || {
        let rows = signal(vec![
            (Row::Focusable, String::from("row 0")),
            (Row::Boxed, String::from("row 1")),
            (Row::Boxed, String::from("row 2")),
        ]);
        let (mut host, mut router, mut screen) = mounted(rows);
        screen
            .frame(&mut host, &mut router, [tab(false)], SIZE)
            .expect("frame");
        let first = router.focus();
        assert!(first.is_some(), "Tab found the focusable row");
        rows.update(|rows| rows[2].0 = Row::Focusable);
        screen
            .frame(&mut host, &mut router, [], SIZE)
            .expect("frame");
        screen
            .frame(&mut host, &mut router, [tab(false)], SIZE)
            .expect("frame");
        assert!(
            router.focus().is_some() && router.focus() != first,
            "Tab skipped the row patched into focus"
        );
    });
}

#[test]
fn a_row_patched_to_take_clicks_receives_them() {
    Runtime::scope("patching-click", || {
        let (armed, hits) = (signal(false), signal(0_u32));
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    let row = container()
                        .layout(LayoutStyle::block().fill_width().height(Host::CELL))
                        .child(text("target"));
                    vec![if armed.get() {
                        row.pointer_down(move |_| hits.update(|count| *count += 1))
                    } else {
                        row
                    }]
                })
        });
        let mut router = EventRouter::new();
        let mut screen = Screen::new(SIZE.0, SIZE.1);
        let at = Host::CELL / 2.0;
        for events in [Vec::new(), click(at, at).to_vec()] {
            screen
                .frame(&mut host, &mut router, events, SIZE)
                .expect("frame");
        }
        armed.set(true);
        for events in [Vec::new(), click(at, at).to_vec()] {
            screen
                .frame(&mut host, &mut router, events, SIZE)
                .expect("frame");
        }
        assert_eq!(hits.get(), 1, "the click missed the row patched to take it");
    });
}
