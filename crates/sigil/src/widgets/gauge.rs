use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::layout_ext::LayoutOverrides;
use crate::kernel::{NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{Radius, Style, StyleAtom, Token, space};
use crate::widgets::Widget;

const TRACK_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 8.0;
const GAUGE_GAP: f32 = space(2);

static TRACK: &[StyleAtom] = &[Radius::Md.atom(), StyleAtom::BgToneStep(4)];

type GaugeAtoms = HashMap<(i16, bool), &'static [StyleAtom]>;

/// Builder of a gauge.
pub struct GaugeBuilder {
    value: Signal<f32>,
    max: f32,
    label: Option<TextContent>,
    warn_threshold: Option<f32>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A labelled meter bar that shows a value against a maximum.
#[must_use]
pub fn gauge(value: Signal<f32>) -> GaugeBuilder {
    GaugeBuilder::new(value)
}

impl GaugeBuilder {
    /// Creates a gauge builder.
    #[must_use]
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            max: 1.0,
            label: None,
            warn_threshold: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Sets the max.
    #[must_use]
    pub const fn max(mut self, max: f32) -> Self {
        self.max = if max > f32::EPSILON {
            max
        } else {
            f32::EPSILON
        };
        self
    }

    /// Sets the label.
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the warn.
    #[must_use]
    pub const fn warn(mut self, threshold: f32) -> Self {
        self.warn_threshold = Some(threshold);
        self
    }

    fn root(self) -> NodeBuilder {
        let (value, max, warn) = (self.value, self.max, self.warn_threshold);
        let name = self.label.as_ref().and_then(TextContent::as_static_str);
        let mut node = container()
            .role(Role::ProgressIndicator)
            .label(name.map_or_else(|| String::from("Gauge"), ToOwned::to_owned))
            .value(value, 0.0, max)
            .layout(
                LayoutStyle::flex_row()
                    .gap(GAUGE_GAP)
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
                        .fixed_size(TRACK_WIDTH, TRACK_HEIGHT)
                        .align_items(AlignItems::Center),
                )
                .class(TRACK)
                .child(
                    container()
                        .role(Role::GenericContainer)
                        .layout(LayoutStyle::block().height(TRACK_HEIGHT))
                        .class(Radius::Md.atom())
                        .style_reactive(move || fill_atoms(value, max, warn)),
                ),
        );
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        node
    }
}

builder_impls!([] GaugeBuilder: paint(style_accumulator), widget, node);

fn fill_atoms(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> &'static [StyleAtom] {
    static CACHE: LazyLock<Mutex<GaugeAtoms>> = LazyLock::new(Mutex::default);
    let ratio = (value.get() / max).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let px = (ratio * TRACK_WIDTH).round() as i16;
    let is_warn = warn_threshold.is_some_and(|threshold| ratio < threshold);
    let tone = if is_warn {
        Token::Warn9
    } else {
        Token::Accent8
    };
    let build = || -> &'static [StyleAtom] {
        Box::leak(Box::new([
            StyleAtom::Width(crate::style::atom::ATOM_PX, px),
            tone.bg(),
        ]))
    };
    let Ok(mut map) = CACHE.lock() else {
        return build();
    };
    map.entry((px, is_warn)).or_insert_with(build)
}
