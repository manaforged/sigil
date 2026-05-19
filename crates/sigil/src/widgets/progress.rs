//! Progress widget — determinate or indeterminate progress bar.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{Dimension, NodeStyleExt, Px, StateHost, Style, StyleAccess, StyleAtom};

const TRACK_WIDTH: f32 = 240.0;
const TRACK_HEIGHT: f32 = 8.0;
const INDETERMINATE_RATIO: f32 = 0.30;

static TRACK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

/// Builder returned by [`progress`].
pub struct ProgressBuilder {
    value: Signal<f32>,
    max: f32,
    /// Switch to indeterminate animation mode.
    ///
    /// In indeterminate mode `value` is treated as an animation phase
    /// (0.0–1.0). Drive it with a reactive signal that advances each frame.
    pub indeterminate: bool,
    /// Override the track height in logical pixels (default: 8.0).
    track_height: f32,
    style_accumulator: Style,
}

/// Create a progress bar widget.
///
/// - `value` — current progress value (0.0..=`max`), or animation phase
///   (0.0–1.0) in indeterminate mode.
/// - `max`   — maximum value (must be > 0).
#[must_use]
pub fn progress(value: Signal<f32>, max: f32) -> ProgressBuilder {
    ProgressBuilder::new(value, max)
}

impl ProgressBuilder {
    /// Create a progress builder with default settings.
    #[must_use]
    pub fn new(value: Signal<f32>, max: f32) -> Self {
        Self {
            value,
            max: max.max(f32::EPSILON),
            indeterminate: false,
            track_height: TRACK_HEIGHT,
            style_accumulator: Style::new(),
        }
    }

    /// Enable indeterminate mode (looping bar animation driven by `value`).
    #[must_use]
    pub const fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }

    /// Override the track height in logical pixels.
    ///
    /// Useful when rendering into a golden harness that requires a minimum
    /// canvas dimension (e.g. ≥32 px) while still showing the bar at
    /// ≥70% frame coverage.
    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.track_height = px;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let value = self.value;
        let max = self.max;
        let track_h = self.track_height;
        let is_indeterminate = self.indeterminate;
        let style_accumulator = self.style_accumulator;

        let fill = if is_indeterminate {
            indeterminate_fill(value, track_h)
        } else {
            determinate_fill(value, max, track_h)
        };

        let mut node = container()
            .role(Role::ProgressIndicator)
            .value(value, 0.0, max)
            .layout(track_layout(track_h))
            .class("rounded-md")
            .class(TRACK)
            .child(fill);
        // Styled accumulator installed AFTER the built-in widget atoms
        // so the trait surface layers ON TOP of the defaults (additive
        // composition per DR-049 §10). Order matches batch 1.
        if !style_accumulator.is_empty() {
            node = node.style(style_accumulator);
        }
        node
    }
}

impl StyleAccess for ProgressBuilder {
    fn style(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl StateHost for ProgressBuilder {}

impl IntoNode for ProgressBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn track_layout(track_h: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(TRACK_WIDTH, track_h)
        .align_items(AlignItems::Center)
}

fn determinate_fill(value: Signal<f32>, max: f32, track_h: f32) -> NodeBuilder {
    let fill_px = derived_eq(move || {
        let ratio = (value.get() / max).clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = (ratio * TRACK_WIDTH).round() as i16;
        px
    });

    container()
        .layout(LayoutStyle::block().height_px(track_h))
        .class("rounded-md")
        .style_reactive(move || width_atoms_px(fill_px.get()))
        .class(FILL)
}

fn indeterminate_fill(phase: Signal<f32>, track_h: f32) -> NodeBuilder {
    let offset_px = derived_eq(move || {
        let t = phase.get().clamp(0.0, 1.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = (t * TRACK_WIDTH * (1.0 - INDETERMINATE_RATIO)).round() as i16;
        px
    });

    let bar_width_px = (TRACK_WIDTH * INDETERMINATE_RATIO).round();
    container()
        .layout(
            LayoutStyle::block()
                .width_px(bar_width_px)
                .height_px(track_h),
        )
        .class("rounded-md")
        .style_reactive(move || left_margin_atoms_px(offset_px.get()))
        .class(FILL)
}

fn width_atoms_px(px: i16) -> &'static [StyleAtom] {
    cached_atoms(px, || vec![StyleAtom::Width(Dimension::Px(Px(px)))])
}

fn left_margin_atoms_px(px: i16) -> &'static [StyleAtom] {
    cached_atoms(px.wrapping_add(i16::MAX / 2), || {
        vec![StyleAtom::MarginLeft(Px(px))]
    })
}

fn cached_atoms(key: i16, build: impl FnOnce() -> Vec<StyleAtom>) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cache.lock() else {
        return Box::leak(build().into_boxed_slice());
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked: &'static [StyleAtom] = Box::leak(build().into_boxed_slice());
    map.insert(key, leaked);
    leaked
}
