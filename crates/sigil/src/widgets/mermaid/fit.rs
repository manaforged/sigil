use crate::kernel::{NodeBuilder, container};
use crate::layout::{JustifyContent, LayoutStyle};
use crate::reactive::{derived, effect, signal};
use crate::style::NumericAtomKind;
use crate::widgets::Widget;
use crate::widgets::scroll::Horizontal;

const MIN_SCALE: f32 = 0.6;

pub(super) fn fitted(picture: NodeBuilder, (width, height): (f32, f32)) -> NodeBuilder {
    let extent = signal(width);
    let scroll = Horizontal::new(extent, None);
    let viewport = scroll.viewport;
    let scale = derived(move || scale_for(viewport.get(), width));
    let drawn_width = derived(move || width * scale.get());
    let drawn_height = derived(move || height * scale.get());
    effect(move || extent.set(drawn_width.get()));
    let centred = container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .justify_content(JustifyContent::Center),
        )
        .child(
            picture
                .drive(NumericAtomKind::Width, drawn_width)
                .drive(NumericAtomKind::Height, drawn_height),
        );
    scroll.wrap(centred)
}

fn scale_for(viewport: f32, width: f32) -> f32 {
    if viewport <= 0.0 || width <= 0.0 {
        return 1.0;
    }
    (viewport / width).clamp(MIN_SCALE, 1.0)
}
