#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic, clippy::unwrap_used)]

use sigil::kernel::{Cx, container};
use sigil::reactive::{Runtime, signal};
use sigil::style::{ColorKey, NodeStyleExt, Paint, StyleAtom, StyleBinding, Token};

#[test]
fn paint_when_toggles_on_signal_change() {
    Runtime::scope("sigil_style_paint_when_toggle", || {
        let highlighted = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().paint_when(highlighted, |s| s.bg(Token::Accent)));

        let initial = reactive_slice(&cx, id);
        assert!(
            initial.is_empty(),
            "paint_when starts with empty slice while signal is false, got {initial:?}",
        );

        highlighted.set(true);
        let live = reactive_slice(&cx, id);
        assert_eq!(
            live,
            &[StyleAtom::BackgroundThemeRef(ColorKey::Token(
                Token::Accent
            ))],
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
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}
