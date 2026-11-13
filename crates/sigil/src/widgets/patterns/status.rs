use crate::kernel::{NodeBuilder, Role, container, text};
use crate::layout::snap::line;
use crate::layout::{AlignItems, JustifyContent, LayoutStyle, Length, Space};
use crate::reactive::{Reactive, Signal, memo};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{Radius, StyleAtom, Token, Tone, space};
use crate::widgets::{Layout, Widget};

use super::super::chip::chip;
use super::super::icon::CHECK_GLYPH;

const BADGE_MAX: usize = 99;
const MARKER_PX: f32 = space(6);

/// A count shown as a small pill; it hides at zero and reads `99+` past 99.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::style::Tone;
/// use sigil::widgets::badge;
///
/// fn inbox() -> impl IntoNode {
///     let unread = signal(3_usize);
///     row().child(text("Inbox")).child(badge(unread, Tone::Error))
/// }
/// ```
pub fn badge(count: impl Reactive<usize>, tone: Tone) -> NodeBuilder {
    let count = count.into_signal();
    let shown = memo(move || count.get() > 0);
    let shown_text = memo(move || {
        let n = count.get();
        if n > BADGE_MAX {
            format!("{BADGE_MAX}+")
        } else {
            n.to_string()
        }
    });
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::flex_row().align_items(AlignItems::Center))
        .visible(shown)
        .children_reactive(move || {
            shown.get().then(|| {
                chip(move || shown_text.get(), tone)
                    .map_node(move |node| node.role(Role::Status).label(shown_text))
            })
        })
}

static BAR: &[StyleAtom] = &[Radius::Sm.atom(), Tone::Neutral.step(4).background()];

/// A loading placeholder of `lines` muted bars, announced to screen readers as a polite "Loading" status.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::skeleton;
///
/// fn loading() -> impl IntoNode {
///     column().child(skeleton(3))
/// }
/// ```
pub fn skeleton(lines: usize) -> NodeBuilder {
    let lines = lines.max(1);
    let mut root = container()
        .role(Role::Status)
        .label(theme_label(StringKey::Loading))
        .layout(LayoutStyle::flex_column().fill_width().gap(Space::Xs));
    for index in 0..lines {
        let width = if index + 1 == lines && lines > 1 {
            Length::Percent(0.6)
        } else {
            Length::Fill
        };
        root = root.child(
            container()
                .layout(LayoutStyle::block().width(width).height(line(space(4))))
                .class(BAR)
                .hittable(false),
        );
    }
    root
}

static MARK_DONE: &[StyleAtom] = &[
    Radius::Full.atom(),
    Tone::Primary.step(9).background(),
    Token::OnAccent.text(),
];
static MARK_TODO: &[StyleAtom] = &[
    Radius::Full.atom(),
    Tone::Neutral.step(4).background(),
    Tone::Neutral.step(12).text(),
];
static LABEL_CURRENT: &[StyleAtom] = &[Token::OnSurface.text()];
static LABEL_OTHER: &[StyleAtom] = &[Tone::Neutral.step(11).text()];

/// A row of numbered steps: steps before `current` show a check, and `current` is selected.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::stepper;
///
/// fn checkout() -> impl IntoNode {
///     let step = signal(1_usize);
///     stepper(["Cart", "Shipping", "Pay"], step)
/// }
/// ```
pub fn stepper<S: Into<String>>(
    steps: impl IntoIterator<Item = S>,
    current: impl Reactive<usize>,
) -> NodeBuilder {
    let current = current.into_signal();
    let mut root = container().role(Role::List).layout(
        LayoutStyle::flex_row()
            .gap(Space::Lg)
            .align_items(AlignItems::Center),
    );
    for (index, name) in steps.into_iter().enumerate() {
        root = root.child(step(index, name.into(), current));
    }
    root
}

fn step(index: usize, name: String, current: Signal<usize>) -> NodeBuilder {
    let selected = memo(move || current.get() == index);
    let reached = memo(move || current.get() >= index);
    let number = (index + 1).to_string();
    container()
        .role(Role::ListItem)
        .label(name.clone())
        .selected(selected)
        .layout(
            LayoutStyle::flex_row()
                .gap(Space::Sm)
                .align_items(AlignItems::Center),
        )
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(MARKER_PX, MARKER_PX)
                        .align_items(AlignItems::Center)
                        .justify_content(JustifyContent::Center)
                        .shrink(0.0),
                )
                .style_reactive(move || if reached.get() { MARK_DONE } else { MARK_TODO })
                .child(
                    text(move || {
                        if current.get() > index {
                            CHECK_GLYPH.to_owned()
                        } else {
                            number.clone()
                        }
                    })
                    .class("text-xs font-medium")
                    .hittable(false),
                ),
        )
        .child(
            text(name)
                .nowrap()
                .style_reactive(move || {
                    if selected.get() {
                        LABEL_CURRENT
                    } else {
                        LABEL_OTHER
                    }
                })
                .hittable(false),
        )
}
