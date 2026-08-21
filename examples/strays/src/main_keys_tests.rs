use std::cell::Cell as StdCell;
use std::rc::Rc;
use std::time::Duration;

use super::*;
use crate::scan::Stray;
use crate::state::AppState;
use sigil::prelude::{LogicalKey, Shortcut};
use sigil_tui::{Host, key_down};

/// A bare `r` reaches the root through the capture phase, even though
/// focus sits on a widget inside the table.
#[test]
fn a_bare_key_reaches_the_root() {
    sigil::prelude::Runtime::scope("keys", || {
        let hits = Rc::new(StdCell::new(0_u32));
        let counter = Rc::clone(&hits);
        let mut host = Host::cells();
        let mut router = sigil_tui::EventRouter::new();
        host.mount(move |cx: &mut Cx| {
            let counter = Rc::clone(&counter);
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column())
                    .key_capture(Shortcut::plain(LogicalKey::character("r")), move |_| {
                        counter.set(counter.get() + 1);
                    })
                    .child(sigil::widgets::button("Focusable")),
            )
        });
        host.layout((480.0, 128.0)).expect("layout");
        router.focus_next(host.cx());
        sigil_tui::step(
            &mut host,
            &mut router,
            [key_down(LogicalKey::character("r"))],
            60,
            16,
        )
        .expect("layout");
        assert_eq!(hits.get(), 1, "the root must see a bare r");
    });
}

/// Pressing `k` with no row selected must not kill anything.
#[test]
fn kill_needs_a_selected_stray() {
    sigil::prelude::Runtime::scope("kill-guard", || {
        let state = AppState::from_rows(
            vec![Stray {
                pid: 1,
                pgid: 0,
                ports: vec![1],
                age: Duration::from_secs(99_999),
                rss: 0,
                cwd: Some(PathBuf::from("/dev/a")),
                cmd: "x".into(),
                tty: false,
            }],
            PathBuf::from("/dev"),
            Duration::from_secs(1),
        );
        assert_eq!(state.kill_selected(), None, "no selection, no kill");
    });
}
