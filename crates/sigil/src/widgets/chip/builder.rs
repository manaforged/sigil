//! `ChipBuilder` — fluent surface for the [`chip`](crate::widgets::chip) badge widget.

use sigil_kernel::NodeBuilder;
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::Signal;
use sigil_style::{NodeStyleExt, StyleAtom};

use super::{CHIP_PADDING_X, CHIP_PADDING_Y, ChipTone};

/// Builder returned by [`chip()`](crate::widgets::chip). Static tone by default;
/// `.tone_signal(...)` retints reactively without a remount (the
/// widget-playground FRICTION ask).
pub struct ChipBuilder {
    label: sigil_kernel::TextContent,
    tone: ChipTone,
    tone_signal: Option<Signal<ChipTone>>,
}

impl ChipBuilder {
    /// Construct a chip with a static tone.
    #[must_use]
    pub(crate) fn new(label: impl Into<sigil_kernel::TextContent>, tone: ChipTone) -> Self {
        Self {
            label: label.into(),
            tone,
            tone_signal: None,
        }
    }

    /// Retint the chip (static override of the constructor tone).
    #[must_use]
    pub const fn tone(mut self, tone: ChipTone) -> Self {
        self.tone = tone;
        self
    }

    /// Drive the tone from a signal — flips background/foreground live
    /// with no `children_reactive` remount.
    #[must_use]
    pub const fn tone_signal(mut self, tone: Signal<ChipTone>) -> Self {
        self.tone_signal = Some(tone);
        self
    }

    /// Set the chip's layout — mirrors the `NodeBuilder::layout`
    /// chaining surface the pre-builder `chip()` returned, so call
    /// sites like the log-tailer example keep their shape.
    #[must_use]
    pub fn layout(self, layout: sigil_layout::prelude::LayoutStyle) -> NodeBuilder {
        self.into_node_builder().layout(layout)
    }

    /// Consume into a [`NodeBuilder`].
    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let node = sigil_kernel::container()
            .role(sigil_kernel::Role::Label)
            .layout(
                sigil_layout::prelude::LayoutStyle::flex_row()
                    .padding_x(CHIP_PADDING_X)
                    .padding_y(CHIP_PADDING_Y),
            )
            .class("rounded-sm text-xs")
            .class(self.tone.class())
            .child(sigil_kernel::text(self.label));
        match self.tone_signal {
            Some(tone) => node.style_reactive(move || tone_atoms(tone.get())),
            None => node,
        }
    }
}

impl sigil_kernel::IntoNode for ChipBuilder {
    fn build(self, cx: &mut sigil_kernel::Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

/// Pre-parsed atom slices for each tone, built once on first use.
fn tone_atoms(tone: ChipTone) -> &'static [StyleAtom] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[&'static [StyleAtom]; 5]> = OnceLock::new();
    let [neutral, info, success, warn, error] = TABLE.get_or_init(|| {
        [
            parse_and_leak("bg-gray-3 text-gray-12"),
            parse_and_leak("bg-info-3 text-info-12"),
            parse_and_leak("bg-success-3 text-success-12"),
            parse_and_leak("bg-warn-3 text-warn-12"),
            parse_and_leak("bg-error-3 text-error-11"),
        ]
    });
    match tone {
        ChipTone::Neutral => neutral,
        ChipTone::Info => info,
        ChipTone::Success => success,
        ChipTone::Warn => warn,
        ChipTone::Error => error,
    }
}

/// Parse a known-good utility-class fragment into a leaked static slice.
/// Falls back to an empty slice (unreachable for the tone table's
/// compile-known tokens).
fn parse_and_leak(class: &str) -> &'static [StyleAtom] {
    match sigil_style::parse_class_string(class) {
        Ok(atoms) => Box::leak(atoms.into_vec().into_boxed_slice()),
        Err(_) => &[],
    }
}
