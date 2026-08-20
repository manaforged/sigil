//! Pump [`sigil_app::Host`] with no window and read layout + text/tone.
//!
//! Run: `cargo test -p sigil-app --no-default-features --test host_layout`

#![allow(clippy::unwrap_used, missing_docs, unused_crate_dependencies)]

use sigil_app::Host;
use sigil_kernel::{container, text};
use sigil_style::{Tone, ToneContext};

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
    assert!(host.layout((80.0, 24.0)).is_some());
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
