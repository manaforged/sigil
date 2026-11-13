use crate::layout::Space;

use super::layout_ext::LayoutOverrides;
use super::shared::{METER_TRACK, METER_TRACK_HEIGHT, accessible_name_or, caller_style};
use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::intern::{KEYED_CAP, Keyed, keyed};
use crate::style::strings::StringKey;
use crate::style::{Radius, Style, StyleAtom, Token, Tone, space};
use crate::widgets::Layout;

const TRACK_WIDTH: f32 = 160.0;
const GAUGE_GAP: f32 = space(2);

/// Builder of a gauge.
#[derive(Debug)]
#[must_use]
pub struct GaugeBuilder {
    value: Signal<f32>,
    max: f32,
    label: Option<TextContent>,
    warn_threshold: Option<f32>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A labelled meter bar that shows a value against a maximum.
pub fn gauge(value: Signal<f32>) -> GaugeBuilder {
    GaugeBuilder::new(value)
}

impl GaugeBuilder {
    /// Creates a gauge of `value` against a maximum of 1.0.
    pub(crate) fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            max: 1.0,
            label: None,
            warn_threshold: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the value at which the bar is full, at least `f32::EPSILON`; the default is 1.0.
    pub const fn max(mut self, max: f32) -> Self {
        self.max = if max > f32::EPSILON {
            max
        } else {
            f32::EPSILON
        };
        self
    }

    /// Sets the text shown before the bar, which is also the accessible name.
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws the bar in the warning color while `value / max` is below `threshold`.
    pub const fn warn(mut self, threshold: f32) -> Self {
        self.warn_threshold = Some(threshold);
        self
    }

    fn root(self) -> NodeBuilder {
        let (value, max, warn) = (self.value, self.max, self.warn_threshold);
        let mut node = container()
            .role(Role::ProgressIndicator)
            .label(accessible_name_or(self.label.as_ref(), StringKey::Gauge))
            .value(value, 0.0, max)
            .layout(
                LayoutStyle::flex_row()
                    .gap(Space::px(GAUGE_GAP))
                    .align_items(AlignItems::Center),
            );
        if let Some(label) = self.label {
            node = node.child(text(label).class(Token::OnSurface.text()));
        }
        node = node.child(
            container()
                .role(Role::GenericContainer)
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(TRACK_WIDTH, METER_TRACK_HEIGHT)
                        .align_items(AlignItems::Center),
                )
                .class(METER_TRACK)
                .child(
                    container()
                        .role(Role::GenericContainer)
                        .layout(LayoutStyle::block().height(METER_TRACK_HEIGHT))
                        .class(Radius::Md.atom())
                        .style_reactive(move || fill_atoms(value, max, warn)),
                ),
        );
        caller_style(node, self.style_accumulator)
    }
}

builder_impls!([] GaugeBuilder: paint(style_accumulator), widget, node);

fn fill_atoms(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> &'static [StyleAtom] {
    static CACHE: Keyed<(i16, bool)> = Keyed::new();
    let ratio = (value.get() / max).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let px = (ratio * TRACK_WIDTH).round() as i16;
    let is_warn = warn_threshold.is_some_and(|threshold| ratio < threshold);
    let tone = if is_warn {
        Tone::Warning.step(9)
    } else {
        Tone::Primary.step(8)
    };
    keyed(&CACHE, KEYED_CAP, (px, is_warn), || {
        vec![
            StyleAtom::Width(crate::style::atom::ATOM_PX, px),
            tone.background(),
        ]
    })
}
