use crate::events::NodeEventsExt;
use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{JustifyContent, LayoutBuilderExt, LayoutStyle};
use crate::style::{NodeStyleExt, Token};

use super::builder::{SliderRuntime, ValueChange};
use super::handlers::quantize;

pub(super) fn with_marks(
    slider: NodeBuilder,
    runtime: SliderRuntime,
    marks: Vec<String>,
    on_change: ValueChange,
) -> NodeBuilder {
    let column = runtime.length.map_or_else(
        || LayoutStyle::flex_column().fill_width(),
        |width| LayoutStyle::flex_column().width(width),
    );
    container()
        .layout(column)
        .child(slider)
        .child(row(runtime, marks, on_change))
}

fn row(runtime: SliderRuntime, marks: Vec<String>, on_change: ValueChange) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .justify_content(JustifyContent::SpaceBetween),
        )
        .children_reactive(move || {
            let current = runtime.value.get();
            let last = marks.len().saturating_sub(1).max(1);
            marks
                .iter()
                .enumerate()
                .map(|(index, mark)| {
                    #[expect(clippy::cast_precision_loss, reason = "mark counts are small")]
                    let at = quantize(
                        runtime.range,
                        runtime
                            .range
                            .span()
                            .mul_add(index as f32 / last as f32, runtime.range.min),
                        runtime.step,
                    );
                    let tone = if (at - current).abs() < f32::EPSILON {
                        Token::Accent11
                    } else {
                        Token::Gray10
                    };
                    let change = std::rc::Rc::clone(&on_change);
                    container()
                        .child(text(mark.clone()).class("text-xs").class(tone.text()))
                        .pointer_down(move |event| {
                            if !runtime.disabled() {
                                runtime.value.set(at);
                                change(at);
                                event.stop_propagation();
                            }
                        })
                })
                .collect::<Vec<_>>()
        })
}
