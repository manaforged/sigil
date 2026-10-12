use super::*;

fn wrapping(wrap: FlexWrap, align: Option<AlignContent>) -> StyleData {
    StyleData {
        flex_wrap: wrap,
        align_content: align,
        ..flex_box(150.0, 200.0)
    }
}

fn rigid(h: f32) -> StyleData {
    StyleData {
        flex_shrink: 0.0,
        ..fixed(80.0, h)
    }
}

fn line_ys(parent: StyleData, count: usize, h: f32) -> Vec<f32> {
    let items = vec![rigid(h); count];
    kids(parent, &items).iter().map(|b| b.y).collect()
}

fn assert_ys(got: &[f32], want: &[f32]) {
    assert!(
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| near(*g, *w)),
        "ys={got:?} want {want:?}"
    );
}

#[test]
fn nowrap_keeps_overflowing_items_on_single_line() {
    let parent = StyleData {
        flex_wrap: FlexWrap::NoWrap,
        ..flex_box(150.0, 100.0)
    };
    assert_ys(&line_ys(parent, 3, 20.0), &[0.0, 0.0, 0.0]);
}

#[test]
fn wrap_breaks_overflowing_items_onto_next_line() {
    let parent = wrapping(FlexWrap::Wrap, None);
    assert_ys(&line_ys(parent, 3, 30.0), &[0.0, 30.0, 60.0]);
}

#[test]
fn wrap_reverse_orders_lines_from_cross_end() {
    let parent = wrapping(FlexWrap::WrapReverse, Some(AlignContent::Start));
    assert_ys(&line_ys(parent, 2, 30.0), &[30.0, 0.0]);
}

#[test]
fn align_content_places_lines_on_cross_axis() {
    for (align, want) in [
        (AlignContent::Center, [70.0, 100.0]),
        (AlignContent::End, [140.0, 170.0]),
    ] {
        let parent = wrapping(FlexWrap::Wrap, Some(align));
        assert_ys(&line_ys(parent, 2, 30.0), &want);
    }
}

#[test]
fn gap_inserts_main_axis_gutter_between_items() {
    let parent = StyleData {
        gap: StyleSize::new(LengthPercentage::Px(8.0), LengthPercentage::Px(0.0)),
        ..flex_box(300.0, 100.0)
    };
    let b = kids(parent, &[fixed(40.0, 20.0), fixed(40.0, 20.0)]);
    assert!(near(b[0].x, 0.0));
    assert!(near(b[1].x, 48.0), "b.x={}", b[1].x);
}
