//! `ChipBuilder` — fluent surface for the [`chip`](crate::widgets::chip) badge widget.

use std::rc::Rc;

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role};
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::Signal;
use sigil_style::{NodeStyleExt, StyleAtom, Tone};

use crate::interaction::InteractionState;

use super::super::shared::{Activate, activate_key_reactive, activate_pointer_reactive};
use super::{CHIP_PADDING_X, CHIP_PADDING_Y, chip_class};

/// Builder returned by [`chip()`](crate::widgets::chip). Static tone by default;
/// `.tone_signal(...)` retints reactively without a remount (the
/// widget-playground FRICTION ask).
pub struct ChipBuilder {
    label: sigil_kernel::TextContent,
    tone: Tone,
    tone_signal: Option<Signal<Tone>>,
    activate: Option<Activate>,
    pill: bool,
}

impl ChipBuilder {
    /// Construct a chip with a static tone.
    #[must_use]
    pub(crate) fn new(label: impl Into<sigil_kernel::TextContent>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
            tone_signal: None,
            activate: None,
            pill: false,
        }
    }

    /// Round the chip into a badge (full pill).
    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    /// Retint the chip (static override of the constructor tone).
    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Drive the tone from a signal — flips background/foreground live
    /// with no `children_reactive` remount.
    #[must_use]
    pub const fn tone_signal(mut self, tone: Signal<Tone>) -> Self {
        self.tone_signal = Some(tone);
        self
    }

    /// Run a callback when the chip is activated by pointer or Enter.
    ///
    /// Makes the chip a button (`Role::Button`, focusable, hittable).
    /// Without this the chip stays a label.
    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
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
        let activate = self.activate;
        let mut node = sigil_kernel::container()
            .role(if activate.is_some() {
                Role::Button
            } else {
                Role::Label
            })
            .layout(
                sigil_layout::prelude::LayoutStyle::flex_row()
                    .padding_x(CHIP_PADDING_X)
                    .padding_y(CHIP_PADDING_Y)
                    .shrink(0.0),
            )
            .class(if self.pill {
                "rounded-full text-xs"
            } else {
                "rounded-sm text-xs"
            })
            .class(chip_class(self.tone))
            .child(sigil_kernel::text(self.label));
        if let Some(activate) = activate {
            let interaction = InteractionState::new(true);
            let pointer_activate = Rc::clone(&activate);
            node = node
                .pointer_up(move |event| {
                    activate_pointer_reactive(event, interaction, None, &pointer_activate);
                })
                .key(Key::Enter, move |event| {
                    activate_key_reactive(event, interaction, None, &activate);
                });
            node = interaction.install(node);
        }
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
fn tone_atoms(tone: Tone) -> &'static [StyleAtom] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[&'static [StyleAtom]; 6]> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        [
            parse_and_leak(chip_class(Tone::Neutral)),
            parse_and_leak(chip_class(Tone::Primary)),
            parse_and_leak(chip_class(Tone::Info)),
            parse_and_leak(chip_class(Tone::Success)),
            parse_and_leak(chip_class(Tone::Warn)),
            parse_and_leak(chip_class(Tone::Error)),
        ]
    });
    match tone {
        Tone::Neutral => table[0],
        Tone::Primary => table[1],
        Tone::Info => table[2],
        Tone::Success => table[3],
        Tone::Warn => table[4],
        Tone::Error => table[5],
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
