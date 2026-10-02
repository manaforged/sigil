#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::icons::{BakedIcon, IconBaker};

const SIDE: u32 = 32;

fn bake(body: &str) -> BakedIcon {
    let svg =
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">{body}</svg>"#);
    IconBaker::new().bake(&svg, SIDE, SIDE).unwrap()
}

fn alpha(icon: &BakedIcon, x: u32, y: u32) -> u8 {
    usize::try_from(y * icon.width + x)
        .ok()
        .and_then(|index| icon.alpha.get(index))
        .copied()
        .unwrap()
}

#[test]
fn open_fill_subpath_closes_back_to_its_start() {
    let icon = bake(r#"<path d="M4 4H20V20H4" fill="black"/>"#);
    assert_eq!(alpha(&icon, 16, 16), 255);
    assert_eq!(alpha(&icon, 2, 16), 0);
}

#[test]
fn stroked_polyline_leaves_rows_outside_its_outline_empty() {
    let icon = bake(
        r#"<path d="M22 12h-2.48l-2.35 8.36L9.24 2.18l-2.35 8.36H2" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
    );
    let edge: Vec<u8> = (0..SIDE).map(|y| alpha(&icon, 0, y)).collect();
    assert!(edge.iter().all(|&a| a == 0), "left edge inked: {edge:?}");
    let corner: Vec<u8> = (0..SIDE).map(|x| alpha(&icon, x, 0)).collect();
    assert!(corner.iter().all(|&a| a == 0), "top row inked: {corner:?}");
}
