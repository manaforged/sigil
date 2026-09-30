use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{Dimension, PaintTarget, Px, Radius, Style, StyleAtom, StyleSlot, Token, space};

const TRACK_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 8.0;
const GAUGE_GAP: f32 = space(2);

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
        let value = self.value;
        let max = self.max;
        let warn_threshold = self.warn_threshold;
        let style_accumulator = self.style_accumulator;

        let mut node = container()
            .role(Role::ProgressIndicator)
            .label(accessible_label(self.label.as_ref()))
            .value(value, 0.0, max)
            .layout(
                self.layout_overrides.apply(
                    LayoutStyle::flex_row()
                        .gap(GAUGE_GAP)
                        .align_items(AlignItems::Center),
                ),
            );
        if let Some(label) = self.label {
            node = node.child(label_node(label));
        }
        node = node.child(track(value, max, warn_threshold));
        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
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
        self.into_node_builder().build(cx)
    }
}

fn label_node(label: TextContent) -> impl IntoNode {
    text(label).class(Token::OnSurface.text())
}

fn accessible_label(label: Option<&TextContent>) -> String {
    label
        .and_then(TextContent::as_static_str)
        .map_or_else(|| String::from("Gauge"), ToOwned::to_owned)
}

fn track(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(track_layout())
        .class(TRACK)
        .child(fill(value, max, warn_threshold))
}

fn track_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(TRACK_WIDTH, TRACK_HEIGHT)
        .align_items(AlignItems::Center)
}

fn fill(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().height(TRACK_HEIGHT))
        .class(Radius::Md.atom())
        .style_reactive(move || fill_atoms(value, max, warn_threshold))
}

fn fill_atoms(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> &'static [StyleAtom] {
    let ratio = (value.get() / max).clamp(0.0, 1.0);
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "values are bounded to the target range before the cast"
    )]
    let px = (ratio * TRACK_WIDTH).round() as i16;
    let is_warn = warn_threshold.is_some_and(|threshold| ratio < threshold);
    cached_fill_atoms(px, is_warn)
}

fn cached_fill_atoms(px: i16, is_warn: bool) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<FillAtomCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (px, is_warn);
    let Ok(mut map) = cache.lock() else {
        return Box::leak(build_fill_atoms(px, is_warn).into_boxed_slice());
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked: &'static [StyleAtom] = Box::leak(build_fill_atoms(px, is_warn).into_boxed_slice());
    map.insert(key, leaked);
    leaked
}

fn build_fill_atoms(px: i16, is_warn: bool) -> Vec<StyleAtom> {
    let tone = if is_warn {
        Token::Warn9
    } else {
        Token::Accent8
    };
    vec![StyleAtom::Width(Dimension::Px(Px(px))), tone.bg()]
}
