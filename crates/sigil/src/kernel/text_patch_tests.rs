use crate::kernel::{Cx, container, text};
use crate::layout::LayoutStyle;
use crate::reactive::{Owner, signal};
use crate::widgets::{Layout, Widget};

#[test]
fn a_patched_text_node_stays_in_the_tree() {
    Owner::scope("text-patch", || {
        let lines = signal(vec!["top".to_string()]);
        let mut host = crate::Host::new(crate::kernel::HostMode::Cell);
        host.mount(move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width())
                .children_reactive(move || {
                    lines.with(|lines| {
                        lines
                            .iter()
                            .map(|line| text(line.clone()))
                            .collect::<Vec<_>>()
                    })
                })
        });
        host.layout((320.0, 96.0)).expect("layout");
        let before = describe(&mut host);
        lines.set(vec!["a".to_string()]);
        host.layout((320.0, 96.0)).expect("layout");
        let after = describe(&mut host);
        let scene = host.emit((320.0, 96.0));
        assert!(
            scene.find_text("a").is_some(),
            "scene {scene:?}\nafter {after:#?}"
        );
        assert!(
            after.iter().any(|line| line.contains("\"a\"")),
            "before {before:#?}\nafter {after:#?}"
        );
    });
}

#[test]
fn map_node_sees_a_width_set_after_the_call() {
    Owner::scope("text-map-node-order", || {
        let seen = std::rc::Rc::new(std::cell::Cell::new(0));
        let flag = seen.clone();
        let mut cx = Cx::new_root();
        let _ = cx.add(
            text("hi")
                .map_node(move |node| {
                    flag.set(node.pre_extensions.len());
                    node
                })
                .width(40.0),
        );
        assert_eq!(seen.get(), 1);
    });
}

fn describe(host: &mut crate::Host) -> Vec<String> {
    let cx = host.cx();
    let mut out = Vec::new();
    let mut stack: Vec<crate::kernel::NodeId> = cx.roots().to_vec();
    while let Some(id) = stack.pop() {
        if let Some(node) = cx.get(id) {
            out.push(format!(
                "{id:?} kind={:?} parent={:?} children={:?} version={}",
                node.kind,
                node.parent,
                node.children,
                node.version()
            ));
            stack.extend(node.children.iter().copied());
        } else {
            out.push(format!("{id:?} missing"));
        }
    }
    out
}

#[test]
fn text_bound_to_a_signal_outlives_the_signals_owner() {
    Owner::scope("text-owner-gone", || {
        let owner = crate::reactive::Owner::new("label", crate::reactive::Owner::current());
        let label = owner.with(|| signal("a".to_string()));
        let mut host = crate::Host::new(crate::kernel::HostMode::Cell);
        host.mount(move || text(label));
        host.layout((320.0, 96.0)).expect("layout");
        crate::reactive::batch(|| {
            label.set("b".to_string());
            owner.dispose();
        });
        host.layout((320.0, 96.0)).expect("layout");
    });
}
