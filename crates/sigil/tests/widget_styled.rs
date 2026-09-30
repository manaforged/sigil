#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::indexing_slicing)]

#[path = "events_support/input.rs"]
mod input;
#[path = "scene_support/mod.rs"]
mod scene_support;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::{pointer_down, pointer_move};
use scene_support::color;
use sigil::events::EventRouter;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::LayoutResult;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use widget_support::{compute, rect_colors};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);
const A: Rgba8 = color(200, 20, 20, 255);
const B: Rgba8 = color(20, 200, 20, 255);
const C: Rgba8 = color(20, 20, 200, 255);
const D: Rgba8 = color(200, 200, 20, 255);
const E: Rgba8 = color(20, 200, 200, 255);

#[derive(Clone, Copy)]
enum Drive {
    Rest,
    Hover,
    Press,
    Focus,
}

fn laid_out(build: impl IntoNode, drive: Drive) -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(build);
    let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    let mut router = EventRouter::new();
    match drive {
        Drive::Rest => {}
        Drive::Hover => router.route(&mut cx, &layout, pointer_move(8.0, 8.0)),
        Drive::Press => {
            router.route(&mut cx, &layout, pointer_move(8.0, 8.0));
            router.route(&mut cx, &layout, pointer_down(8.0, 8.0));
        }
        Drive::Focus => router.set_focus(&mut cx, Some(root)),
    }
    (cx, root)
}

fn painted(build: impl IntoNode, drive: Drive) -> Vec<Rgba8> {
    let (cx, root) = laid_out(build, drive);
    rect_colors(&cx, root)
}

fn width(build: impl IntoNode) -> f32 {
    let (cx, root) = laid_out(build, Drive::Rest);
    cx.get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .map_or(0.0, |result| result.rect().width)
}

fn assert_paints(colors: &[Rgba8], expected: Rgba8, what: &str) {
    assert!(
        colors.contains(&expected),
        "{what} must paint {expected:?}, got {colors:?}"
    );
}

#[test]
fn button_styled_bg_emits_atom() {
    Runtime::scope("widget-styled-button-bg", || {
        let colors = painted(button("Run").bg(A).rounded(Radius::Lg), Drive::Rest);
        assert_paints(&colors, A, "button bg");
    });
}

#[test]
fn button_styled_hover_populates_state_slot() {
    Runtime::scope("widget-styled-button-hover", || {
        let colors = painted(button("Run").hover(|s| s.bg(A).opacity(1.0)), Drive::Hover);
        assert_paints(&colors, A, "button hover");
    });
}

#[test]
fn button_styled_accepts_full_compile_surface() {
    Runtime::scope("widget-styled-button-compile", || {
        let build = || {
            button("X")
                .bg(B)
                .layout(LayoutStyle::default().padding_x(48.0).padding_y(8.0))
                .hover(|s| s.bg(A))
        };
        assert_paints(&painted(build(), Drive::Rest), B, "button bg");
        assert_paints(&painted(build(), Drive::Hover), A, "button hover");
        assert!(width(build()) > width(button("X")));
    });
}

#[test]
fn checkbox_styled_bg_emits_atom() {
    Runtime::scope("widget-styled-checkbox-bg", || {
        let colors = painted(
            checkbox(signal(false)).bg(A).rounded(Radius::Md),
            Drive::Rest,
        );
        assert_paints(&colors, A, "checkbox bg");
    });
}

#[test]
fn checkbox_styled_hover_populates_state_slot() {
    Runtime::scope("widget-styled-checkbox-hover", || {
        let colors = painted(checkbox(signal(false)).hover(|s| s.bg(A)), Drive::Hover);
        assert_paints(&colors, A, "checkbox hover");
    });
}

#[test]
fn dropdown_styled_emits_on_trigger_accumulator() {
    Runtime::scope("widget-styled-dropdown-bg", || {
        let build = |px: f32| {
            dropdown(signal(0_usize), vec!["a".into(), "b".into()])
                .bg(A)
                .layout(LayoutStyle::default().width(px))
        };
        assert_paints(
            &painted(build(200.0), Drive::Rest),
            A,
            "dropdown trigger bg",
        );
        assert!((width(build(200.0)) - 200.0).abs() < f32::EPSILON);
        assert!((width(build(260.0)) - 260.0).abs() < f32::EPSILON);
    });
}

#[test]
fn dropdown_styled_pressed_populates_state_slot() {
    Runtime::scope("widget-styled-dropdown-pressed", || {
        let build = dropdown(signal(0_usize), vec!["a".into()]).pressed(|s| s.bg(A));
        assert_paints(&painted(build, Drive::Press), A, "dropdown pressed");
    });
}

#[test]
fn panel_accepts_styled_surface() {
    Runtime::scope("widget-styled-panel-surface", || {
        let build = |padding: f32| {
            panel()
                .title("Settings")
                .bg(A)
                .layout(LayoutStyle::default().padding_x(padding))
        };
        assert_paints(&painted(build(20.0), Drive::Rest), A, "panel bg");
        assert!(width(build(20.0)) >= width(build(0.0)));
    });
}

#[test]
fn text_field_styled_composes_with_state_variant() {
    Runtime::scope("widget-styled-text-field-state-compose", || {
        let build = || {
            let field = text_field(signal(String::new()))
                .placeholder("Search…")
                .bg(A)
                .layout(LayoutStyle::default().padding_x(10.0))
                .hover(|s| s.bg(B))
                .focus(|s| s.bg(C))
                .pressed(|s| s.bg(D));
            StateVariants::disabled_style(field, |s| s.bg(E))
        };
        assert_paints(&painted(build(), Drive::Rest), A, "text_field bg");
        assert_paints(&painted(build(), Drive::Hover), B, "text_field hover");
        assert_paints(&painted(build(), Drive::Focus), C, "text_field focus");
        assert_paints(&painted(build(), Drive::Press), D, "text_field pressed");
        assert!(!painted(build(), Drive::Rest).contains(&E));
    });
}

#[test]
fn slider_accepts_styled_surface() {
    Runtime::scope("widget-styled-slider", || {
        let build = slider(signal(0.5_f32), 0.0..=1.0)
            .bg(A)
            .layout(LayoutStyle::default().padding(8.0));
        assert_paints(&painted(build, Drive::Rest), A, "slider bg");
    });
}

#[test]
fn progress_accepts_styled_surface() {
    Runtime::scope("widget-styled-progress", || {
        let build = progress(signal(0.75_f32), 1.0).bg(A);
        assert_paints(&painted(build, Drive::Rest), A, "progress bg");
    });
}

#[test]
fn style_and_typed_methods_share_one_store() {
    Runtime::scope("widget-styled-one-store", || {
        let layered = button("X")
            .layout(LayoutStyle::default().padding(8.0))
            .layout(LayoutStyle::default().padding(12.0));
        let single = button("X").layout(LayoutStyle::default().padding(12.0));
        assert!(
            (width(layered) - width(single)).abs() < f32::EPSILON,
            "the later padding must win"
        );

        let padded = LayoutStyle::default().padding(4.0);
        let _cb = checkbox(signal(false)).layout(padded.clone());
        let _tg = toggle(signal(false)).layout(padded.clone());
        let _sl = slider(signal(0.5_f32), 0.0..=1.0).layout(padded);
    });
}

#[test]
fn modal_accepts_styled_surface() {
    Runtime::scope("widget-styled-modal-surface", || {
        let build = modal(signal(true))
            .title("Confirm action")
            .bg(A)
            .layout(LayoutStyle::default().padding_x(20.0));
        assert_paints(&painted(build, Drive::Rest), A, "modal bg");
    });
}

#[test]
fn tabs_accepts_styled_surface() {
    Runtime::scope("widget-styled-tabs", || {
        let build = || {
            tabs()
                .active(signal(0_usize))
                .labels(["A", "B"])
                .label("Settings sections")
                .bg(A)
                .hover(|s| s.bg(B))
        };
        assert_paints(&painted(build(), Drive::Rest), A, "tabs bg");
        assert_paints(&painted(build(), Drive::Hover), B, "tabs hover");
    });
}

#[test]
fn scroll_accepts_styled_surface() {
    Runtime::scope("widget-styled-scroll", || {
        let inner: NodeBuilder = container();
        let build = scroll(inner).extent(400.0_f32).bg(A);
        assert_paints(&painted(build, Drive::Rest), A, "scroll bg");
    });
}

#[test]
fn menu_accepts_styled_surface() {
    Runtime::scope("widget-styled-menu-surface", || {
        let build = menu(signal(true))
            .label("File menu")
            .item(MenuItem::new("New"))
            .item(MenuItem::new("Save").shortcut("Ctrl+S"))
            .separator()
            .item(MenuItem::new("Quit"))
            .bg(A)
            .layout(LayoutStyle::default().padding_x(20.0));
        assert_paints(&painted(build, Drive::Rest), A, "menu bg");
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

#[test]
fn layout_override_sizes_every_widget_root() {
    Runtime::scope("widget-styled-layout-override", || {
        let wide = || LayoutStyle::default().width(210.0);
        let value = signal(0.5_f32);
        let widths = [
            ("panel", width(panel().layout(wide()))),
            ("progress", width(progress(value, 1.0).layout(wide()))),
            ("gauge", width(gauge(value).layout(wide()))),
            ("slider", width(slider(value, 0.0..=1.0).layout(wide()))),
            ("scroll", width(scroll(container()).layout(wide()))),
            (
                "tooltip",
                width(tooltip("tip").child(button("X")).layout(wide())),
            ),
            (
                "interactive",
                width(interactive(container()).layout(wide())),
            ),
            ("diff", width(diff("").width(210.0))),
        ];
        for (name, laid) in widths {
            assert!(
                (laid - 210.0).abs() < f32::EPSILON,
                "{name} laid out at {laid}"
            );
        }
    });
}
