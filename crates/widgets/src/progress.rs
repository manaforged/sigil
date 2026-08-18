//! Progress widget — determinate or indeterminate progress bar.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use sigil_reactive::{Signal, derived_eq};
use sigil_style::{NodeStyleExt, Style, StyleAccess, StyleAtom, Tone};

const TRACK_HEIGHT: f32 = 8.0;
const INDETERMINATE_RATIO: f32 = 0.30;

static TRACK: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border-strong")];
static FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-8")];

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
    track_width: Option<f32>,
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
            track_width: None,
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

    /// Override the track width in logical pixels.
    ///
    /// The default is a fixed 240px, which is what makes a bare progress
    /// bar ignore the width of whatever it is placed in. The fill and the
    /// indeterminate sweep are both measured against this width, so they
    /// stay proportional at any value set here.
    #[must_use]
    pub const fn width(mut self, px: f32) -> Self {
        self.track_width = Some(px);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let value = self.value;
        let max = self.max;
        let track_h = self.track_height;
        let track_w = self.track_width;
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
            .layout(track_layout(track_h, track_w))
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
    fn style_mut(&mut self) -> &mut Style {
        &mut self.style_accumulator
    }
}

impl IntoNode for ProgressBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn track_layout(track_h: f32, track_w: Option<f32>) -> LayoutStyle {
    // Fill the parent by default; `.width()` pins an explicit extent.
    let base = LayoutStyle::flex_row().height(track_h);
    let base = match track_w {
        Some(w) => base.width(w),
        None => base.fill_width(),
    };
    base.align_items(AlignItems::Center)
}

/// Determinate fill as a PROPORTION of the track.
///
/// The fill and a trailing spacer split the track by flex-grow, so the
/// bar stays correct at any track width — including a track that fills
/// its parent, where no pixel width is known at build time. (Expressing
/// this as a percentage is not possible: the style layer's `Dimension`
/// has only `Px`/`Full`/`Auto`.)
fn determinate_fill(value: Signal<f32>, max: f32, track_h: f32) -> NodeBuilder {
    let filled = derived_eq(move || grow_units(value.get() / max));
    let empty = derived_eq(move || GROW_SCALE - grow_units(value.get() / max));
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(track_h))
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .class("rounded-md")
                .style_reactive(move || grow_atoms(filled.get()))
                .class(FILL)
                .tone(Tone::Primary),
        )
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .style_reactive(move || grow_atoms(empty.get())),
        )
}

/// `FlexGrow` is a u16 scaled by 1000 in the style->layout adapter.
const GROW_SCALE: u16 = 1_000;

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn grow_units(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * f32::from(GROW_SCALE)).round() as u16
}

fn grow_atoms(units: u16) -> &'static [StyleAtom] {
    #[allow(clippy::cast_possible_wrap)]
    cached_atoms(units as i16, || vec![StyleAtom::FlexGrow(units)])
}

/// Indeterminate sweep, also proportional: a leading spacer, the bar at a
/// fixed fraction of the track, then a trailing spacer. Same reason as
/// [`determinate_fill`] — no pixel width is known when the track fills its
/// parent.
fn indeterminate_fill(phase: Signal<f32>, track_h: f32) -> NodeBuilder {
    let lead = derived_eq(move || {
        let t = phase.get().clamp(0.0, 1.0);
        grow_units(t * (1.0 - INDETERMINATE_RATIO))
    });
    let trail = derived_eq(move || {
        let t = phase.get().clamp(0.0, 1.0);
        let used = t * (1.0 - INDETERMINATE_RATIO) + INDETERMINATE_RATIO;
        GROW_SCALE - grow_units(used)
    });
    let spacer = move |units: sigil_reactive::Signal<u16>| {
        container()
            .layout(LayoutStyle::block().height(track_h).width(0.0))
            .style_reactive(move || grow_atoms(units.get()))
    };
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(track_h))
        .child(spacer(lead))
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .class("rounded-md")
                .style_reactive(|| grow_atoms(grow_units(INDETERMINATE_RATIO)))
                .class(FILL),
        )
        .child(spacer(trail))
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
