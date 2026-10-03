#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::EventRouter;
use sigil::layout::LayoutResult;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::ButtonBuilder;

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host, router: &mut EventRouter) {
    for _ in 0..3 {
        let _ = host.frame(router, VIEW, 1.0 / 60.0, FrameHooks::default());
    }
}

fn drawn(host: &mut Host, label: &str) -> bool {
    host.emit(VIEW).find_text(label).is_some()
}

#[test]
fn diff_redraws_when_its_patch_signal_changes() {
    Runtime::scope("api_one_spelling::diff", || {
        let patch = signal(String::from("@@ -1 +1 @@\n-before\n+first\n"));
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(diff(patch)));
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "first"));

        patch.set(String::from("@@ -1 +1 @@\n-before\n+second\n"));
        settle(&mut host, &mut router);

        assert!(drawn(&mut host, "second"));
        assert!(!drawn(&mut host, "first"));
    });
}

#[test]
fn sparkline_redraws_when_its_values_change() {
    Runtime::scope("api_one_spelling::sparkline", || {
        let values = signal(vec![1.0_f32, 1.0, 1.0, 1.0]);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(sparkline(values).range(0.0, 8.0)));
        settle(&mut host, &mut router);
        let before = host.emit(VIEW);

        values.set(vec![8.0, 1.0, 8.0, 1.0]);
        settle(&mut host, &mut router);

        assert_ne!(host.emit(VIEW), before);
    });
}

#[test]
fn visible_when_follows_a_plain_closure() {
    Runtime::scope("api_one_spelling::visible_when", || {
        let shown = signal(true);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(text("Toggled").visible_when(move || shown.get())));
        settle(&mut host, &mut router);
        assert!(drawn(&mut host, "Toggled"));

        shown.set(false);
        settle(&mut host, &mut router);

        assert!(!drawn(&mut host, "Toggled"));
    });
}

#[test]
fn virtual_list_rows_can_be_any_node_and_invalid_when_takes_a_closure() {
    Runtime::scope("api_one_spelling::rows_and_invalid", || {
        let email = signal(String::new());
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(
            column()
                .child(virtual_list(
                    vec![String::from("Row text")],
                    |_, row: &String| text(row.clone()),
                ))
                .child(field(
                    "Email",
                    text_field(email).invalid_when(move || !email.get().contains('@')),
                )),
        );
        settle(&mut host, &mut router);

        assert!(drawn(&mut host, "Row text"));
    });
}

#[test]
fn button_width_takes_text_relative_units() {
    Runtime::scope("api_one_spelling::button_width", || {
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(|| column().child(button("Go").width(cols(12.0))));
        settle(&mut host, &mut router);

        let root = host.root().expect("mounted");
        let cx = host.cx();
        let column = cx.get(root).expect("root node");
        let button_id = *column.children().first().expect("button child");
        let width = cx
            .get(button_id)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .expect("button laid out")
            .rect()
            .width;

        assert_eq!(width, 12.0 * Host::CELL);
    });
}

#[test]
fn chip_tone_follows_a_closure() {
    Runtime::scope("api_one_spelling::chip_tone", || {
        let alert = signal(false);
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(
            chip("State", Tone::Info).tone(
                move || {
                    if alert.get() { Tone::Error } else { Tone::Info }
                },
            ),
        ));
        settle(&mut host, &mut router);
        let calm = host.emit(VIEW);

        alert.set(true);
        settle(&mut host, &mut router);

        assert_ne!(host.emit(VIEW), calm);
    });
}

#[test]
fn button_tone_and_fill_do_not_depend_on_call_order() {
    Runtime::scope("api_one_spelling::button_order", || {
        let scene = |make: fn() -> ButtonBuilder| {
            let mut host = Host::new();
            let mut router = EventRouter::new();
            host.mount(move || column().child(make()));
            settle(&mut host, &mut router);
            host.emit(VIEW)
        };
        assert_eq!(
            scene(|| button("Go").ghost().primary()),
            scene(|| button("Go").primary().ghost())
        );
        assert_eq!(
            scene(|| button("Go").outline().destructive()),
            scene(|| button("Go").destructive().outline())
        );
        assert_ne!(
            scene(|| button("Go").primary().ghost()),
            scene(|| button("Go").ghost())
        );
    });
}
