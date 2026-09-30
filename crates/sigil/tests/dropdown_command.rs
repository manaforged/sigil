#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

#[path = "widget_support/mod.rs"]
mod widget_support;

use sigil::events::EventRouter;
use sigil::events::ShellEvent;
use sigil::events::{KeyEventInit, PhysicalKey};
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil_kittest::stub_text_measure;
use widget_support::{compute, rect_colors};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

#[test]
fn dropdown_has_combobox_role() {
    sigil::reactive::Runtime::scope("dd-role", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(signal(0_usize), vec!["A".to_owned()]));
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::ComboBox);
    });
}

#[test]
fn dropdown_trigger_emits_surface_inset_rect() {
    sigil::reactive::Runtime::scope("dd-surface", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(
            signal(0_usize),
            vec!["A".to_owned(), "B".to_owned()],
        ));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.gray.step(1)));
    });
}

#[test]
fn dropdown_popup_hidden_emits_trigger_rect_only() {
    sigil::reactive::Runtime::scope("dd-popup-hidden", || {
        let mut cx = Cx::new_root();
        let opts = vec!["X".to_owned(), "Y".to_owned(), "Z".to_owned()];
        let root = cx.add(dropdown(signal(0_usize), opts));
        compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let theme = Theme::default();
        let colors = rect_colors(&cx, root);
        assert!(!colors.is_empty(), "trigger rect should emit when closed");
        assert!(
            colors.contains(&theme.gray.step(1)),
            "trigger uses the surface-inset color (gray step 1)"
        );
    });
}

#[test]
fn dropdown_is_focusable() {
    sigil::reactive::Runtime::scope("dd-focusable", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(signal(0_usize), vec!["A".to_owned()]));
        let node = cx.get(root).unwrap();
        assert!(node.a11y.focusable);
    });
}

#[test]
fn dropdown_disabled_not_focusable() {
    sigil::reactive::Runtime::scope("dd-disabled", || {
        let disabled = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(signal(0_usize), vec!["A".to_owned()]).disabled(disabled));
        let node = cx.get(root).unwrap();
        assert!(!node.a11y.focusable);
    });
}

#[test]
fn dropdown_selected_signal_updates_on_open_and_select() {
    sigil::reactive::Runtime::scope("dd-select", || {
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let opts = vec!["Alpha".to_owned(), "Beta".to_owned()];
        let root = cx.add(dropdown(selected, opts));
        let mut layout = LayoutEngine::with_text_measure(stub_text_measure());
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout");
        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(
            &mut cx,
            &layout,
            ShellEvent::Key(KeyEvent::new(KeyEventInit {
                key: Key::ArrowDown.into(),
                physical: PhysicalKey::ArrowDown,
                modifiers: ModifiersState::empty(),
                repeat: false,
                phase: KeyPhase::Down,
                text: None,
            })),
        );
        assert_eq!(selected.get(), 0);
    });
}
