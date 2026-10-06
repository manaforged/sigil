#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::events::{FileDropEvent, FileDropPhase, ShellEvent};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;
use sigil_kittest::input::pointer_move;

#[test]
fn a_drop_reaches_only_the_pane_under_the_pointer() {
    Runtime::scope("file-drop-route", || {
        let got = signal(Vec::<&'static str>::new());
        let pane = move |name: &'static str| {
            container()
                .layout(LayoutStyle::block().width(200.0).height(200.0))
                .file_drop(move |event: &FileDropEvent| {
                    if event.phase == FileDropPhase::Dropped {
                        got.update(|seen| seen.push(name));
                    }
                })
        };
        let mut ui =
            Harness::new(move || row().child(pane("left")).child(pane("right"))).pixels(500, 220);
        ui.settle();
        ui.send([pointer_move(300.0, 100.0)]);
        ui.send([ShellEvent::FileDrop(FileDropEvent {
            path: Some("/tmp/notes.txt".to_owned()),
            phase: FileDropPhase::Dropped,
        })]);
        ui.settle();
        assert_eq!(got.peek(), vec!["right"]);
    });
}
