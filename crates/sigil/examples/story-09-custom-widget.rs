use sigil::node::{Cx, NodeId};
use sigil::prelude::*;
use sigil::widgets::{Framed, WidgetFrame};

fn main() -> AppResult {
    run("Custom widget", review_form)
}

/// Builder of a star rating input.
#[derive(Debug)]
#[must_use]
pub struct RatingBuilder {
    value: Signal<u8>,
    max: u8,
    disabled: Option<Signal<bool>>,
    frame: WidgetFrame,
}

/// Creates a star rating input bound to `value`.
pub const fn rating(value: Signal<u8>) -> RatingBuilder {
    RatingBuilder {
        value,
        max: 5,
        disabled: None,
        frame: WidgetFrame::new(),
    }
}

impl RatingBuilder {
    /// Sets the number of stars shown by the rating input.
    pub const fn max(mut self, max: u8) -> Self {
        self.max = max;
        self
    }

    /// Disables the rating input while `disabled` is true.
    pub fn disabled(mut self, disabled: impl Reactive<bool>) -> Self {
        self.disabled = Some(disabled.into_signal());
        self
    }

    fn root(self) -> NodeBuilder {
        let value = self.value;
        let max = self.max.max(1);
        let disabled = self.disabled;
        let stars = (1..=max).map(|star| {
            let mut item = button(move || if value.get() >= star { "★" } else { "☆" })
                .ghost()
                .size(ControlSize::Compact)
                .accessible(format!("{star} of {max}"))
                .activate(move || value.set(star));
            if let Some(flag) = disabled {
                item = item.disabled(flag);
            }
            item
        });
        let root = row()
            .role(Role::Slider)
            .accessible("Rating")
            .focusable(true)
            .gap(Space::Xxs)
            .key(NamedKey::ArrowRight, move |event| {
                event.stop_propagation();
                value.update(|v| *v = (*v + 1).min(max));
            })
            .key(NamedKey::ArrowLeft, move |event| {
                event.stop_propagation();
                value.update(|v| *v = v.saturating_sub(1));
            })
            .children(stars);
        self.frame.finish(root)
    }
}

impl Framed for RatingBuilder {
    fn frame_mut(&mut self) -> &mut WidgetFrame {
        &mut self.frame
    }
}

impl IntoNode for RatingBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.root().build(cx)
    }
}

pub(crate) fn review_form() -> impl IntoNode {
    let score = signal(3_u8);
    let quality = signal(2_u8);
    column()
        .padding(Space::Md)
        .gap(Space::Sm)
        .child(
            rating(score)
                .max(5)
                .padding(Space::Md)
                .gap(Space::Sm)
                .background(Token::Surface)
                .hover(|s| s.background(Token::SurfaceHover))
                .pressed(|s| s.background(Token::SurfacePressed)),
        )
        .child(field("Quality", rating(quality).max(4)).padding(Space::Sm))
        .child(text(move || {
            format!("Score: {}, Quality: {}", score.get(), quality.get())
        }))
}
