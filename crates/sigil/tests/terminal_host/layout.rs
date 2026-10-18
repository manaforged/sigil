use sigil::Host;
use sigil::kernel::{container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::LayoutBuilderExt;
use sigil::reactive::{Runtime, signal};

#[test]
fn a_frame_records_the_siblings_its_layout_moved() {
    Runtime::scope("layout-delta", || {
        let lines = signal(vec![String::from("one")]);
        let mut host = Host::cells();
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
                .child(container().child(text("below")).on_added({
                    let seen = std::rc::Rc::clone(&seen);
                    move |_, id| seen.set(Some(id))
                }))
        });
        host.layout((320.0, 160.0)).expect("layout");
        drop(host.cx().take_delta());
        lines.set(vec![String::from("one"), String::from("two")]);
        host.layout((320.0, 160.0)).expect("layout");
        let delta = host.cx().take_delta();
        let moved = below.get().expect("the sibling mounted");
        assert!(delta.layout.contains(&moved), "{delta:?}");
    });
}
