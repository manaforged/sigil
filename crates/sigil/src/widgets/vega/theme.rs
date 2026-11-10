use serde_json::json;

use crate::style::{Theme, Token};
use crate::widgets::chart::series_tokens;
use crate::widgets::svg_picture::{FONT_FAMILY, hex};

pub(super) fn config(theme: &Theme) -> String {
    let color = |token| hex(theme, token);
    let (text, muted, border) = (
        color(Token::OnSurface),
        color(Token::OnSurfaceMuted),
        color(Token::Border),
    );
    let category: Vec<String> = series_tokens().map(color).collect();
    let first = category.first().cloned().unwrap_or_else(|| text.clone());
    json!({
        "background": "transparent",
        "font": FONT_FAMILY,
        "view": { "stroke": border },
        "axis": {
            "domainColor": muted,
            "tickColor": muted,
            "gridColor": border,
            "labelColor": muted,
            "titleColor": text,
        },
        "legend": { "labelColor": muted, "titleColor": text },
        "header": { "labelColor": text, "titleColor": text },
        "title": { "color": text, "subtitleColor": muted },
        "mark": { "color": first },
        "text": { "color": text },
        "range": { "category": category },
    })
    .to_string()
}
