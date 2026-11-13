use sigil::events::{Event, FileDropEvent, FileDropPhase};
use sigil::geometry::Point;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

#[test]
fn a_drop_reaches_only_the_pane_under_the_pointer() {
    Owner::scope("file-drop-route", || {
        let got = signal(Vec::<&'static str>::new());
        let pane = move |name: &'static str| {
            container()
                .layout(LayoutStyle::block().width(200.0).height(200.0))
                .file_drop(move |event: &FileDropEvent| {
                    if event.phase() == FileDropPhase::Dropped {
                        got.update(|seen| seen.push(name));
                    }
                })
        };
        let mut ui =
            Harness::new(move || row().child(pane("left")).child(pane("right"))).pixels(500, 220);
        ui.settle();
        ui.send([Event::pointer_move(Point::new(300.0, 100.0))]);
        ui.send([Event::file_drop(
            Some("/tmp/notes.txt".to_owned()),
            FileDropPhase::Dropped,
        )]);
        ui.settle();
        assert_eq!(got.peek(), vec!["right"]);
    });
}
