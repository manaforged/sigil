//! Behavioral tests for the G6 `Styled` trait surface on widget
//! builders (batch 1: button, checkbox, dropdown). See DR-049 + the
//! W4.G6-widgets-batch1 dispatch brief.
//!
//! Tests assert that the trait surface is wired to each widget's style
//! accumulator and that the accumulator's atoms / state-slot overlays
//! survive into the builder's [`Styled::style`] view. The pixel-level
//! merge into the kernel node is covered by `visual_goldens` (the
//! existing 30 PNG cases) — these tests cover the trait-level wiring.
//!
//! Note on method-name disambiguation: `ButtonBuilder` / `CheckboxBuilder`
//! / `DropdownBuilder` each carry an inherent `.style(Style) -> Self`
//! (consuming-self merge of a struct-built style) AND impl
//! `Styled::style(&mut self) -> &mut Style`. Rust's method resolution
//! prefers the inherent when the receiver shape matches. Tests use the
//! fully-qualified call `StyleAccess::style(&mut b)` to reach the trait
//! method's accumulator view. End-user code chaining `.bg() / .px() /
//! .hover(...)` is unaffected because those names live ONLY on the
//! trait.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

// Test-only deps that the dev-dep set pulls in for OTHER widget tests.
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use wgpu as _;

use sigil::prelude::{
    Fill, Geometry, NodeBuilder, Spacing, StateVariants, StyleAccess, StyleAtom, Visual, button,
    checkbox, container, dropdown, panel, progress, scroll, signal, slider, text_field,
};
use sigil::reactive::Runtime;

// ----- button -----

#[test]
fn button_styled_bg_emits_atom() {
    Runtime::scope("widget-styled-button-bg", || {
        let mut b = button("Run").bg("surface").rounded(8);
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "button bg('surface') must push BackgroundThemeRef onto the accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::BorderRadius(sigil_style::Px(8))),
            "button rounded(8) must push BorderRadius, got {atoms:?}",
        );
    });
}

#[test]
fn button_styled_hover_populates_state_slot() {
    Runtime::scope("widget-styled-button-hover", || {
        let mut b = button("Run").hover(|s| s.bg("surface-hover").opacity(8_000));
        let style = StyleAccess::style(&mut b);
        let hover = style.hover_atoms().to_vec();
        assert_eq!(
            hover.len(),
            2,
            "hover closure must record both bg + opacity atoms, got {hover:?}",
        );
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("surface-hover"));
        assert_eq!(hover[1], StyleAtom::Opacity(8_000));
    });
}

#[test]
fn button_styled_accepts_full_compile_surface() {
    Runtime::scope("widget-styled-button-compile", || {
        // Compile-time smoke test against the brief's signature:
        //   button("X").bg("surface").px(16).py(8).hover(|s| s.bg("surface-hover"))
        let mut b = button("X")
            .bg("surface")
            .px(16)
            .py(8)
            .hover(|s| s.bg("surface-hover"));
        let style = StyleAccess::style(&mut b);
        assert!(
            !style.is_empty(),
            "chained styled methods must populate the accumulator",
        );
        assert_eq!(style.hover_atoms().len(), 1);
    });
}

// ----- checkbox -----

#[test]
fn checkbox_styled_bg_emits_atom() {
    Runtime::scope("widget-styled-checkbox-bg", || {
        let checked = signal(false);
        let mut b = checkbox(checked).bg("surface").rounded(6);
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "checkbox bg('surface') must reach the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::BorderRadius(sigil_style::Px(6))),
            "rounded(6) must land, got {atoms:?}",
        );
    });
}

#[test]
fn checkbox_styled_hover_populates_state_slot() {
    Runtime::scope("widget-styled-checkbox-hover", || {
        let checked = signal(false);
        let mut b = checkbox(checked).hover(|s| s.bg("accent-hover"));
        let hover = StyleAccess::style(&mut b).hover_atoms().to_vec();
        assert_eq!(hover.len(), 1, "hover slot must capture exactly one atom");
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("accent-hover"));
    });
}

// ----- dropdown -----

#[test]
fn dropdown_styled_emits_on_trigger_accumulator() {
    Runtime::scope("widget-styled-dropdown-bg", || {
        let selected = signal(0_usize);
        let mut b = dropdown(selected, vec!["a".into(), "b".into()])
            .bg("surface")
            .padding(12);
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "dropdown bg('surface') must hit the trait accumulator (which is then \
             merged into the trigger at build time), got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingAll(sigil_style::Px(12))),
            "padding(12) must land in the accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn dropdown_styled_pressed_populates_state_slot() {
    Runtime::scope("widget-styled-dropdown-pressed", || {
        let selected = signal(0_usize);
        let mut b =
            dropdown(selected, vec!["a".into()]).pressed(|s| s.bg("surface-pressed-strong"));
        let pressed = StyleAccess::style(&mut b).pressed_atoms().to_vec();
        assert_eq!(pressed.len(), 1);
        assert_eq!(
            pressed[0],
            StyleAtom::BackgroundThemeRef("surface-pressed-strong"),
        );
    });
}

// ----- panel -----

#[test]
fn panel_accepts_styled_surface() {
    Runtime::scope("widget-styled-panel-surface", || {
        // Confirm the Styled accumulator wiring on PanelBuilder: chained
        // base + state-slot atoms must land in the trait view, and the
        // accumulator's atoms must compose with (not displace) the
        // panel's `surface-hover` elevation default + `rounded-md`
        // class — that composition is asserted at pixel level by the
        // `panel_default` golden.
        // `panel().padding(...)` is the existing inherent (f32 arg);
        // the Styled-trait padding is `.px(...)` here to avoid the
        // method-resolution collision (DR-049 §11 — inherent wins on
        // a concrete receiver). End-user code mixing inherent +
        // trait stays unambiguous because the surface words differ.
        let mut b = panel()
            .title("Settings")
            .bg("accent")
            .px(20)
            .hover(|s| s.bg("accent-hover"));
        let style = StyleAccess::style(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("accent")),
            "panel bg('accent') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil_style::Px(20))),
            "panel px(20) must land in the trait accumulator, got {atoms:?}",
        );
        let hover = style.hover_atoms().to_vec();
        assert_eq!(
            hover.len(),
            1,
            "panel hover closure must record exactly one atom, got {hover:?}",
        );
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("accent-hover"));
    });
}

// ----- text_field -----

#[test]
fn text_field_styled_composes_with_state_variant() {
    Runtime::scope("widget-styled-text-field-state-compose", || {
        // text_field already ships state-driven backgrounds (FIELD_BASE
        // / FIELD_REST / FIELD_FOCUS / FIELD_HOVER / FIELD_DOWN /
        // FIELD_DISABLED). The Styled accumulator must populate its
        // own hover/focus/pressed/disabled slots WITHOUT touching the
        // built-in FIELD_* arrays — composition happens at engine
        // install time (G6.2), not at builder construction. This test
        // asserts the trait-level wiring: each state-slot closure
        // lands an atom into the matching accumulator slot, and the
        // base accumulator slot also captures bg / padding atoms
        // independently of the field's own state arms.
        let value = signal(String::new());
        // `TextFieldBuilder::disabled` is the existing inherent
        // (consuming, takes `Signal<bool>`); we exercise the
        // Styled-trait disabled slot via fully-qualified call after
        // the chained surface to avoid the method-resolution
        // collision (DR-049 §11 — inherent wins on a concrete
        // receiver). The bg/px/hover/focus/pressed names do not
        // collide and chain naturally.
        let mut b = text_field(value)
            .placeholder("Search…")
            .bg("surface")
            .px(10)
            .hover(|s| s.bg("surface-hover"))
            .focus(|s| s.bg("on-surface"))
            .pressed(|s| s.bg("surface-pressed"));
        b = StateVariants::disabled(b, |s| s.opacity(5_000));
        let style = StyleAccess::style(&mut b);
        let base = style.as_atoms().to_vec();
        assert!(
            base.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "text_field bg('surface') must land in trait accumulator base, got {base:?}",
        );
        assert!(
            base.contains(&StyleAtom::PaddingX(sigil_style::Px(10))),
            "text_field px(10) must land in trait accumulator base, got {base:?}",
        );
        // Each state slot must independently capture its closure's
        // atoms — confirms the accumulator's StateStyle storage is
        // wired through the trait surface and ready to compose with
        // the engine's existing FIELD_* arms at install time.
        let hover = style.hover_atoms().to_vec();
        assert_eq!(hover.len(), 1, "hover slot must capture one atom");
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("surface-hover"));
        let focus = style.focus_atoms().to_vec();
        assert_eq!(focus.len(), 1, "focus slot must capture one atom");
        assert_eq!(focus[0], StyleAtom::BackgroundThemeRef("on-surface"));
        let pressed = style.pressed_atoms().to_vec();
        assert_eq!(pressed.len(), 1, "pressed slot must capture one atom");
        assert_eq!(pressed[0], StyleAtom::BackgroundThemeRef("surface-pressed"));
        let disabled = style.disabled_atoms().to_vec();
        assert_eq!(disabled.len(), 1, "disabled slot must capture one atom");
        assert_eq!(disabled[0], StyleAtom::Opacity(5_000));
    });
}

// ----- slider -----

#[test]
fn slider_accepts_styled_surface() {
    Runtime::scope("widget-styled-slider", || {
        let value = signal(0.5_f32);
        let mut b = slider(value, 0.0..=1.0).bg("surface").padding(8);
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "slider bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingAll(sigil_style::Px(8))),
            "padding(8) must land in the accumulator, got {atoms:?}",
        );
    });
}

// ----- progress -----

#[test]
fn progress_accepts_styled_surface() {
    Runtime::scope("widget-styled-progress", || {
        let value = signal(0.75_f32);
        let mut b = progress(value, 1.0)
            .bg("surface")
            .hover(|s| s.bg("surface-hover"));
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        let hover = StyleAccess::style(&mut b).hover_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "progress bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
        assert_eq!(hover.len(), 1);
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("surface-hover"));
    });
}

// ----- scroll -----

#[test]
fn scroll_accepts_styled_surface() {
    Runtime::scope("widget-styled-scroll", || {
        let inner: NodeBuilder = container();
        let mut b = scroll(inner, 400.0_f32)
            .bg("surface")
            .pressed(|s| s.bg("surface-pressed"));
        let atoms = StyleAccess::style(&mut b).as_atoms().to_vec();
        let pressed = StyleAccess::style(&mut b).pressed_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "scroll bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
        assert_eq!(pressed.len(), 1);
        assert_eq!(pressed[0], StyleAtom::BackgroundThemeRef("surface-pressed"));
    });
}
