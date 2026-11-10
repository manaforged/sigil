use ratex_layout::{LayoutOptions, layout, to_display_list, validate_layout};
use ratex_parser::{ParseNode, parse};
use ratex_svg::{SvgOptions, render_to_svg};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

use crate::kernel::NodeBuilder;
use crate::style::{Rgba8, Theme, Token};
use crate::widgets::svg_picture::picture;

const EM_PX: f64 = 12.75;
const STROKE_PER_EM: f64 = 1.5 / 40.0;

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let ast = parse(source).ok()?;
    let (node, (width, _)) = picture(source, move |theme| svg(&ast, theme))?;
    Some((node, width))
}

fn svg(ast: &[ParseNode], theme: &Theme) -> Option<String> {
    let Rgba8 { r, g, b, .. } = theme.color(Token::OnSurface);
    let color = Color::rgb(unit(r), unit(g), unit(b));
    let options = LayoutOptions::default()
        .with_style(MathStyle::Display)
        .with_color(color);
    let tree = layout(ast, &options);
    validate_layout(&tree).ok()?;
    let svg = SvgOptions {
        font_size: EM_PX,
        padding: 0.0,
        stroke_width: EM_PX * STROKE_PER_EM,
        embed_glyphs: true,
        font_dir: String::new(),
    };
    Some(render_to_svg(&to_display_list(&tree), &svg))
}

fn unit(channel: u8) -> f32 {
    f32::from(channel) / 255.0
}
