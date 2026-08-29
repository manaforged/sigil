use std::rc::Rc;

use sigil_kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived};

use super::button::icon_button;
use super::slider::slider;

#[must_use]
pub fn transport(
    playing: Signal<bool>,
    progress: Signal<f32>,
    toggle: impl Fn() + 'static,
) -> TransportBuilder {
    TransportBuilder {
        playing,
        progress,
        toggle: Rc::new(toggle),
        seek: None,
    }
}

pub struct TransportBuilder {
    playing: Signal<bool>,
    progress: Signal<f32>,
    toggle: Rc<dyn Fn()>,
    seek: Option<Rc<dyn Fn(f32)>>,
}

impl TransportBuilder {
    #[must_use]
    pub fn seek(mut self, seek: impl Fn(f32) + 'static) -> Self {
        self.seek = Some(Rc::new(seek));
        self
    }
}

impl IntoNode for TransportBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        into_node(self).build(cx)
    }
}

fn into_node(builder: TransportBuilder) -> NodeBuilder {
    let label = derived(move || {
        if builder.playing.get() {
            String::from("\u{23F8}")
        } else {
            String::from("\u{25B6}")
        }
    });
    let toggle = builder.toggle;
    let mut track = slider(builder.progress, 0.0..=1.0).step(0.01);
    if let Some(seek) = builder.seek {
        track = track.change(move |ratio| seek(ratio));
    }
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(8.0)
                .align_items(AlignItems::Center)
                .fill_width(),
        )
        .child(icon_button(label, move || toggle()).accessible("Play or pause"))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0))
                .child(track),
        )
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
