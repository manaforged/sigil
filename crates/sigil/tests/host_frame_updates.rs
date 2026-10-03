#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sigil::Host;
use sigil::app::{FrameHooks, FrameInputHook, set_theme};
use sigil::events::EventRouter;
use sigil::layout::LayoutResult;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::style::FocusRing;

const VIEW: (f32, f32) = (400.0, 200.0);

fn frame(host: &mut Host, router: &mut EventRouter, input: Option<FrameInputHook<'_>>) {
    let hooks = FrameHooks {
        input,
        ..FrameHooks::default()
    };
    let _ = host.frame(router, VIEW, 1.0 / 60.0, hooks);
}

fn text_color(host: &mut Host, label: &str) -> Rgba8 {
    host.emit(VIEW)
        .find_text(label)
        .expect("label is drawn")
        .run
        .color
}

#[test]
fn theme_switch_recolors_text_that_did_not_change() {
    Runtime::scope("host_frame_updates::theme_switch", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(text("Unchanged label")));
        set_theme(Theme::dark());
        frame(&mut host, &mut router, None);
        frame(&mut host, &mut router, None);
        assert_eq!(
            text_color(&mut host, "Unchanged label"),
            Theme::dark().on_surface
        );

        set_theme(Theme::light());
        frame(&mut host, &mut router, None);

        assert_eq!(
            text_color(&mut host, "Unchanged label"),
            Theme::light().on_surface
        );
    });
}

#[test]
fn theme_set_during_input_paints_in_the_same_frame() {
    Runtime::scope("host_frame_updates::theme_in_input", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(text("Label")));
        set_theme(Theme::dark());
        frame(&mut host, &mut router, None);
        let painted = std::cell::Cell::new(None);

        let hooks = FrameHooks {
            input: Some(Box::new(|_, _| {
                set_theme(Theme::light());
                None
            })),
            paint: Some(Box::new(|target, _, _| {
                painted.set(Some(target.theme().on_surface));
            })),
            ..FrameHooks::default()
        };
        let _ = host.frame(&mut router, VIEW, 1.0 / 60.0, hooks);

        assert_eq!(painted.get(), Some(Theme::light().on_surface));
    });
}

#[test]
fn input_hook_viewport_sizes_the_same_frame() {
    Runtime::scope("host_frame_updates::input_viewport", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().layout(LayoutStyle::flex_column().fill_width().fill_height()));
        frame(
            &mut host,
            &mut router,
            Some(Box::new(|_, _| Some((640.0, 90.0)))),
        );

        let root = host.root().expect("mounted");
        let rect = host
            .cx()
            .get(root)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .expect("laid out")
            .rect();

        assert_eq!((rect.width, rect.height), (640.0, 90.0));
    });
}

#[test]
fn automatic_window_focus_shows_no_focus_ring() {
    Runtime::scope("host_frame_updates::auto_focus_ring", || {
        let mut host = Host::new();
        let mut router = EventRouter::new();
        host.mount(column().child(button("Save")));
        frame(&mut host, &mut router, None);

        let focused = router.focus().expect("the first control takes focus");
        let ring = host
            .cx()
            .get(focused)
            .and_then(|node| node.get_extension::<FocusRing>())
            .map(FocusRing::visible);

        assert_eq!(ring, Some(false));
    });
}
