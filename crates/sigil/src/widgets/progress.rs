use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use crate::layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived_eq};
use crate::style::{NodeStyleExt, Style, StyleAccess, StyleAtom, Tone};

const TRACK_HEIGHT: f32 = 8.0;
const INDETERMINATE_RATIO: f32 = 0.30;

static TRACK: &[StyleAtom] = &[StyleAtom::BgToneStep(4)];
static FILL: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

pub struct ProgressBuilder {
    value: Signal<f32>,
    max: f32,
    pub indeterminate: bool,
    track_height: f32,
    track_width: Option<f32>,
    style_accumulator: Style,
}

#[must_use]
pub fn progress(value: Signal<f32>, max: f32) -> ProgressBuilder {
    ProgressBuilder::new(value, max)
}

impl ProgressBuilder {
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

    #[must_use]
    pub const fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }

    #[must_use]
    pub const fn height(mut self, px: f32) -> Self {
        self.track_height = px;
        self
    }

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
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn track_layout(track_h: f32, track_w: Option<f32>) -> LayoutStyle {
    let base = LayoutStyle::flex_row().height(track_h);
    let base = match track_w {
        Some(w) => base.width(w),
        None => base.fill_width(),
    };
    base.align_items(AlignItems::Center)
}

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

const GROW_SCALE: u16 = 1_000;

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn grow_units(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * f32::from(GROW_SCALE)).round() as u16
}

fn grow_atoms(units: u16) -> &'static [StyleAtom] {
    #[allow(
        clippy::cast_possible_wrap,
        reason = "values are bounded well inside the target range"
    )]
    cached_atoms(units as i16, || vec![StyleAtom::FlexGrow(units)])
}

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
    let spacer = move |units: crate::reactive::Signal<u16>| {
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
