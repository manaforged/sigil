use super::*;

#[test]
fn order_reorders_items_by_ascending_order_field() {
    let a = StyleData {
        order: 10,
        ..fixed(40.0, 20.0)
    };
    let b = StyleData {
        order: -5,
        ..fixed(40.0, 20.0)
    };
    let boxes = kids(flex_box(300.0, 50.0), &[a, b]);
    assert!(near(boxes[1].x, 0.0));
    assert!(near(boxes[0].x, 40.0));
}

#[test]
fn display_none_and_absolute_items_leave_the_flow() {
    let hidden = StyleData {
        display: Display::None,
        ..fixed(40.0, 20.0)
    };
    let absolute = StyleData {
        position: Position::Absolute,
        ..fixed(40.0, 20.0)
    };
    for skipped in [hidden, absolute] {
        let mut f = LayoutForest::new();
        let parent = f.insert(None, flex_box(300.0, 100.0));
        add_child(&mut f, parent, skipped);
        let flow = add_child(&mut f, parent, fixed(60.0, 20.0));
        let x = run(&f, parent, 400.0, 400.0).boxed(flow).unwrap().x;
        assert!(near(x, 0.0), "x={x}");
    }
}

#[test]
fn container_box_picks_up_explicit_size() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, flex_box(220.0, 70.0));
    let child = add_child(&mut f, parent, fixed(40.0, 20.0));
    let s = run(&f, parent, 400.0, 400.0);
    let b = s.boxed(parent).unwrap();
    assert!(near(b.w, 220.0) && near(b.h, 70.0), "{b:?}");
    assert!(s.boxed(child).is_some());
}

#[test]
fn empty_flex_container_produces_only_self_box() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, flex_box(100.0, 50.0));
    assert!(run(&f, parent, 200.0, 100.0).boxed(parent).is_some());
}
