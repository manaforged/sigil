use rust_latex_parser::EqNode;

use crate::kernel::{NodeBuilder, fragment, text};
use crate::reactive::Reactive;

/// A typeset LaTeX formula in display style.
#[must_use]
pub fn math(source: impl Reactive<String>) -> NodeBuilder {
    fragment().children_reactive(move || [block(&source.read())])
}

pub(crate) fn block(source: &str) -> NodeBuilder {
    text(unicode(source)).into()
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
