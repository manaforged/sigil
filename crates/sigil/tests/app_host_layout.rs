#![allow(clippy::unwrap_used, unused_crate_dependencies)]

use sigil::Host;
use sigil::kernel::{Surface, container, text};
use sigil::layout::{LayoutBuilderExt, LayoutError, LayoutStyle};
use sigil::style::{Paint, Token, Tone, ToneContext};

#[test]
fn host_root_is_none_until_build() {
    let host = Host::new();
    assert!(host.root().is_none());
}

#[test]
fn host_mount_accepts_text_node() {
    let mut host = Host::new();
    host.mount(move || text("hello"));
    assert!(host.root().is_some());
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let scene = host.scene((80.0, 24.0)).unwrap();
    assert!(
        scene.find_text("hello").is_some(),
        "expected hello in {scene:?}"
    );
}

#[test]
fn host_layout_reads_text_and_tone_without_a_window() {
    let mut host = Host::new();
    host.mount(move || {
        container()
            .with(ToneContext::new(Tone::Primary))
            .child(text("hello"))
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let scene = host.scene((80.0, 24.0)).unwrap();
    let hello = scene.find_text("hello").expect("hello run");
    assert!(
        hello.run.rect.width > 0.0 && hello.run.rect.height > 0.0,
        "expected a laid-out rect in {scene:?}"
    );
}

#[test]
fn host_layout_returns_missing_root_error() {
    let mut host = Host::new();
    host.mount(|| text("gone"));
    let root = host.root().unwrap();
    assert!(host.cx().remove(root));

    assert_eq!(
        host.layout((80.0, 24.0)),
        Err(LayoutError::NodeMissing(root))
    );
}

#[test]
fn host_surfaces_reports_hole_rect() {
    let mut host = Host::new();
    host.mount(move || {
        container()
            .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
            .with(Surface { key: 3 })
            .child(text("hole"))
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let holes = host.surfaces();
    let hole = holes.first().copied().expect("one surface hole");
    assert_eq!(hole.0, 3);
    assert!(hole.1.width > 0.0);
}

#[test]
fn host_paint_skips_surface_children() {
    let mut host = Host::new();
    host.mount(move || {
        container()
            .layout(LayoutStyle::flex_row().fixed_size(80.0, 24.0))
            .child(text("visible"))
            .child(
                container()
                    .layout(LayoutStyle::block().fixed_size(40.0, 24.0))
                    .with(Surface { key: 1 })
                    .bg(Token::Accent)
                    .child(text("secret")),
            )
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let painted = host.scene((80.0, 24.0)).unwrap();
    assert!(
        painted.find_text("visible").is_some(),
        "expected visible in {painted:?}"
    );
    assert!(
        painted.find_text("secret").is_none(),
        "hole children must not paint; got {painted:?}"
    );
    #[cfg(feature = "desktop")]
    {
        let scene = host.scene((80.0, 24.0)).unwrap();
        let rects = scene
            .commands
            .iter()
            .filter(|command| matches!(command, sigil::scene::SceneCommand::Rect(_)))
            .count();
        assert_eq!(rects, 0, "hole fill must not emit a rect");
        assert!(
            scene.find_text("secret").is_none(),
            "hole children must not emit glyphs"
        );
    }
}
