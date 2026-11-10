use super::*;

fn root_box(style: StyleData, w: f32, h: f32) -> (f32, f32) {
    let mut f = LayoutForest::new();
    let root = f.insert(None, style);
    let b = run(&f, root, w, h).boxed(root).unwrap();
    (b.w, b.h)
}

#[test]
fn box_sizing_decides_whether_padding_adds() {
    for (sizing, w, h) in [
        (BoxSizing::BorderBox, 100.0, 50.0),
        (BoxSizing::ContentBox, 120.0, 70.0),
    ] {
        let style = StyleData {
            padding: Edges::splat(Length::Px(10.0)),
            box_sizing: sizing,
            ..px(100.0, 50.0)
        };
        let (bw, bh) = root_box(style, 400.0, 300.0);
        assert!(near(bw, w) && near(bh, h));
    }
}

#[test]
fn min_width_floors_and_max_height_caps() {
    let floored = StyleData {
        min_size: StyleSize::new(Length::Px(100.0), Length::Auto),
        ..px(50.0, 50.0)
    };
    assert!(near(root_box(floored, 400.0, 300.0).0, 100.0));
    let capped = StyleData {
        max_size: StyleSize::new(Length::Auto, Length::Px(120.0)),
        ..px(50.0, 500.0)
    };
    assert!(near(root_box(capped, 400.0, 300.0).1, 120.0));
}

#[test]
fn percent_width_resolves_against_containing_block() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, px(400.0, 200.0));
    let child = kid(
        &mut f,
        parent,
        sized(Length::Percent(0.5), Length::Px(30.0)),
    );
    assert!(near(
        run(&f, parent, 1000.0, 800.0).boxed(child).unwrap().w,
        200.0
    ));
}
