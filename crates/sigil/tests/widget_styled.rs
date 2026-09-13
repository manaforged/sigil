#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

use sigil::prelude::{
    FctSpawn, Fill, Geometry, MenuItem, NodeBuilder, Spacing, StateVariants, StyleAccess,
    StyleAtom, Visual, button, checkbox, container, dropdown, floating_text, menu, modal, panel,
    progress, scroll, signal, slider, tabs, text_field, toggle,
};
use sigil::reactive::Runtime;

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
            atoms.contains(&StyleAtom::BorderRadius(sigil::style::Px(8))),
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
            atoms.contains(&StyleAtom::BorderRadius(sigil::style::Px(6))),
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
            atoms.contains(&StyleAtom::PaddingAll(sigil::style::Px(12))),
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

#[test]
fn panel_accepts_styled_surface() {
    Runtime::scope("widget-styled-panel-surface", || {
        let mut b = panel().title("Settings").bg("accent").px(20);
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("accent")),
            "panel bg('accent') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil::style::Px(20))),
            "panel px(20) must land in the trait accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn text_field_styled_composes_with_state_variant() {
    Runtime::scope("widget-styled-text-field-state-compose", || {
        let value = signal(String::new());
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
            base.contains(&StyleAtom::PaddingX(sigil::style::Px(10))),
            "text_field px(10) must land in trait accumulator base, got {base:?}",
        );
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
            atoms.contains(&StyleAtom::PaddingAll(sigil::style::Px(8))),
            "padding(8) must land in the accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn progress_accepts_styled_surface() {
    Runtime::scope("widget-styled-progress", || {
        let value = signal(0.75_f32);
        let mut b = progress(value, 1.0).bg("surface");
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "progress bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn floating_text_accepts_styled_surface() {
    Runtime::scope("widget-styled-floating-text", || {
        let spawn = signal(FctSpawn::default());
        let mut b = floating_text(spawn)
            .duration(1.5)
            .capacity(16)
            .bg("surface");
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "floating_text bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn trap3_style_and_trait_style_coexist() {
    Runtime::scope("widget-styled-trap3-style", || {
        let my_style = sigil::style::Style::new()
            .padding(8)
            .background_theme("surface");
        let mut b = button("X").style(my_style);
        let style_view = StyleAccess::style_mut(&mut b);
        assert!(style_view.is_empty());

        let checked = signal(false);
        let _cb = checkbox(checked).style(sigil::style::Style::new().padding(4));

        let on = signal(false);
        let _tg = toggle(on).style(sigil::style::Style::new().padding(4));

        let value = signal(0.5_f32);
        let _sl = slider(value, 0.0..=1.0).style(sigil::style::Style::new().padding(4));
    });
}

#[test]
fn modal_accepts_styled_surface() {
    Runtime::scope("widget-styled-modal-surface", || {
        let mut b = modal(signal(false))
            .title("Confirm action")
            .bg("surface")
            .px(20);
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "modal bg('surface') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil::style::Px(20))),
            "modal px(20) must land in the trait accumulator, got {atoms:?}",
        );
    });
}

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
        let atoms = StyleAccess::style_mut(&mut b).as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "scroll bg('surface') must hit the trait accumulator, got {atoms:?}",
        );
    });
}

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
        let style = StyleAccess::style_mut(&mut b);
        let atoms = style.as_atoms().to_vec();
        assert!(
            atoms.contains(&StyleAtom::BackgroundThemeRef("surface")),
            "menu bg('surface') must land in the trait accumulator, got {atoms:?}",
        );
        assert!(
            atoms.contains(&StyleAtom::PaddingX(sigil::style::Px(20))),
            "menu px(20) must land in the trait accumulator, got {atoms:?}",
        );
    });
}

#[test]
fn menu_item_chain_works() {
    let item = MenuItem::new("Save As\u{2026}")
        .shortcut("Ctrl+Shift+S")
        .disabled(true)
        .submenu(false);
    assert_eq!(item.label, "Save As\u{2026}");
    assert_eq!(item.shortcut.as_deref(), Some("Ctrl+Shift+S"));
    assert!(item.disabled);
    assert!(!item.submenu_indicator);
}
