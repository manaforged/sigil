use super::*;

fn direction(dir: FlexDirection, w: f32, h: f32) -> StyleData {
    StyleData {
        flex_direction: dir,
        ..flex_box(w, h)
    }
}

fn basis(px: f32, grow: f32, shrink: f32) -> StyleData {
    StyleData {
        flex_basis: Dimension::Px(px),
        flex_grow: grow,
        flex_shrink: shrink,
        ..sized(Dimension::Auto, Dimension::Px(20.0))
    }
}

#[test]
fn row_lays_children_along_inline_axis() {
    let b = kids(
        flex_box(300.0, 100.0),
        &[fixed(40.0, 30.0), fixed(60.0, 30.0)],
    );
    assert!(near(b[0].x, 0.0) && near(b[0].y, 0.0), "a={:?}", b[0]);
    assert!(near(b[1].x, 40.0) && near(b[1].y, 0.0), "b={:?}", b[1]);
}

#[test]
fn grow_item_is_not_start_packed() {
    let grown = StyleData {
        flex_grow: 1.0,
        ..fixed(40.0, 40.0)
    };
    let b = kids(flex_box(300.0, 40.0), &[fixed(40.0, 40.0), grown])[1];
    assert!(
        b.w > 200.0,
        "grow item must take leftover main space, w={}",
        b.w
    );
}

#[test]
fn column_lays_children_along_block_axis() {
    let parent = direction(FlexDirection::Column, 200.0, 300.0);
    let b = kids(parent, &[fixed(40.0, 30.0), fixed(40.0, 50.0)]);
    assert!(near(b[0].y, 0.0));
    assert!(near(b[1].y, 30.0), "b.y={}", b[1].y);
}

#[test]
fn row_reverse_reverses_main_order() {
    let parent = direction(FlexDirection::RowReverse, 200.0, 100.0);
    let b = kids(parent, &[fixed(40.0, 20.0), fixed(60.0, 20.0)]);
    assert!(near(b[1].x, 0.0), "b.x={}", b[1].x);
    assert!(near(b[0].x, 60.0), "a.x={}", b[0].x);
}

#[test]
fn column_reverse_reverses_block_order() {
    let parent = direction(FlexDirection::ColumnReverse, 100.0, 200.0);
    let b = kids(parent, &[fixed(40.0, 30.0), fixed(40.0, 50.0)]);
    assert!(near(b[1].y, 0.0));
    assert!(near(b[0].y, 50.0), "a.y={}", b[0].y);
}

#[test]
fn flex_basis_overrides_size_for_main_axis() {
    let item = StyleData {
        flex_basis: Dimension::Px(80.0),
        ..fixed(10.0, 20.0)
    };
    let b = kids(flex_box(300.0, 100.0), &[item])[0];
    assert!(near(b.w, 80.0), "w={}", b.w);
}

#[test]
fn flex_grow_distributes_free_space_proportionally() {
    let b = kids(
        flex_box(300.0, 100.0),
        &[basis(0.0, 1.0, 1.0), basis(0.0, 2.0, 1.0)],
    );
    assert!(near(b[0].w, 100.0), "a.w={}", b[0].w);
    assert!(near(b[1].w, 200.0), "b.w={}", b[1].w);
}

#[test]
fn flex_shrink_distributes_deficit_proportionally() {
    let item = basis(80.0, 0.0, 1.0);
    let b = kids(flex_box(100.0, 50.0), &[item.clone(), item]);
    assert!(near(b[0].w, 50.0), "a.w={}", b[0].w);
    assert!(near(b[1].w, 50.0), "b.w={}", b[1].w);
}

#[test]
fn no_grow_no_shrink_keeps_basis() {
    let b = kids(flex_box(400.0, 50.0), &[basis(60.0, 0.0, 0.0)])[0];
    assert!(near(b.w, 60.0));
}
