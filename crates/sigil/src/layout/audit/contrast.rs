use crate::kernel::{Cx, NodeId, NodeType};
use crate::layout::LayoutEngine;
use crate::style::srgb::{AA_TEXT, contrast as ratio};
use crate::style::{Color, TextStyle, Theme, render_style_for, text_style_for};

use super::{Issue, label};

const LARGE: f32 = 3.0;
const LARGE_PX: f32 = 24.0;
const LARGE_BOLD_PX: f32 = 18.5;
const BOLD: u16 = 700;
const OPAQUE: u16 = 10_000;

#[derive(Clone)]
struct Inherited<'a> {
    theme: &'a Theme,
    text: Option<Color>,
    style: TextStyle,
    behind: Color,
}

fn needed(style: &TextStyle) -> f32 {
    let large = style.font_size >= LARGE_PX
        || (style.font_size >= LARGE_BOLD_PX && style.font_weight >= BOLD);
    if large { LARGE } else { AA_TEXT }
}

fn has_words(cx: &Cx, id: NodeId, layout: &LayoutEngine) -> bool {
    let shown = layout
        .result(id)
        .is_some_and(|result| result.rect().width > 0.0 && result.rect().height > 0.0);
    shown
        && cx
            .get(id)
            .and_then(|node| match &node.kind {
                NodeType::Text { content, .. } => Some(content.resolve()),
                _ => None,
            })
            .is_some_and(|words| words.chars().any(char::is_alphanumeric))
}

pub(super) fn contrast(
    cx: &Cx,
    layout: &LayoutEngine,
    root: NodeId,
    theme: &Theme,
    out: &mut Vec<Issue>,
) {
    let start = Inherited {
        theme,
        text: None,
        style: TextStyle::default(),
        behind: theme.surface,
    };
    walk(cx, layout, root, &start, out);
}

fn walk(cx: &Cx, layout: &LayoutEngine, id: NodeId, above: &Inherited<'_>, out: &mut Vec<Issue>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node
        .a11y
        .disabled
        .is_some_and(|flag| flag.try_peek().unwrap_or(false))
    {
        return;
    }
    let theme = node.extension::<Theme>().unwrap_or(above.theme);
    let render = render_style_for(node, theme, above.text);
    if render.opacity < OPAQUE
        || render.paint.gradient.is_some()
        || render.paint.image.is_some()
    {
        return;
    }
    let behind = render
        .background_color
        .map_or(above.behind, |color| color.over(above.behind));
    let style = text_style_for(node, theme, &above.style);
    if let Some(ink) = render.text_color
        && has_words(cx, id, layout)
    {
        let (found, floor) = (ratio(ink.over(behind), behind), needed(&style));
        if found < floor {
            out.push(Issue::Contrast {
                node: label(id, node),
                ratio: found,
                needed: floor,
            });
        }
    }
    let inherited = Inherited {
        theme,
        text: render.text_color.or(above.text),
        style,
        behind,
    };
    for child in node.children() {
        walk(cx, layout, *child, &inherited, out);
    }
}
