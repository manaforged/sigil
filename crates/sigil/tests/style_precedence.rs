#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[path = "events_support/input.rs"]
mod input;
#[path = "widget_support/mod.rs"]
mod widget_support;

use input::pointer_move;
use sigil::events::EventRouter;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::{LayoutEngine, LayoutResult};
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::style::scale::ruby_light;
use sigil::style::{Style, Token, render_style_for};
use widget_support::{compute, rect_colors};

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

fn laid_out(build: impl IntoNode) -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let root = cx.add(build);
    let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
    (cx, root, layout)
}

fn width(build: impl IntoNode) -> f32 {
    let (cx, root, _) = laid_out(build);
    cx.get(root)
        .and_then(|node| node.get_extension::<LayoutResult>())
        .map_or(0.0, |result| result.rect().width)
}

fn rest_and_hover_colors(build: impl IntoNode) -> (Vec<Rgba8>, Vec<Rgba8>) {
    let (mut cx, root, layout) = laid_out(build);
    let rest = rect_colors(&cx, root);
    EventRouter::new().route(&mut cx, &layout, pointer_move(8.0, 8.0));
    (rest, rect_colors(&cx, root))
}

#[test]
fn a_padding_class_widens_a_button() {
    Runtime::scope("precedence-class-padding", || {
        let plain = width(button("A"));
        let padded = width(button("A").class("p-32"));
        assert!(
            padded >= plain + 40.0,
            "p-32 left the button at {padded} (plain {plain})"
        );
    });
}

#[test]
fn typed_padding_widens_a_button() {
    Runtime::scope("precedence-typed-padding", || {
        let plain = width(button("A"));
        let padded = width(button("A").padding(32));
        assert!(
            padded >= plain + 40.0,
            "padding(32) left the button at {padded} (plain {plain})"
        );
    });
}

#[test]
fn a_hover_state_in_style_paints_on_hover() {
    Runtime::scope("precedence-style-hover", || {
        let (rest, hovered) = rest_and_hover_colors(
            container()
                .layout(LayoutStyle::block().fixed_size(32.0, 32.0))
                .style(Style::new().class("bg-surface hover:bg-accent")),
        );
        assert_ne!(rest, hovered, "the hover state of .style() never painted");
    });
}

#[test]
fn a_background_set_on_a_button_holds_under_hover() {
    Runtime::scope("precedence-button-hover", || {
        let ruby = ruby_light().step(9);
        let (rest, hovered) = rest_and_hover_colors(button("Save").bg(ruby));
        assert!(rest.contains(&ruby), "rest colors: {rest:?}");
        assert!(
            hovered.contains(&ruby),
            "hover replaced the background: {hovered:?}"
        );
    });
}

#[test]
fn a_border_set_on_a_text_field_holds_at_rest() {
    Runtime::scope("precedence-field-border", || {
        let value = signal(String::new());
        let (cx, root, _) = laid_out(text_field(value).class([Token::Accent9.border()]));
        let node = cx.get(root).expect("the field is in the tree");
        let theme = Theme::light();
        let style = render_style_for(root, node, &theme, None);
        assert_eq!(style.border_color, Some(theme.color(Token::Accent9)));
    });
}
