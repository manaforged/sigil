use super::*;

#[test]
fn a_still_tree_reports_no_movement() {
    use sigil::Host;

    sigil::reactive::Runtime::scope("still", || {
        let mut host = Host::cells();
        host.mount(move || container().child(text("steady")));
        host.layout((320.0, 64.0)).expect("layout");
        let first = host.emit((320.0, 64.0));
        assert!(
            first.find_text("steady").is_some(),
            "the first paint draws the tree"
        );
        host.layout((320.0, 64.0)).expect("layout");
        assert_eq!(
            first,
            host.emit((320.0, 64.0)),
            "a tree nobody touched must report no movement"
        );
    });
}

#[test]
fn a_reactive_class_change_repaints() {
    use sigil::Host;
    use sigil::reactive::signal;
    use sigil::style::{NodeStyleExt, Paint, Token};

    fn fills(scene: &sigil::scene::CommandScene) -> Vec<sigil::style::Rgba8> {
        scene
            .commands
            .iter()
            .filter_map(|command| match command {
                sigil::scene::SceneCommand::Rect(rect) => Some(rect.color),
                _ => None,
            })
            .collect()
    }

    fn truth(start: bool) -> Vec<sigil::style::Rgba8> {
        sigil::reactive::Runtime::scope("reactive-truth", || {
            let active = signal(start);
            let mut host = Host::cells();
            host.mount(move || {
                container()
                    .paint_when(active, |s| s.bg(Token::Accent))
                    .child(text("cell"))
            });
            host.layout((320.0, 64.0)).expect("layout");
            fills(&host.emit((320.0, 64.0)))
        })
    }
    let cold_truth = truth(false);
    let hot_truth = truth(true);
    assert_ne!(
        cold_truth, hot_truth,
        "control: the class must resolve to a different fill at all"
    );

    sigil::reactive::Runtime::scope("reactive-class", || {
        let active = signal(false);
        let mut host = Host::cells();
        host.mount(move || {
            container()
                .paint_when(active, |s| s.bg(Token::Accent))
                .child(text("cell"))
        });
        host.layout((320.0, 64.0)).expect("layout");
        let first = host.emit((320.0, 64.0));
        let before = fills(&first);

        active.set(true);
        host.layout((320.0, 64.0)).expect("layout");
        let next = host.emit((320.0, 64.0));
        let moved = next != first;
        let after = fills(&next);

        assert_ne!(before, after, "the reactive class must change the fill");
        assert!(
            moved,
            "a changed fill must report movement, or nothing is sent"
        );
    });
}

#[test]
fn a_skipped_frame_returns_what_is_showing() {
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::terminal::Screen;

    sigil::reactive::Runtime::scope("skip-grid", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = Screen::new(30, 6);
        host.mount(move || container().child(text("onscreen")));

        let drawn = screen
            .frame(&mut host, &mut router, [], (30, 6))
            .expect("layout")
            .text();
        assert!(drawn.contains("onscreen"), "first frame paints:\n{drawn}");
        let mut sink = std::io::sink();
        screen.flush(&mut sink).expect("flush");

        let again = screen
            .frame(&mut host, &mut router, [], (30, 6))
            .expect("layout")
            .text();
        assert_eq!(drawn, again, "a skipped frame must report the screen");
    });
}

#[test]
fn only_a_change_sends_bytes() {
    use sigil::Host;
    use sigil::events::EventRouter;
    use sigil::reactive::signal;
    use sigil::terminal::Screen;

    sigil::reactive::Runtime::scope("bytes", || {
        let count = signal(0_i32);
        let label = sigil::reactive::derived(move || format!("n={}", count.get()));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        let mut screen = Screen::new(30, 6);
        host.mount(move || container().child(text(label)));

        let sent = |next: &mut Screen, app: &mut Host, events: &mut EventRouter| {
            next.frame(app, events, [], (30, 6)).expect("layout");
            let mut out: Vec<u8> = Vec::new();
            next.flush(&mut out).expect("flush");
            out.len()
        };

        assert!(
            sent(&mut screen, &mut host, &mut router) > 0,
            "first frame draws"
        );
        assert_eq!(
            sent(&mut screen, &mut host, &mut router),
            0,
            "an unchanged frame sends nothing"
        );

        count.set(1);
        assert!(
            sent(&mut screen, &mut host, &mut router) > 0,
            "a changed label must reach the terminal"
        );
        assert_eq!(
            sent(&mut screen, &mut host, &mut router),
            0,
            "and then settle again"
        );
    });
}

#[test]
fn a_width_only_resize_repaints_at_the_new_width() {
    sigil::reactive::Runtime::scope("frames-resize", || {
        let mut host = sigil::Host::cells();
        host.mount(|| sigil::kernel::text("fixed"));
        let mut router = sigil::events::EventRouter::new();
        let mut screen = sigil::terminal::Screen::new(20, 5);
        for _ in 0..2 {
            screen
                .frame(&mut host, &mut router, [], (20, 5))
                .expect("frame");
            screen.flush(&mut std::io::sink()).expect("flush");
        }
        let wider = screen
            .frame(&mut host, &mut router, [], (30, 5))
            .expect("frame")
            .cols();
        assert_eq!(wider, 30);
        let taller = screen
            .frame(&mut host, &mut router, [], (30, 8))
            .expect("frame")
            .rows();
        assert_eq!(taller, 8);
    });
}
