use crate::layout::Space;

use super::layout_ext::LayoutOverrides;
use super::shared::{METER_TRACK, METER_TRACK_HEIGHT, caller_style};
use crate::kernel::{AccessibleName, NodeBuilder, Role, TextContent, container, text};
use crate::layout::snap::CellForm;
use crate::layout::{AlignItems, LayoutStyle};
use crate::reactive::{Reactive, Signal, memo};
use crate::style::intern::{KEYED_CAP, Keyed, keyed};
use crate::style::strings::StringKey;
use crate::style::theme::ambient::theme_label;
use crate::style::{ControlSize, Metrics, Radius, Style, StyleAtom, Token, Tone};
use crate::widgets::Layout;

const INDETERMINATE_RATIO: f32 = 0.30;
const CELL_RULE_THICKNESS: f32 = 1.0;

static FILL: &[StyleAtom] = &[Radius::Md.atom(), Token::Accent.background()];
static EMPTY: &[StyleAtom] = &[];

/// Builder of a progress bar.
#[derive(Debug)]
#[must_use]
pub struct ProgressBuilder {
    value: Signal<f32>,
    max: f32,
    indeterminate: bool,
    tone: Tone,
    name: Option<AccessibleName>,
    caption: Option<TextContent>,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// A progress bar.
pub fn progress(value: impl Reactive<f32>, max: f32) -> ProgressBuilder {
    ProgressBuilder::new(value.into_signal(), max)
}

impl ProgressBuilder {
    /// Creates a bar that shows `value` as a fraction of `max`.
    pub(crate) const fn new(value: Signal<f32>, max: f32) -> Self {
        Self {
            value,
            max: max.max(f32::EPSILON),
            indeterminate: false,
            tone: Tone::Primary,
            name: None,
            caption: None,
            style_accumulator: Style::new(),
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Draws a fixed segment that moves along the track as `value` goes from 0.0 to 1.0.
    pub const fn indeterminate(mut self) -> Self {
        self.indeterminate = true;
        self
    }

    /// Sets the fill color of a determinate bar; the default is `Tone::Primary`.
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Sets the accessible name and shows it as a caption before the bar.
    pub fn label(mut self, name: impl Into<AccessibleName>) -> Self {
        let name = name.into();
        self.caption = match &name {
            AccessibleName::None => None,
            AccessibleName::Static(value) => Some(TextContent::from(value.clone())),
            AccessibleName::Derived(value) => Some(TextContent::from(*value)),
        };
        self.name = Some(name);
        self
    }

    fn root(self) -> NodeBuilder {
        let (value, max) = (self.value, self.max);
        let track_h = self
            .layout_overrides
            .height_hint()
            .unwrap_or(METER_TRACK_HEIGHT);
        let fill = if self.indeterminate {
            indeterminate_fill(value, track_h)
        } else {
            determinate_fill(value, max, track_h, self.tone)
        };

        let name = self
            .name
            .unwrap_or_else(|| AccessibleName::Derived(theme_label(StringKey::Progress)));
        let node = container()
            .role(Role::ProgressIndicator)
            .label(name)
            .value(value, 0.0, max)
            .layout(if self.caption.is_some() {
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
            })
            .class(METER_TRACK)
            .child(fill);
        let node = caller_style(node, self.style_accumulator);
        match self.caption {
            Some(caption) => captioned_row(caption, node),
            None => node,
        }
    }
}

builder_impls!([] ProgressBuilder: paint(style_accumulator), widget, node);

fn captioned_row(caption: TextContent, bar: NodeBuilder) -> NodeBuilder {
    let metrics = Metrics::current();
    container()
        .layout(
            LayoutStyle::flex_row()
                .fill_width()
                .gap(Space::px(metrics.control_gap(ControlSize::Default)))
                .align_items(AlignItems::Center),
        )
        .child(text(caption).nowrap())
        .child(bar)
}

fn determinate_fill(value: Signal<f32>, max: f32, track_h: f32, tone: Tone) -> NodeBuilder {
    let filled = memo(move || grow_units(value.get() / max));
    let empty = memo(move || GROW_SCALE - grow_units(value.get() / max));
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(track_h))
        .child(
            container()
                .layout(LayoutStyle::block().height(track_h).width(0.0))
                .style_reactive(move || fill_style(filled.get(), tone)),
        )
        .child(empty_segment(empty, track_h))
}

fn empty_segment(units: Signal<u16>, track_h: f32) -> NodeBuilder {
    let (height, paint) =
        CellForm::new((track_h, EMPTY), (CELL_RULE_THICKNESS, METER_TRACK)).pick();
    container()
        .layout(LayoutStyle::block().height(height).width(0.0))
        .class(paint)
        .style_reactive(move || grow_atoms(units.get()))
}

const fn fill_atoms(tone: Tone) -> [StyleAtom; 2] {
    let paint = match tone {
        Tone::Primary => Token::Accent.background(),
        _ => tone.fill().background(),
    };
    [Radius::Md.atom(), paint]
}

fn fill_style(units: u16, tone: Tone) -> &'static [StyleAtom] {
    static CACHE: Keyed<(u16, Tone)> = Keyed::new();
    if units == 0 {
        return grow_atoms(0);
    }
    keyed(&CACHE, KEYED_CAP, (units, tone), || {
        let [radius, paint] = fill_atoms(tone);
        vec![StyleAtom::FlexGrow(units), radius, paint]
    })
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
    static CACHE: Keyed<u16> = Keyed::new();
    keyed(&CACHE, KEYED_CAP, units, || {
        vec![StyleAtom::FlexGrow(units)]
    })
}

fn indeterminate_fill(phase: Signal<f32>, track_h: f32) -> NodeBuilder {
    let lead = memo(move || {
        let t = phase.get().clamp(0.0, 1.0);
        grow_units(t * (1.0 - INDETERMINATE_RATIO))
    });
    let trail = memo(move || {
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
