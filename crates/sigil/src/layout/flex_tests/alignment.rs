use super::*;

#[test]
fn justify_content_places_items_on_main_axis() {
    let cases: [(JustifyContent, f32, &[f32], &[f32]); 6] = [
        (JustifyContent::Start, 300.0, &[50.0], &[0.0]),
        (JustifyContent::End, 300.0, &[60.0], &[240.0]),
        (JustifyContent::Center, 300.0, &[100.0], &[100.0]),
        (
            JustifyContent::SpaceBetween,
            300.0,
            &[50.0, 50.0],
            &[0.0, 250.0],
        ),
        (
            JustifyContent::SpaceAround,
            300.0,
            &[50.0, 50.0],
            &[50.0, 200.0],
        ),
        (
            JustifyContent::SpaceEvenly,
            310.0,
            &[50.0, 50.0],
            &[70.0, 190.0],
        ),
    ];
    for (justify, width, widths, xs) in cases {
        let parent = StyleData {
            justify_content: Some(justify),
            ..flex_box(width, 50.0)
        };
        let items: Vec<_> = widths.iter().map(|&w| fixed(w, 20.0)).collect();
        let boxes = kids(parent, &items);
        for (b, &x) in boxes.iter().zip(xs) {
            assert!(near(b.x, x), "{justify:?}: x={} want {x}", b.x);
        }
    }
}

#[test]
fn align_items_places_items_on_cross_axis() {
    for (align, y) in [
        (AlignItems::Start, 0.0),
        (AlignItems::End, 70.0),
        (AlignItems::Center, 35.0),
    ] {
        let parent = StyleData {
            align_items: Some(align),
            ..flex_box(200.0, 100.0)
        };
        let b = kids(parent, &[fixed(40.0, 30.0)])[0];
        assert!(near(b.y, y), "{align:?}: y={}", b.y);
    }
}

#[test]
fn align_items_stretch_fills_cross_when_cross_size_auto() {
    let parent = StyleData {
        align_items: Some(AlignItems::Stretch),
        ..flex_box(200.0, 100.0)
    };
    let b = kids(parent, &[sized(Dimension::Px(40.0), Dimension::Auto)])[0];
    assert!(near(b.h, 100.0), "h={}", b.h);
}

#[test]
fn align_self_overrides_container_align_items() {
    let parent = StyleData {
        align_items: Some(AlignItems::Start),
        ..flex_box(200.0, 100.0)
    };
    let item = StyleData {
        align_self: Some(AlignSelf::End),
        ..fixed(40.0, 30.0)
    };
    assert!(near(kids(parent, &[item])[0].y, 70.0));
}
