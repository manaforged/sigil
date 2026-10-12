use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use crate::kernel::{
    AccessibleName, Cx, IntoNode, NodeBuilder, Role, TextContent, container, text,
};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutBuilderExt, LayoutStyle};
use crate::reactive::{Signal, derived};
use crate::style::{
    ControlSize, Metrics, NodeStyleExt, PaintTarget, Radius, Style, StyleAtom, StyleSlot, Token,
    Tone,
};

const TRACK_HEIGHT: f32 = 8.0;
const INDETERMINATE_RATIO: f32 = 0.30;

static TRACK: &[StyleAtom] = &[Radius::Md.atom(), StyleAtom::BgToneStep(4)];
static FILL: &[StyleAtom] = &[Radius::Md.atom(), Token::Accent.bg()];
static EMPTY: &[StyleAtom] = &[];

pub struct ProgressBuilder {
    value: Signal<f32>,
    max: f32,
    pub indeterminate: bool,
    tone: Tone,
    name: AccessibleName,
    caption: Option<TextContent>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
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
            tone: Tone::Primary,
            name: AccessibleName::from("Progress"),
            caption: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub const fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        self.name = name.into();
        self.caption = match &self.name {
            AccessibleName::None => None,
            AccessibleName::Static(value) => Some(TextContent::from(value.clone())),
            AccessibleName::Derived(value) => Some(TextContent::from(*value)),
        };
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let (value, max) = (self.value, self.max);
        let track_h = self.layout_overrides.height_hint().unwrap_or(TRACK_HEIGHT);
        let fill = if self.indeterminate {
            indeterminate_fill(value, track_h)
        } else {
            determinate_fill(value, max, track_h, self.tone)
        };

        let mut node = container()
            .role(Role::ProgressIndicator)
            .label(self.name)
            .value(value, 0.0, max)
            .layout(self.layout_overrides.apply(if self.caption.is_some() {
                LayoutStyle::flex_row()
                    .height(track_h)
                    .grow(1.0)
                    .width(0.0)
                    .align_items(AlignItems::Center)
            } else {
                LayoutStyle::flex_row()
                    .height(track_h)
                    .fill_width()
                    .align_items(AlignItems::Center)
            }))
            .class(TRACK)
            .child(fill);
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        match self.caption {
            Some(caption) => captioned_row(caption, node),
            None => node,
        }
    }
}

impl PaintTarget for ProgressBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ProgressBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for ProgressBuilder {
    fn build(self, cx: &mut Cx) -> crate::kernel::NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}

fn captioned_row(caption: TextContent, bar: NodeBuilder) -> NodeBuilder {
    let metrics = Metrics::current();
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(metrics.gap(ControlSize::Default))
                .align_items(AlignItems::Center),
        )
        .child(text(caption).nowrap())
        .child(bar)
}

fn determinate_fill(value: Signal<f32>, max: f32, track_h: f32, tone: Tone) -> NodeBuilder {
    let filled = derived(move || grow_units(value.get() / max));
    let empty = derived(move || GROW_SCALE - grow_units(value.get() / max));
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(track_h))
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .style_reactive(move || grow_atoms(filled.get()))
                .class(fill_atoms(tone))
                .tone(tone),
        )
        .child(empty_segment(empty, track_h))
}

fn empty_segment(units: Signal<u16>, track_h: f32) -> NodeBuilder {
    let (height, paint) = CellForm::new((track_h, EMPTY), (1.0, TRACK)).pick();
    container()
        .layout(LayoutStyle::block().height(height).width(0.0))
        .class(paint)
        .style_reactive(move || grow_atoms(units.get()))
}

fn fill_atoms(tone: Tone) -> [StyleAtom; 2] {
    let paint = match tone {
        Tone::Primary => Token::Accent.bg(),
        _ => tone.fill().bg(),
    };
    [Radius::Md.atom(), paint]
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
    static CACHE: OnceLock<Mutex<HashMap<u16, &'static [StyleAtom]>>> = OnceLock::new();
    let mut map = CACHE
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    map.entry(units).or_insert_with(|| -> &'static [StyleAtom] {
        Box::leak(Box::new([StyleAtom::FlexGrow(units)]))
    })
}

fn indeterminate_fill(phase: Signal<f32>, track_h: f32) -> NodeBuilder {
    let lead = derived(move || {
        let t = phase.get().clamp(0.0, 1.0);
        grow_units(t * (1.0 - INDETERMINATE_RATIO))
    });
    let trail = derived(move || {
        let t = phase.get().clamp(0.0, 1.0);
        let used = t * (1.0 - INDETERMINATE_RATIO) + INDETERMINATE_RATIO;
        GROW_SCALE - grow_units(used)
    });
    let spacer = move |units: Signal<u16>| empty_segment(units, track_h);
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(track_h))
        .child(spacer(lead))
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .style_reactive(|| grow_atoms(grow_units(INDETERMINATE_RATIO)))
                .class(FILL),
        )
        .child(spacer(trail))
}
