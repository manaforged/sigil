//! Pump [`sigil_app::Host`] with no window and read layout + text/tone.
//!
//! Run: `cargo test -p sigil-app --no-default-features --test host_layout`

#![allow(clippy::unwrap_used, missing_docs, unused_crate_dependencies)]

use sigil_app::Host;
use sigil_kernel::{Surface, container, text};
use sigil_layout::LayoutError;
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle};
use sigil_style::{NodeStyleExt, Tone, ToneContext};

#[test]
fn host_root_is_none_until_build() {
    let host = Host::new();
    assert!(host.root().is_none());
}

#[test]
fn host_mount_accepts_text_node() {
    let mut host = Host::new();
    host.mount(text("hello"));
    assert!(host.root().is_some());
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let painted = host.painted();
    assert!(
        painted
            .iter()
            .any(|node| node.text.as_deref() == Some("hello")),
        "expected hello in {painted:?}"
    );
}

#[test]
fn host_layout_reads_text_and_tone_without_a_window() {
    let mut host = Host::new();
    host.build(|cx| {
        cx.add(
            container()
                .with(ToneContext::new(Tone::Primary))
                .child(text("hello")),
        )
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let painted = host.painted();
    assert!(
        painted
            .iter()
            .any(|node| node.text.as_deref() == Some("hello")),
        "expected hello in {painted:?}"
    );
    assert!(
        painted.iter().any(|node| node.tone == Some(Tone::Primary)),
        "expected primary tone in {painted:?}"
    );
    assert!(
        painted
            .iter()
            .any(|node| node.rect.width > 0.0 && node.rect.height > 0.0),
        "expected a laid-out rect in {painted:?}"
    );
}

#[test]
fn host_layout_returns_missing_root_error() {
    let mut host = Host::new();
    let mut root = None;
    host.build(|cx| {
        let id = cx.add(text("gone"));
        root = Some(id);
        id
    });
    let root = root.unwrap();
    assert!(host.cx().remove(root));

    assert_eq!(
        host.layout((80.0, 24.0)),
        Err(LayoutError::NodeMissing(root))
    );
}

#[test]
fn host_surfaces_reports_hole_rect() {
    let mut host = Host::new();
    host.build(|cx| {
        cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
                .with(Surface { key: 3 })
                .child(text("hole")),
        )
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let holes = host.surfaces();
    assert_eq!(holes.len(), 1);
    assert_eq!(holes[0].0, 3);
    assert!(holes[0].1.width > 0.0);
}

#[test]
fn host_paint_skips_surface_children() {
    let mut host = Host::new();
    host.build(|cx| {
        cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(80.0, 24.0))
                .child(text("visible"))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(40.0, 24.0))
                        .with(Surface { key: 1 })
                        .class("bg-accent")
                        .child(text("secret")),
                ),
        )
    });
    assert!(host.layout((80.0, 24.0)).unwrap().is_some());
    let painted = host.painted();
    assert!(
        painted
            .iter()
            .any(|node| node.text.as_deref() == Some("visible")),
        "expected visible in {painted:?}"
    );
    assert!(
        painted
            .iter()
            .all(|node| node.text.as_deref() != Some("secret")),
        "hole children must not paint; got {painted:?}"
    );
    #[cfg(feature = "desktop")]
    {
        let scene = host.scene((80.0, 24.0)).unwrap();
        let rects = scene
            .commands
            .iter()
            .filter(|command| matches!(command, sigil_renderer::SceneCommand::Rect(_)))
            .count();
        assert_eq!(rects, 0, "hole fill must not emit a rect");
        assert!(
            scene.find_text("secret").is_none(),
            "hole children must not emit glyphs"
        );
    }
}
