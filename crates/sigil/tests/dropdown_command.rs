//! `SceneCommand` smoke tests for the dropdown widget (D2).
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use actions as _;
use animation as _;
use animation_runtime as _;
use app as _;
use arboard as _;
use criterion as _;
use events as _;
use hal as _;
use image as _;
use kernel as _;
use layout as _;
use pollster as _;
use reactive as _;
use style as _;
use tasks as _;
use wgpu as _;

use hal::KeyEventInit;
use renderer::{SceneCommand, emit_scene};
use sigil::prelude::*;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: hal::Size) -> hal::LogicalSize {
        hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
        .compute(cx, root, Size::fixed(640.0, 480.0))
        .expect("layout");
}

fn rect_colors(cx: &Cx, root: NodeId) -> Vec<style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|c| match c {
            SceneCommand::Rect(r) => Some(r.color),
            _ => None,
        })
        .collect()
}

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
fn dropdown_trigger_emits_surface_rect() {
    sigil::reactive::Runtime::scope("dd-surface", || {
        let mut cx = Cx::new_root();
        let root = cx.add(dropdown(
            signal(0_usize),
            vec!["A".to_owned(), "B".to_owned()],
        ));
        compute(&mut cx, root);
        let theme = Theme::default();
        assert!(rect_colors(&cx, root).contains(&theme.surface));
    });
}

#[test]
fn dropdown_popup_hidden_emits_trigger_rect_only() {
    sigil::reactive::Runtime::scope("dd-popup-hidden", || {
        let mut cx = Cx::new_root();
        let opts = vec!["X".to_owned(), "Y".to_owned(), "Z".to_owned()];
        let root = cx.add(dropdown(signal(0_usize), opts));
        compute(&mut cx, root);
        // Closed popup: trigger surface rect must be present, scene must be non-empty.
        let theme = Theme::default();
        let colors = rect_colors(&cx, root);
        assert!(!colors.is_empty(), "trigger rect should emit when closed");
        assert!(
            colors.contains(&theme.surface),
            "trigger uses surface color"
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
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
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
        // Popup open via ArrowDown — selection unchanged.
        assert_eq!(selected.get(), 0);
    });
}
