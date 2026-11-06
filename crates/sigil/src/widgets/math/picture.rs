use std::sync::LazyLock;

use latex_rust::{Color, Dim, MathBox, MathFont, MathStyle, SvgOptions};

use crate::kernel::NodeBuilder;
use crate::style::{Rgba8, Theme, Token};
use crate::widgets::svg_picture::picture;

static FONT: LazyLock<Option<MathFont>> = LazyLock::new(|| MathFont::stix_two_math().ok());

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let font = FONT.as_ref()?;
    let tree = layout(source, font)?;
    let (node, (width, _)) = picture(source, move |theme| {
        latex_rust::render_svg(&tree, font, &options(theme)).ok()
    })?;
    Some((node, width))
}

fn layout(source: &str, font: &MathFont) -> Option<MathBox> {
    let ast = latex_rust::parse(source).ok()?;
    latex_rust::layout_with_em_size_pt(&ast, font, MathStyle::Display, &em()).ok()
}

fn options(theme: &Theme) -> SvgOptions {
    let Rgba8 { r, g, b, .. } = theme.color(Token::OnSurface);
    SvgOptions {
        font_size_pt: em(),
        color: Color::rgb(r, g, b),
        display: true,
    }
}

fn em() -> Dim {
    Dim::ratio(105, 8)
}
