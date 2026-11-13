#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::cell::Cell;
use std::rc::Rc;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::{Event, Key, KeyPhase, NamedKey, PhysicalKey};
use sigil::kernel::{ImageSource, NodeId, image};
use sigil::layout::LayoutResult;
use sigil::prelude::*;
use sigil::reactive::Owner;

const VIEW: (f32, f32) = (480.0, 320.0);

fn settle(host: &mut Host) {
    for _ in 0..3 {
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
    }
}

fn tab_key() -> Event {
    Event::key(Key::Named(NamedKey::Tab), KeyPhase::Down).physical(PhysicalKey::Tab)
}

fn first_child_height(host: &mut Host) -> f32 {
    let root = host.root().expect("mounted");
    let cx = host.cx();
    let child = *cx
        .get(root)
        .expect("root")
        .children()
        .first()
        .expect("child");
    cx.get(child)
        .and_then(|node| node.extension::<LayoutResult>())
        .expect("laid out")
        .rect()
        .height
}

#[test]
fn a_tab_capture_handler_runs_before_focus_moves() {
    Owner::scope("run_defects::tab", || {
        let captured = Rc::new(Cell::new(0_u32));
        let second_focused = Rc::new(Cell::new(false));
        let mut host = Host::new(HostMode::Pixel);
        let (count, moved) = (Rc::clone(&captured), Rc::clone(&second_focused));
        host.mount(
            column()
                .key_capture(NamedKey::Tab, move |event| {
                    count.set(count.get() + 1);
                    event.stop_propagation();
                })
                .child(button("First").autofocus())
                .child(button("Second").focus_in(move || moved.set(true))),
        );
        settle(&mut host);
        let _ = host.frame(
            VIEW,
            1.0 / 60.0,
            FrameHooks::default().input(|target| {
                target.route(tab_key());
                None
            }),
        );
        settle(&mut host);

        assert_eq!(captured.get(), 1);
        assert!(!second_focused.get());
    });
}

#[test]
fn follow_row_keeps_its_row_when_rows_arrive_later() {
    Owner::scope("run_defects::follow_row", || {
        let items = signal(Vec::<String>::new());
        let scroll = signal(0.0_f32);
        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            column().child(
                virtual_list(items, |_, row: &String| text(row.clone()))
                    .row_height(20.0)
                    .viewport_height(100.0)
                    .scroll_y(scroll)
                    .follow_row(signal(0)),
            ),
        );
        settle(&mut host);
        items.set((0..50).map(|n| format!("Row {n}")).collect());
        settle(&mut host);

        assert_eq!(scroll.get(), 0.0);
        assert!(host.emit(VIEW).find_text("Row 0").is_some());
    });
}

#[derive(Clone)]
struct Proc {
    name: &'static str,
}

#[test]
fn a_selected_row_the_filter_hides_leaves_the_selection() {
    Owner::scope("run_defects::table_filter", || {
        let state = TableState::new(
            vec![Proc { name: "postgres" }, Proc { name: "sshd" }],
            vec![
                TableColumn::new("name", "Name").filterable(|p: &Proc, q: &str| p.name.contains(q)),
            ],
        );
        state.select(0);
        assert_eq!(state.selected_row().map(|p| p.name), Some("postgres"));

        state.quick_filter().set(String::from("sh"));

        assert!(state.selected_row().is_none());
        state.select_by(1);
        assert_eq!(state.selected_row().map(|p| p.name), Some("sshd"));
    });
}

#[test]
fn text_edit_and_dropdown_default_to_cell_sized_heights_in_a_terminal() {
    Owner::scope("run_defects::cell_heights", || {
        let mut editor = Host::new(HostMode::Cell);
        editor.mount(|| column().child(text_edit(signal(String::new()))));
        settle(&mut editor);
        assert_eq!(first_child_height(&mut editor), 5.0 * Host::CELL);

        let mut select = Host::new(HostMode::Cell);
        select.mount(|| column().child(dropdown(signal(0), vec![String::from("sonnet")])));
        settle(&mut select);
        assert_eq!(first_child_height(&mut select), 3.0 * Host::CELL);
    });
}

#[test]
fn a_closeable_tab_leaves_a_gap_before_its_close_button() {
    Owner::scope("run_defects::tab_gap", || {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(
            column().child(
                tabs()
                    .labels(signal(vec![String::from("Alpha")]))
                    .closeable(true),
            ),
        );
        settle(&mut host);
        let scene = host.emit(VIEW);
        let label = scene.find_text("Alpha").expect("label").rect;
        let close = scene.find_text("\u{00D7}").expect("close").rect;

        assert!(
            close.origin.x >= label.origin.x + label.width + 4.0,
            "{label:?} {close:?}"
        );
    });
}

fn painted(make: impl Fn(bool) -> NodeBuilder + Copy + 'static) -> bool {
    let scene = |paint: bool| {
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || column().child(make(paint)));
        settle(&mut host);
        host.emit(VIEW)
    };
    scene(true) != scene(false)
}

#[test]
fn every_layout_builder_takes_paint() {
    Owner::scope("run_defects::paint", || {
        assert!(painted(|paint| {
            let side = sidebar(text("Files"));
            container().child(if paint {
                side.background(Token::Accent)
            } else {
                side
            })
        }));
        assert!(painted(|paint| {
            let list = virtual_list(vec![String::from("Row")], |_, row: &String| {
                text(row.clone())
            });
            container().child(if paint {
                list.background(Token::Accent)
            } else {
                list
            })
        }));
        assert!(painted(|paint| {
            let split = splitter(Orientation::Horizontal)
                .pane(text("Left"))
                .commit();
            container().child(if paint {
                split.background(Token::Accent)
            } else {
                split
            })
        }));
        assert!(painted(|paint| {
            let plot = chart([1.0, 2.0, 3.0]);
            container().child(if paint {
                plot.background(Token::Accent)
            } else {
                plot
            })
        }));
        assert!(painted(|paint| {
            let options = segmented(signal(0), vec![String::from("One")]);
            container().child(if paint {
                options.background(Token::Accent)
            } else {
                options
            })
        }));
    });
}

#[test]
fn an_image_takes_a_radius() {
    Owner::scope("run_defects::image_radius", || {
        let pixels = ImageSource::from_rgba8(1, 1, vec![255, 0, 0, 255]).expect("pixels");
        let _: NodeId = {
            let mut cx = sigil::kernel::Cx::new_root();
            cx.add(image(pixels).radius(Radius::Md))
        };
    });
}
