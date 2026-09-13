use super::*;

#[test]
fn hidden_axis_tracks_intermediate_weight() {
    let model = VariationModel {
        axes: vec![
            Axis {
                tag: *b"YAXS",
                default: 300.0,
                hidden: true,
            },
            Axis {
                tag: *b"wght",
                default: 400.0,
                hidden: false,
            },
        ]
        .into_boxed_slice(),
        instances: vec![
            Instance {
                coords: vec![320.0, 400.0].into_boxed_slice(),
            },
            Instance {
                coords: vec![350.0, 700.0].into_boxed_slice(),
            },
        ]
        .into_boxed_slice(),
        weight_axis: Some(1),
    };

    assert_eq!(
        model.settings(550, 14.0).as_slice(),
        &[(*b"YAXS", 335.0), (*b"wght", 550.0)]
    );
}
