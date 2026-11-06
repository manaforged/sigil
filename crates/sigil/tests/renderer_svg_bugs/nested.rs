use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree};

use super::{frames, picture, svg};

const SIDE: u32 = 200;

const CLIP_ON_CHILD: &str = r#"<g><clipPath id="clip2"><circle cx="100" cy="100" r="60"/></clipPath><clipPath id="clip1"><path d="M 100 15 l 50 160 l -130 -100 l 160 0 l -130 100 z" clip-rule="evenodd" clip-path="url(#clip2)"/></clipPath><rect width="200" height="200" fill="green" clip-path="url(#clip1)"/></g>"#;
const MASK_ON_CHILD: &str = r#"<g><linearGradient id="lg1"><stop offset="0" stop-color="white" stop-opacity="0"/><stop offset="1" stop-color="black"/></linearGradient><mask id="mask1"><rect x="20" y="20" width="160" height="160" fill="url(#lg1)"/></mask><mask id="mask2"><rect x="20" y="20" width="160" height="160" fill="green" mask="url(#mask1)"/></mask><rect x="20" y="20" width="160" height="160" fill="green" mask="url(#mask2)"/></g>"#;
const TOLERANCE: u8 = 32;
const MAX_OFF: f32 = 0.02;

fn reference(source: &str) -> Vec<u8> {
    let tree = Tree::from_str(source, &usvg::Options::default()).expect("svg");
    let mut pixmap = Pixmap::new(SIDE, SIDE).expect("pixmap");
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    pixmap.data().to_vec()
}

fn off(got: &[u8], want: &[u8]) -> f32 {
    let wrong = got
        .chunks_exact(4)
        .zip(want.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(*b).take(3).any(|(x, y)| x.abs_diff(*y) > TOLERANCE))
        .count();
    wrong as f32 / (SIDE * SIDE) as f32
}

#[track_caller]
fn assert_like_resvg(body: &str) {
    let source = svg(body, SIDE);
    let want = reference(&source);
    let scene = picture(&source, SIDE);
    let failures: Vec<String> = frames(&scene, SIDE)
        .into_iter()
        .map(|(name, got)| (name, off(&got, &want)))
        .filter(|(_, fraction)| *fraction > MAX_OFF)
        .map(|(name, fraction)| format!("{name}: {fraction:.3} of pixels off"))
        .collect();
    assert!(failures.is_empty(), "{body}: {failures:?}");
}

#[test]
fn a_clip_path_on_a_clip_path_child_clips_like_resvg() {
    assert_like_resvg(CLIP_ON_CHILD);
}

#[test]
fn a_mask_on_a_mask_child_masks_like_resvg() {
    assert_like_resvg(MASK_ON_CHILD);
}
