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
//! Note on method-name disambiguation: per W4.99.B2 the inherent
//! consuming-self method `.style(Style) -> Self` was renamed to
//! `.style(Style) -> Self` on `ButtonBuilder` / `CheckboxBuilder`
//! / `ToggleBuilder` / `SliderBuilder` so that the `style()` name is
//! reserved for the [`StyleAccess`] trait method
//! (`fn style_mut(&mut self) -> &mut Style`). Tests can therefore call
//! `.style()` (zero-arg, &mut self) freely without method-resolution
//! ambiguity.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

// Test-only deps that the dev-dep set pulls in for OTHER widget tests.
use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::{
    FctSpawn, Fill, Geometry, MenuItem, NodeBuilder, Spacing, StateVariants, StyleAccess,
    StyleAtom, Visual, button, checkbox, container, dropdown, floating_text, menu, modal, panel,
    progress, scroll, signal, slider, tabs, text_field, toggle,
};
use sigil::reactive::Runtime;

// ----- button -----

#[test]
fn button_styled_bg_emits_atom() {
    Runtime::scope("widget-styled-button-bg", || {
        let mut b = button("Run").bg("surface").rounded(8);
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
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
        let mut b = button("Run").hover(|s| s.bg("surface-hover").opacity(0.8));
        let style = StyleAccess::style_mut(&mut b);
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
        let style = StyleAccess::style_mut(&mut b);
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
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
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
        let hover = StyleAccess::style_mut(&mut b).hover_atoms().to_vec();
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
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
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
        let pressed = StyleAccess::style_mut(&mut b).pressed_atoms().to_vec();
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
        // DS2.5 — `impl StateHost for PanelBuilder` was removed because
        // panel doesn't expose a stateful slot for user `.hover()` /
        // `.pressed()` atoms to merge into. The trait-level base
        // accumulator (bg / px) still works.
        let mut b = panel().title("Settings").bg("accent").px(20);
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("accent")),
            "panel bg('accent') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil_style::Px(20))),
            "panel px(20) must land in the trait accumulator, got {atoms:?}",
        );
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
        b = StateVariants::disabled_style(b, |s| s.opacity(0.5));
        let style = StyleAccess::style_mut(&mut b);
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
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
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
        let mut b = progress(value, 1.0).bg("surface");
        // 5/5 Wave 1: `impl StateHost` removed from this display-only
        // widget — `.hover()`/`.pressed()` on it is now a compile error by
        // design (panel precedent). Only base StyleAccess is exercised here.
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "progress bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

// ----- floating_text -----

#[test]
fn floating_text_accepts_styled_surface() {
    Runtime::scope("widget-styled-floating-text", || {
        let spawn = signal(FctSpawn::default());
        let mut b = floating_text(spawn)
            .duration(1.5)
            .capacity(16)
            .bg("surface");
        // 5/5 Wave 1: `impl StateHost` removed from this display-only
        // widget — `.hover()`/`.pressed()` on it is now a compile error by
        // design (panel precedent). Only base StyleAccess is exercised here.
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "floating_text bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

// ----- scroll -----

// ----- W4.99.B2 trap 3 regression -----

/// Trap 3: the consuming inherent `.style(Style)` on `ButtonBuilder` /
/// `CheckboxBuilder` / `ToggleBuilder` / `SliderBuilder` was renamed
/// to `.style(Style)` to reserve the `style()` name for the
/// `StyleAccess` trait method (`fn style_mut(&mut self) -> &mut Style`).
///
/// This test is primarily a compile-time gate: it must build cleanly
/// with `.style(my_style)` (consuming, with arg) AND a fully-
/// qualified trait call `StyleAccess::style_mut(&mut b)` (&mut, zero-arg)
/// on the same builder, with no shadow or resolution ambiguity.
#[test]
fn trap3_style_and_trait_style_coexist() {
    Runtime::scope("widget-styled-trap3-style", || {
        let my_style = sigil_style::Style::new()
            .padding(8)
            .background_theme("surface");
        let mut b = button("X").style(my_style);
        // Trait `style_mut()` returns &mut Style — must compile on the
        // same builder without shadowing the inherent.
        let style_view = StyleAccess::style_mut(&mut b);
        // Empty accumulator at this point — `style` writes via
        // the mutator queue, not the trait accumulator.
        assert!(style_view.is_empty());

        let checked = signal(false);
        let _cb = checkbox(checked).style(sigil_style::Style::new().padding(4));

        let on = signal(false);
        let _tg = toggle(on).style(sigil_style::Style::new().padding(4));

        let value = signal(0.5_f32);
        let _sl = slider(value, 0.0..=1.0).style(sigil_style::Style::new().padding(4));
    });
}

// ----- modal -----

#[test]
fn modal_accepts_styled_surface() {
    Runtime::scope("widget-styled-modal-surface", || {
        // Confirm the Styled accumulator wiring on ModalBuilder: chained
        // base + state-slot atoms must land in the trait view, and the
        // accumulator's atoms compose with (not displace) the dialog's
        // `surface-hover` default. The pixel-level merge is covered by
        // the `modal_dismissing` golden (opacity composition).
        let mut b = modal().title("Confirm action").bg("surface").px(20);
        // 5/5 Wave 1: `impl StateHost` removed from this display-only
        // widget — `.hover()`/`.pressed()` on it is now a compile error by
        // design (panel precedent). Only base StyleAccess is exercised here.
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "modal bg('surface') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil_style::Px(20))),
            "modal px(20) must land in the trait accumulator, got {atoms:?}",
        );
    });
}

// ----- tabs -----

#[test]
fn tabs_accepts_styled_surface() {
    Runtime::scope("widget-styled-tabs", || {
        let active = signal(0_usize);
        let mut b = tabs(active, vec!["A".into(), "B".into()])
            .label("Settings sections")
            .bg("surface")
            .hover(|s| s.bg("surface-hover"));
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        let hover = StyleAccess::style_mut(&mut b).hover_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "tabs bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
        assert_eq!(hover.len(), 1, "hover slot must capture one atom");
        assert_eq!(hover[0], StyleAtom::BackgroundThemeRef("surface-hover"));
    });
}

#[test]
fn scroll_accepts_styled_surface() {
    Runtime::scope("widget-styled-scroll", || {
        let inner: NodeBuilder = container();
        let mut b = scroll(inner, 400.0_f32).bg("surface");
        // 5/5 Wave 1: `impl StateHost` removed from this display-only
        // widget — `.hover()`/`.pressed()` on it is now a compile error by
        // design (panel precedent). Only base StyleAccess is exercised here.
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "scroll bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

// ----- menu -----

#[test]
fn menu_accepts_styled_surface() {
    Runtime::scope("widget-styled-menu-surface", || {
        let open = signal(true);
        let mut b = menu(open)
            .label("File menu")
            .item(MenuItem::new("New"))
            .item(MenuItem::new("Save").shortcut("Ctrl+S"))
            .separator()
            .item(MenuItem::new("Quit"))
            .bg("surface")
            .px(20);
        // 5/5 Wave 1: `impl StateHost` removed from this display-only
        // widget — `.hover()`/`.pressed()` on it is now a compile error by
        // design (panel precedent). Only base StyleAccess is exercised here.
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "menu bg('surface') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil_style::Px(20))),
            "menu px(20) must land in the trait accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn menu_item_chain_works() {
    // Compile-test for the MenuItem builder chain: confirms `shortcut`,
    // `disabled`, and `submenu` return the same `MenuItem` so chained
    // construction inside a `.item(...)` call type-checks. Field
    // mutations are asserted directly — MenuItem's chain is a pure
    // data builder, no reactive closures to install.
    let item = MenuItem::new("Save As\u{2026}")
        .shortcut("Ctrl+Shift+S")
        .disabled(true)
        .submenu(false);
    assert_eq!(item.label, "Save As\u{2026}");
    assert_eq!(item.shortcut.as_deref(), Some("Ctrl+Shift+S"));
    assert!(item.disabled);
    assert!(!item.submenu_indicator);
}
