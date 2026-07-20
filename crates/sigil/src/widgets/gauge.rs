//! Gauge widget — labeled HP/CT bar for battler HUDs.
//!
//! Reuses the [`progress`](super::progress) widget's track/fill shape
//! (a `surface-hover` track holding a reactive-width `accent` fill)
//! and adds two things a battler HUD needs on top: an optional visible
//! plus accessible label, and a `.warn(threshold)` tone shift. Once
//! `value / max` drops below `threshold` the fill flips from the
//! `accent` scale to the `warn-9` scale step so a low-HP/low-CT bar
//! reads as danger at a glance.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextContent, container, text};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::Signal;
use sigil_style::{Dimension, NodeStyleExt, Px, Style, StyleAccess, StyleAtom};

const TRACK_WIDTH: f32 = 160.0;
const TRACK_HEIGHT: f32 = 10.0;
const GAUGE_GAP: f32 = 8.0;

static TRACK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];

/// Cache key: fill width in px + whether the warn tone is active.
type FillAtomCache = HashMap<(i16, bool), &'static [StyleAtom]>;

/// Builder returned by [`gauge`].
pub struct GaugeBuilder {
    value: Signal<f32>,
    max: f32,
    label: Option<TextContent>,
    warn_threshold: Option<f32>,
    style_accumulator: Style,
}

/// Create a labeled gauge bar (HP, CT, and similar battler HUD stats).
///
/// - `value` — current amount, tracked reactively.
/// - default `max` is `1.0`; override with [`GaugeBuilder::max`].
#[must_use]
pub fn gauge(value: Signal<f32>) -> GaugeBuilder {
    GaugeBuilder::new(value)
}

impl GaugeBuilder {
    /// Create a gauge builder with default settings.
    #[must_use]
    pub fn new(value: Signal<f32>) -> Self {
        Self {
            value,
            max: 1.0,
            label: None,
            warn_threshold: None,
            style_accumulator: Style::new(),
        }
    }

    /// Set the gauge's maximum value (default `1.0`). Clamped above zero.
    #[must_use]
    pub const fn max(mut self, max: f32) -> Self {
        self.max = if max > f32::EPSILON {
            max
        } else {
            f32::EPSILON
        };
        self
    }

    /// Set the visible + accessible label text (e.g. `"HP"`, `"CT"`).
    #[must_use]
    pub fn label(mut self, label: impl Into<TextContent>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shift the fill to the warn tone once `value / max` drops below
    /// `threshold` (expected range `0.0..=1.0`).
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
                LayoutStyle::flex_row()
                    .gap(GAUGE_GAP)
                    .align_items(AlignItems::Center),
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

impl StyleAccess for GaugeBuilder {
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for GaugeBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn label_node(label: TextContent) -> impl IntoNode {
    text(label).class("text-on-surface")
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
        .class("rounded-md")
        .class(TRACK)
        .child(fill(value, max, warn_threshold))
}

fn track_layout() -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(TRACK_WIDTH, TRACK_HEIGHT)
        .align_items(AlignItems::Center)
}

/// The fill node's width AND tone are both driven from one reactive
/// closure so they land on the same `style_reactive` channel — a node
/// only carries one reactive atom slot (see [`fill_atoms`]).
fn fill(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> NodeBuilder {
    container()
        .role(Role::GenericContainer)
        .layout(LayoutStyle::block().height(TRACK_HEIGHT))
        .class("rounded-md")
        .style_reactive(move || fill_atoms(value, max, warn_threshold))
}

fn fill_atoms(value: Signal<f32>, max: f32, warn_threshold: Option<f32>) -> &'static [StyleAtom] {
    let ratio = (value.get() / max).clamp(0.0, 1.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
    let tone = if is_warn { "warn-9" } else { "accent" };
    vec![
        StyleAtom::Width(Dimension::Px(Px(px))),
        StyleAtom::BackgroundThemeRef(tone),
    ]
}
