#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::kernel::{Cx, container};
use sigil::prelude::Widget;
use sigil::reactive::{Owner, signal};
use sigil::style::{ColorAtom, Paint, PaintAtom, StyleAtom, StyleBinding, Token};

#[test]
fn paint_when_toggles_on_signal_change() {
    Owner::scope("sigil_style_paint_when_toggle", || {
        let highlighted = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().paint_when(highlighted, |s| s.background(Token::Accent)));

        let initial = reactive_slice(&cx, id);
        assert!(
            initial.is_empty(),
            "paint_when starts with empty slice while signal is false, got {initial:?}",
        );

        highlighted.set(true);
        let live = reactive_slice(&cx, id);
        assert_eq!(
            live,
            &[
                StyleAtom::BackgroundColor(ColorAtom::Token(Token::Accent)),
                StyleAtom::Paint(PaintAtom::NoGradient),
            ],
            "paint_when must install the accent background when signal flips true, got {live:?}",
        );

        highlighted.set(false);
        assert!(
            reactive_slice(&cx, id).is_empty(),
            "paint_when must clear atoms when signal flips false",
        );
    });
}

fn reactive_slice(cx: &Cx, id: sigil::kernel::NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.extension::<StyleBinding>())
        .unwrap();
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}
