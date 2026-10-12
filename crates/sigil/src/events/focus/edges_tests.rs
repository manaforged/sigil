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

fn edges(focus: &FocusManager, cx: &Cx) -> (bool, bool) {
    let TabEdges { first, last } = focus.tab_edges(cx);
    (first, last)
}

#[test]
fn tab_edges_mark_the_ends_of_the_ring() {
    let mut cx = Cx::new_root();
    ring_of_three(&mut cx);
    let mut focus = FocusManager::default();
    for expected in [(true, false), (false, false), (false, true)] {
        focus.focus_next(&mut cx, None);
        assert_eq!(edges(&focus, &cx), expected);
    }
}

#[test]
fn tab_edges_release_both_keys_without_focusable_nodes() {
    let mut cx = Cx::new_root();
    let _ = cx.add(container().layout(LayoutStyle::block().fixed_size(10.0, 10.0)));
    assert_eq!(edges(&FocusManager::default(), &cx), (true, true));
}
