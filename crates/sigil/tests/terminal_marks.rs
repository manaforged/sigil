#![allow(clippy::expect_used)]

use sigil::prelude::*;
use sigil::reactive::Owner;

fn inked(frame: &str) -> usize {
    frame
        .chars()
        .filter(|c| matches!(u32::from(*c), 0x1FB00..=0x1FB3B | 0x2580..=0x259F))
        .count()
}

#[test]
fn a_thin_stroke_survives_the_sextant_fallback() {
    let frame = sigil_kittest::Harness::new(|| {
        path(400, 200)
            .move_to(0.0, 100.0)
            .line_to(400.0, 100.0)
            .stroke(
                Color {
                    r: 200,
                    g: 60,
                    b: 60,
                    a: 255,
                },
                2.0,
            )
            .layout(LayoutStyle::block().width(cols(60.0)).height(rows(6.0)))
    })
    .size(80, 8)
    .text();
    assert!(inked(&frame) >= 30, "stroke lost in:\n{frame}");
}

fn first_ink(tone: Option<Tone>) -> Option<sigil::terminal::Ink> {
    let mut host = sigil::Host::new(sigil::kernel::HostMode::Cell);
    Owner::scope("progress-tone", || {
        host.mount(move || {
            let bar = progress(signal(1.0), 1.0).height(1.0);
            let bar = match tone {
                Some(tone) => bar.tone(tone),
                None => bar,
            };
            container()
                .layout(LayoutStyle::block().width(cols(20.0)))
                .child(bar)
        });
        let grid = host.grid([], 30, 3).expect("layout");
        grid.get(0, 0).map(|cell| cell.fg)
    })
}

#[test]
fn a_progress_tone_changes_the_fill_ink() {
    let plain = first_ink(None);
    let warn = first_ink(Some(Tone::Warning));
    assert!(plain.is_some() && warn.is_some());
    assert_ne!(plain, warn);
}

#[test]
fn colorfgbg_names_a_dark_or_light_backdrop() {
    use sigil::terminal::{Backdrop, Capabilities};
    let caps = |pair: &'static str| {
        Capabilities::detect(move |key| (key == "COLORFGBG").then(|| pair.to_string())).backdrop()
    };
    assert_eq!(caps("15;0"), Some(Backdrop::Dark));
    assert_eq!(caps("0;15"), Some(Backdrop::Light));
    assert_eq!(Capabilities::detect(|_| None).backdrop(), None);
}
