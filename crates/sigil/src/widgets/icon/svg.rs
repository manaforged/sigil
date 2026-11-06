use std::sync::Arc;

use crate::kernel::{NodeBuilder, container};
use crate::style::{Rgba8, Token};
use crate::widgets::svg_picture::square;

const ROOT: &str = "<svg";

/// An SVG icon drawn as paths in a square of `extent` logical pixels, with `currentColor` and the default fill set to `token`.
pub(super) fn glyph(svg: Arc<str>, token: Token, extent: f32) -> NodeBuilder {
    square("icon", extent, move |theme| {
        tinted(&svg, theme.color(token))
    })
    .unwrap_or_else(container)
}

fn tinted(svg: &str, color: Rgba8) -> Option<String> {
    let start = svg.find(ROOT)? + ROOT.len();
    let root = &svg[start..start + svg[start..].find('>')?];
    let Rgba8 { r, g, b, a } = color;
    let mut extra = String::new();
    if !names(root, "color") {
        extra.push_str(&format!(" color=\"#{r:02x}{g:02x}{b:02x}{a:02x}\""));
    }
    if !names(root, "fill") {
        extra.push_str(" fill=\"currentColor\"");
    }
    Some(format!("{}{extra}{}", &svg[..start], &svg[start..]))
}

fn names(root: &str, attribute: &str) -> bool {
    root.split_whitespace().any(|part| {
        part.strip_prefix(attribute)
            .is_some_and(|rest| rest.starts_with('='))
    })
}
