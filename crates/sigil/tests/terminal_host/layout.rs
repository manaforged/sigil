use sigil::Host;
use sigil::kernel::HostMode;
use sigil::kernel::{container, text};
use sigil::layout::LayoutStyle;
use sigil::reactive::{Owner, signal};
use sigil::widgets::Layout;

#[test]
fn a_frame_records_the_siblings_its_layout_moved() {
    Owner::scope("layout-delta", || {
        let lines = signal(vec![String::from("one")]);
        let mut host = Host::new(HostMode::Cell);
        let below = std::rc::Rc::new(std::cell::Cell::new(None));
        let seen = std::rc::Rc::clone(&below);
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .child(
                    container()
                        .layout(LayoutStyle::flex_column())
                        .children_reactive(move || {
                            lines.with(|lines| {
                                lines
                                    .iter()
                                    .map(|line| text(line.clone()))
                                    .collect::<Vec<_>>()
                            })
                        }),
                )
                .child(container().child(text("below")).added({
                    let seen = std::rc::Rc::clone(&seen);
                    move |_, id| seen.set(Some(id))
                }))
        });
        host.layout((320.0, 160.0)).expect("layout");
        drop(sigil::testing::take_delta(host.cx()));
        lines.set(vec![String::from("one"), String::from("two")]);
        host.layout((320.0, 160.0)).expect("layout");
        let delta = sigil::testing::take_delta(host.cx());
        let moved = below.get().expect("the sibling mounted");
        assert!(delta.layout.contains(&moved), "{delta:?}");
    });
}
