use std::rc::Rc;

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{NodeBuilder, Role};
use sigil_layout::prelude::LayoutBuilderExt;
use sigil_reactive::Signal;
use sigil_style::{NodeStyleExt, StyleAtom, Tone};

use crate::interaction::InteractionState;

use super::super::shared::{Activate, activate_key_reactive, activate_pointer_reactive};
use super::{CHIP_PADDING_X, CHIP_PADDING_Y, chip_class};

pub struct ChipBuilder {
    label: sigil_kernel::TextContent,
    tone: Tone,
    tone_signal: Option<Signal<Tone>>,
    activate: Option<Activate>,
    pill: bool,
}

impl ChipBuilder {
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
    pub fn layout(self, layout: sigil_layout::prelude::LayoutStyle) -> NodeBuilder {
        self.into_node_builder().layout(layout)
    }

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
    match sigil_style::parse_class_string(class) {
        Ok(atoms) => Box::leak(atoms.into_vec().into_boxed_slice()),
        Err(_) => &[],
    }
}
