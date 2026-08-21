#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Regression coverage for Phase C reactive attribute methods of the
//! dogfood-v2 gap-closure plan:
//!
//! - C2: [`ButtonBuilder::disabled(Signal<bool>)`] gates the
//!   constructor focusable bit on the signal's initial value and
//!   keeps the reactive style slice in sync with subsequent flips.
//! - C3: [`TextFieldBuilder::invalid_when(Signal<bool>)`] paints a
//!   red border overlay while the signal peeks `true` and clears it
//!   the moment the signal flips back to `false`.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil_kernel::Cx;
use sigil_reactive::{Runtime, signal};
use sigil_style::{ColorAtom, Rgba8, StyleAtom, StyleBinding};

fn reactive_slice(cx: &Cx, id: sigil_kernel::NodeId) -> &'static [StyleAtom] {
    let binding = cx
        .get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .unwrap();
    // T0 composition: concatenate every reactive channel in install order.
    let atoms: Vec<StyleAtom> = binding.reactive_channels().flatten().copied().collect();
    Box::leak(atoms.into_boxed_slice())
}

#[test]
fn button_disabled_initial_true_blocks_focus() {
    Runtime::scope("button-disabled-initial", || {
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let id = cx.add(button("Save").disabled(disabled));
        let node = cx.get(id).unwrap();
        assert!(
            !node.a11y.focusable,
            "button(disabled=true) at construction must not be focusable",
        );
    });
}

const FIELD_INVALID_RED: Rgba8 = Rgba8 {
    r: 211,
    g: 47,
    b: 47,
    a: 255,
};

#[test]
fn text_field_invalid_when_paints_red_border_on_flip() {
    Runtime::scope("text-field-invalid-flip", || {
        let value = signal(String::new());
        let invalid = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(text_field(value).invalid_when(invalid));

        let before = reactive_slice(&cx, id);
        assert!(
            !before.contains(&StyleAtom::BorderColor(ColorAtom::Rgba8(FIELD_INVALID_RED))),
            "field starts without the red-invalid overlay; got {before:?}",
        );

        // Flip the invalid signal — the reactive effect must rewrite
        // the field's reactive slot to the red FIELD_INVALID overlay.
        invalid.set(true);
        let painted = reactive_slice(&cx, id);
        assert_eq!(
            painted,
            &[StyleAtom::BorderColor(ColorAtom::Rgba8(FIELD_INVALID_RED))],
            "invalid_when(true) must install the red border overlay, got {painted:?}",
        );

        // Flip back — the field must clear the overlay.
        invalid.set(false);
        let cleared = reactive_slice(&cx, id);
        assert!(
            !cleared.contains(&StyleAtom::BorderColor(ColorAtom::Rgba8(FIELD_INVALID_RED))),
            "invalid_when(false) must remove the red border overlay; got {cleared:?}",
        );
    });
}

#[test]
fn button_disabled_signal_flips_reactive_state_slice() {
    Runtime::scope("button-disabled-reactive-flip", || {
        let disabled = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(button("Save").disabled(disabled));

        // Snapshot the rest-state reactive slice while the signal is false.
        let before = reactive_slice(&cx, id);

        // Flip the signal — the `style_reactive` effect must re-run and
        // rewrite the reactive slice to the disabled-slot overlay.
        disabled.set(true);
        let after = reactive_slice(&cx, id);

        assert_ne!(
            before, after,
            "flipping the disabled signal must change the reactive style slice; before={before:?} after={after:?}",
        );
        assert!(
            after
                .iter()
                .any(|atom| matches!(atom, StyleAtom::Opacity(5_000))),
            "disabled overlay must include the Opacity(5_000) atom from the DISABLED state-style; got {after:?}",
        );
    });
}
