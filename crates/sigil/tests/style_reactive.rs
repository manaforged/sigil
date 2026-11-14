#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::kernel::{Cx, container};
use sigil::prelude::Widget;
use sigil::reactive::{Owner, signal};
use sigil::style::{ColorValue, Paint, PaintAtom, StyleAtom, StyleBinding, Token};

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
            &background_atoms(Token::Accent),
            "paint_when must install the accent background when signal flips true, got {live:?}",
        );

        highlighted.set(false);
        assert!(
            reactive_slice(&cx, id).is_empty(),
            "paint_when must clear atoms when signal flips false",
        );
    });
}

#[test]
fn background_follows_a_token_signal() {
    Owner::scope("sigil_style_background_signal", || {
        let fill = signal(Token::Surface);
        let mut cx = Cx::new_root();
        let id = cx.add(container().background(fill));
        assert_eq!(reactive_slice(&cx, id), &background_atoms(Token::Surface));
        fill.set(Token::Accent);
        assert_eq!(reactive_slice(&cx, id), &background_atoms(Token::Accent));
    });
}

#[test]
fn text_color_follows_a_closure() {
    Owner::scope("sigil_style_text_color_closure", || {
        let on = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().text_color(move || {
            if on.get() {
                Token::Accent
            } else {
                Token::OnSurface
            }
        }));
        let text_atom = |token| [StyleAtom::TextColor(ColorValue::Token(token))];
        assert_eq!(reactive_slice(&cx, id), &text_atom(Token::OnSurface));
        on.set(true);
        assert_eq!(reactive_slice(&cx, id), &text_atom(Token::Accent));
    });
}

#[test]
fn a_static_background_installs_no_reactive_channel() {
    Owner::scope("sigil_style_background_static", || {
        let mut cx = Cx::new_root();
        let id = cx.add(container().background(Token::Accent));
        let channels = cx
            .get(id)
            .and_then(|node| node.extension::<StyleBinding>())
            .map_or(0, |binding| binding.reactive_channels().count());
        assert_eq!(channels, 0);
    });
}

const fn background_atoms(token: Token) -> [StyleAtom; 2] {
    [
        StyleAtom::BackgroundColor(ColorValue::Token(token)),
        StyleAtom::Paint(PaintAtom::NoGradient),
    ]
}

fn reactive_slice(cx: &Cx, id: sigil::kernel::NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.extension::<StyleBinding>())
        .unwrap();
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}
