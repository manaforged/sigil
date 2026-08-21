use super::*;

#[test]
fn bottom_center_centers_and_clears_margin() {
    // 2560x1440 monitor at origin, 200x48 window, margin 64 @ 1x.
    let (x, y) = anchor_origin(
        Anchor::BottomCenter { margin: 64.0 },
        (0, 0),
        (2560, 1440),
        (200, 48),
        1.0,
    );
    assert_eq!(x, (2560 - 200) / 2);
    assert_eq!(y, 1440 - 48 - 64);
}

#[test]
fn bottom_center_scales_margin_by_monitor_factor() {
    // Retina 2x: 64 logical px of margin = 128 physical px.
    let (_, y) = anchor_origin(
        Anchor::BottomCenter { margin: 64.0 },
        (0, 0),
        (5120, 2880),
        (400, 96),
        2.0,
    );
    assert_eq!(y, 2880 - 96 - 128);
}

#[test]
fn bottom_center_respects_monitor_origin() {
    // Secondary monitor left of the primary (negative origin).
    let (x, y) = anchor_origin(
        Anchor::BottomCenter { margin: 10.0 },
        (-1920, 200),
        (1920, 1080),
        (100, 40),
        1.0,
    );
    assert_eq!(x, -1920 + (1920 - 100) / 2);
    assert_eq!(y, 200 + 1080 - 40 - 10);
}
