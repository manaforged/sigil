use std::borrow::Cow;

use crate::kernel::NodeBuilder;
use crate::style::{Theme, Token};
use crate::widgets::svg_picture::{FONT_FAMILY, hex, picture};

const FONT_SIZE: f64 = 14.0;

const SERIES: [Token; 10] = [
    Token::Accent9,
    Token::Success9,
    Token::Warn9,
    Token::Error9,
    Token::Info9,
    Token::Accent7,
    Token::Success7,
    Token::Warn7,
    Token::Error7,
    Token::Info7,
];

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let owned = source.to_string();
    let (node, (width, _)) = picture(source, move |theme| {
        mermaid_svg::render_with(&owned, &palette(theme)).ok()
    })?;
    Some((node, width))
}

fn palette(theme: &Theme) -> mermaid_svg::Theme {
    let c = |token| Cow::Owned(hex(theme, token));
    let series: Cow<'static, [Cow<'static, str>]> =
        Cow::Owned(SERIES.iter().map(|token| c(*token)).collect());
    mermaid_svg::Theme {
        fg: c(Token::OnSurface),
        fg_muted: c(Token::OnSurfaceMuted),
        bg: c(Token::Surface),
        actor_fill: c(Token::Accent3),
        actor_stroke: c(Token::Accent8),
        actor_text_color: None,
        lifeline: c(Token::BorderStrong),
        arrow_stroke: c(Token::OnSurfaceMuted),
        signal_text_color: None,
        note_fill: c(Token::Warn3),
        note_stroke: c(Token::Warn8),
        activation_fill: c(Token::Accent4),
        activation_stroke: c(Token::Accent8),
        frame_label_fill: c(Token::Accent3),
        title_color: None,
        flow_node_fill: c(Token::Accent3),
        flow_node_stroke: c(Token::Accent8),
        flow_edge_stroke: c(Token::OnSurfaceMuted),
        flow_label_bg: c(Token::SurfaceSubtle),
        flow_cluster_fill: c(Token::Gray2),
        flow_cluster_stroke: c(Token::Border),
        cscale_palette: series.clone(),
        pie_palette: series.clone(),
        git_palette: series.clone(),
        xychart_palette: series,
        pie_stroke: Some(c(Token::Surface)),
        pie_opacity: None,
        commit_label_color: None,
        tag_label_color: None,
        quadrant_fills: [None, None, None, None],
        quadrant_default_fills: [
            c(Token::Accent2),
            c(Token::Success3),
            c(Token::Warn4),
            c(Token::Info3),
        ],
        font_family: Cow::Borrowed(FONT_FAMILY),
        font_size: FONT_SIZE,
        responsive: false,
    }
}
