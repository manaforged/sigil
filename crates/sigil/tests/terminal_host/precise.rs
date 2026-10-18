use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::{LayoutBuilderExt, Paint};
use sigil::reactive::{Runtime, signal};
use sigil::terminal::{Caps, Glyphs, Screen};

use super::oracle::{COLS, Model, ROWS, tree};

#[test]
fn an_unchanged_frame_writes_nothing() {
    Runtime::scope("precise-idle", || {
        let state = signal(Model {
            lines: (0..ROWS).map(|row| format!("row {row}")).collect(),
            ..Model::default()
        });
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || tree(state));
        let mut screen = Screen::new(COLS, ROWS);
        for _ in 0..3 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        assert!(!screen.needs_flush(), "an idle frame asked to be written");
        let mut written = Vec::new();
        screen.flush(&mut written).expect("flush");
        assert!(written.is_empty(), "an idle frame wrote {written:?}");
    });
}

#[test]
fn ascii_glyphs_hold_on_full_and_region_frames() {
    Runtime::scope("precise-ascii", || {
        let bullet = signal(String::from("• one"));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width().fill_height())
                .child(text(bullet).nowrap())
                .child(text("steady … line").nowrap())
        });
        let caps = Caps {
            glyphs: Glyphs::Ascii,
            ..Caps::default()
        };
        let mut screen = Screen::with_caps(COLS, ROWS, caps);
        let shown = |view: &mut Screen, embed: &mut Host, route: &mut EventRouter| {
            let text = view
                .frame(embed, route, [], (COLS, ROWS))
                .expect("frame")
                .text();
            view.flush(&mut std::io::sink()).expect("flush");
            text
        };
        let first = shown(&mut screen, &mut host, &mut router);
        assert!(
            first.contains("* one") && first.contains("steady . line"),
            "{first}"
        );
        shown(&mut screen, &mut host, &mut router);
        bullet.set(String::from("• two"));
        let second = shown(&mut screen, &mut host, &mut router);
        assert!(second.contains("* two"), "{second}");
        assert!(!second.contains('•') && !second.contains('…'), "{second}");
    });
}

#[test]
fn a_change_hidden_behind_a_cover_writes_nothing() {
    use sigil::layout::Position;
    use sigil::style::{NodeStyleExt, Rgba8};

    let cell = Host::CELL;
    let place = || {
        LayoutStyle::block()
            .position(Position::Absolute)
            .inset_left(2.0 * cell)
            .inset_top(2.0 * cell)
            .width(6.0 * cell)
            .height(2.0 * cell)
    };
    let tone = |r: u8| Rgba8 {
        r,
        g: 60,
        b: 60,
        a: 255,
    };
    Runtime::scope("precise-hidden", || {
        let warm = signal(false);
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move || {
            container()
                .layout(LayoutStyle::block().fill_width().fill_height())
                .child(
                    container()
                        .layout(place())
                        .paint_when(warm, move |style| style.bg(tone(200)))
                        .bg(tone(20)),
                )
                .child(container().layout(place()).bg(tone(90)))
        });
        let mut screen = Screen::new(COLS, ROWS);
        for _ in 0..3 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        warm.set(true);
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        assert!(
            !screen.needs_flush(),
            "a change no cell shows asked to be written"
        );
    });
}

fn linked(url: sigil::prelude::Signal<String>, tick: sigil::prelude::Signal<u32>) -> Host {
    let mut host = Host::cells();
    host.mount(move || {
        container().children_reactive(move || {
            vec![
                container().child(text("docs").description(url.get())),
                container().child(text(format!("n {}", tick.get()))),
            ]
        })
    });
    host
}

#[test]
fn a_changed_link_target_reaches_the_terminal() {
    for other in [false, true] {
        Runtime::scope("precise-link", || {
            let (url, tick) = (signal(String::from("https://a.test")), signal(0_u32));
            let mut host = linked(url, tick);
            let mut router = EventRouter::new();
            let caps = Caps {
                hyperlinks: true,
                ..Caps::default()
            };
            let mut screen = Screen::with_caps(COLS, ROWS, caps);
            for _ in 0..3 {
                screen
                    .frame(&mut host, &mut router, [], (COLS, ROWS))
                    .expect("frame");
                screen.flush(&mut std::io::sink()).expect("flush");
            }
            url.set(String::from("https://b.test"));
            if other {
                tick.set(1);
            }
            let mut written = Vec::new();
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut written).expect("flush");
            let written = String::from_utf8_lossy(&written);
            assert!(
                written.contains("https://b.test"),
                "other change {other}: {written:?}"
            );
        });
    }
}

#[test]
fn a_status_spoken_on_a_quiet_frame_is_written() {
    Runtime::scope("precise-status", || {
        let mut host = Host::cells();
        host.mount(|| container().child(text("steady")));
        let mut router = EventRouter::new();
        let mut screen = Screen::new(COLS, ROWS);
        screen.set_access(sigil::terminal::Access {
            status: std::time::Duration::from_secs(60),
            ..sigil::terminal::Access::default()
        });
        for _ in 0..3 {
            screen
                .frame(&mut host, &mut router, [], (COLS, ROWS))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        sigil::kernel::announce("saved");
        let mut written = Vec::new();
        screen
            .frame(&mut host, &mut router, [], (COLS, ROWS))
            .expect("frame");
        screen.flush(&mut written).expect("flush");
        let written = String::from_utf8_lossy(&written);
        assert!(written.contains("saved"), "{written:?}");
    });
}
