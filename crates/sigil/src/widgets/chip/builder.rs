use std::rc::Rc;

use crate::events::prelude::{Key, NodeEventsExt};
use crate::kernel::{NodeBuilder, Role};
use crate::layout::prelude::LayoutBuilderExt;
use crate::reactive::Signal;
use crate::style::{NodeStyleExt, StyleAtom, Tone};

use crate::widgets::interaction::InteractionState;

use super::super::shared::{Activate, activate_key_reactive, activate_pointer_reactive};
use super::{CHIP_PADDING_X, CHIP_PADDING_Y, chip_class};

pub struct ChipBuilder {
    label: crate::kernel::TextContent,
    tone: Tone,
    tone_signal: Option<Signal<Tone>>,
    activate: Option<Activate>,
    pill: bool,
}

impl ChipBuilder {
    #[must_use]
    pub(crate) fn new(label: impl Into<crate::kernel::TextContent>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
            tone_signal: None,
            activate: None,
            pill: false,
        }
    }

    #[must_use]
    pub const fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    #[must_use]
    pub const fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    #[must_use]
    pub const fn tone_signal(mut self, tone: Signal<Tone>) -> Self {
        self.tone_signal = Some(tone);
        self
    }

    #[must_use]
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    #[must_use]
    pub fn layout(self, layout: crate::layout::prelude::LayoutStyle) -> NodeBuilder {
        self.into_node_builder().layout(layout)
    }

    #[must_use]
    pub fn into_node_builder(self) -> NodeBuilder {
        let activate = self.activate;
        let mut node = crate::kernel::container()
            .role(if activate.is_some() {
                Role::Button
            } else {
                Role::Label
            })
            .layout(
                crate::layout::prelude::LayoutStyle::flex_row()
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
            .child(crate::kernel::text(self.label).nowrap());
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

impl crate::kernel::IntoNode for ChipBuilder {
    fn build(self, cx: &mut crate::kernel::Cx) -> crate::kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

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

fn parse_and_leak(class: &str) -> &'static [StyleAtom] {
    match crate::style::parse_class_string(class) {
        Ok(atoms) => Box::leak(atoms.into_vec().into_boxed_slice()),
        Err(_) => &[],
    }
}
