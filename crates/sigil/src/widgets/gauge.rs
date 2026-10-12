use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{Dimension, PaintTarget, Px, Radius, Style, StyleAtom, StyleSlot, Token, space};

const TRACK_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 8.0;

static TRACK: &[StyleAtom] = &[Radius::Md.atom(), StyleAtom::BgToneStep(4)];

type FillAtomCache = HashMap<(i16, bool), &'static [StyleAtom]>;

pub struct GaugeBuilder {
    value: Signal<f32>,
    max: f32,
    label: Option<TextContent>,
    warn_threshold: Option<f32>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn gauge(value: Signal<f32>) -> GaugeBuilder {
    GaugeBuilder::new(value)
}

impl GaugeBuilder {
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

    #[must_use]
    pub const fn max(mut self, max: f32) -> Self {
        self.max = if max > f32::EPSILON {
            max
        } else {
            f32::EPSILON
        };
        self
    }

    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub const fn warn(mut self, threshold: f32) -> Self {
        self.warn_threshold = Some(threshold);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (value, max, warn) = (self.value, self.max, self.warn_threshold);
        let name = self.label.as_ref().and_then(TextContent::as_static_str);
        let mut node = container()
            .role(Role::ProgressIndicator)
            .label(name.map_or_else(|| String::from("Gauge"), ToOwned::to_owned))
            .value(value, 0.0, max)
            .layout(
                self.layout_overrides.apply(
                    LayoutStyle::flex_row()
                        .gap(space(2))
                        .align_items(AlignItems::Center),
                ),
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

impl PaintTarget for GaugeBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for GaugeBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for GaugeBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

fn fill_atoms(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> &'static [StyleAtom] {
    static CACHE: LazyLock<Mutex<FillAtomCache>> = LazyLock::new(Mutex::default);
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
            StyleAtom::Width(Dimension::Px(Px(px))),
            tone.bg(),
        ]))
    };
    let Ok(mut map) = CACHE.lock() else {
        return build();
    };
    map.entry((px, is_warn)).or_insert_with(build)
}
