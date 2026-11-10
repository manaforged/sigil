use rust_latex_parser::EqNode;

use crate::kernel::{NodeBuilder, Role, container, fragment, text};
use crate::layout::{JustifyContent, LayoutStyle};
use crate::reactive::{Reactive, signal};
use crate::widgets::Widget;
use crate::widgets::scroll::Horizontal;

#[cfg(feature = "scene")]
mod picture;

const MAX_FORMULA_BYTES: usize = 8 * 1024;

/// A typeset LaTeX formula in display style.
#[must_use]
pub fn math(source: impl Reactive<String>) -> NodeBuilder {
    fragment().children_reactive(move || [block(&source.read())])
}

pub(crate) fn block(source: &str) -> NodeBuilder {
    if !crate::style::Metrics::current().is_grid()
        && source.len() <= MAX_FORMULA_BYTES
        && !crate::widgets::source_too_large(source)
        && let Some((formula, width)) = picture(source.trim())
    {
        let centred = container()
            .layout(
                LayoutStyle::flex_row()
                    .fill_width()
                    .min_size(width, 0.0)
                    .justify_content(JustifyContent::Center),
            )
            .class("py-2")
            .child(formula);
        return Horizontal::new(signal(width), None)
            .wrap(centred)
            .role(Role::Figure)
            .label("Math")
            .description(source.trim().to_string());
    }
    text(unicode(source)).into()
}

#[cfg(feature = "scene")]
fn picture(source: &str) -> Option<(NodeBuilder, f32)> {
    picture::figure(source)
}

#[cfg(not(feature = "scene"))]
const fn picture(_source: &str) -> Option<(NodeBuilder, f32)> {
    None
}

pub(crate) fn unicode(source: &str) -> String {
    let tree = rust_latex_parser::parse_equation(source);
    let mut nodes = vec![&tree];
    let mut marks = Vec::new();
    while let Some(node) = nodes.pop() {
        if let EqNode::Seq(parts) = &node.kind {
            for pair in parts.windows(2) {
                if let [left, right] = pair
                    && matches!(left.kind, EqNode::Frac(..))
                    && !matches!(right.kind, EqNode::Op { .. })
                    && !matches!(right.kind, EqNode::Space(width) if width > 0.0)
                {
                    if left.span.is_empty() {
                        return source.to_string();
                    }
                    marks.push((left.span.start, true));
                    marks.push((left.span.end, false));
                }
            }
        }
        nodes.extend(node.children());
    }
    if marks.is_empty() {
        return rust_latex_parser::to_unicode(&tree);
    }
    if marks
        .iter()
        .any(|(offset, _)| !source.is_char_boundary(*offset))
    {
        return source.to_string();
    }
    marks.sort_unstable_by_key(|mark| std::cmp::Reverse(*mark));
    let mut grouped = source.to_string();
    for (offset, open) in marks {
        grouped.insert_str(offset, if open { r"\left(" } else { r"\right)" });
    }
    rust_latex_parser::to_unicode(&rust_latex_parser::parse_equation(&grouped))
}
