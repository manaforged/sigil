use super::*;
use crate::kernel::container;
use crate::layout::*;

fn ring_of_three(cx: &mut Cx) {
    let mut root = container().layout(LayoutStyle::block().fixed_size(100.0, 100.0));
    for _ in 0..3 {
        root = root.child(
            container()
                .focusable(true)
                .layout(LayoutStyle::block().fixed_size(10.0, 10.0)),
        );
    }
    let _ = cx.add(root);
}

#[test]
fn tab_edges_mark_the_ends_of_the_ring() {
    let mut cx = Cx::new_root();
    ring_of_three(&mut cx);
    let mut focus = FocusManager::default();

    focus.focus_next(&mut cx, None);
    assert_eq!(
        focus.tab_edges(&cx),
        TabEdges {
            first: true,
            last: false
        }
    );
    focus.focus_next(&mut cx, None);
    assert_eq!(
        focus.tab_edges(&cx),
        TabEdges {
            first: false,
            last: false
        }
    );
    focus.focus_next(&mut cx, None);
    assert_eq!(
        focus.tab_edges(&cx),
        TabEdges {
            first: false,
            last: true
        }
    );
}

#[test]
fn tab_edges_release_both_keys_without_focusable_nodes() {
    let mut cx = Cx::new_root();
    let _ = cx.add(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0)));
    let focus = FocusManager::default();

    assert_eq!(
        focus.tab_edges(&cx),
        TabEdges {
            first: true,
            last: true
        }
    );
}
