use std::rc::Rc;

use sigil_events::prelude::{Key, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, TextBuilder, container, text};
use sigil_layout::prelude::{AlignItems, JustifyContent, LayoutBuilderExt, LayoutStyle};
use sigil_style::{ClassSource, NodeStyleExt, StateStyle, Style, StyleAtom, StyleVariant, Theme};

use crate::interaction::InteractionState;

use super::shared::{Activate, BuilderMutator, activate_key, activate_pointer, noop_activate};

/// Visual tone for [`button`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonTone {
    /// Neutral surface button.
    #[default]
    Neutral,
    /// Accent-coloured primary action.
    Primary,
}

/// Builder returned by [`button`].
pub struct ButtonBuilder {
    label: String,
    tone: ButtonTone,
    width: f32,
    height: f32,
    enabled: bool,
    on_activate: Option<Activate>,
    mutators: Vec<BuilderMutator>,
}

static PRIMARY_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static PRIMARY_REST: &[StyleAtom] = &[];
static PRIMARY_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static PRIMARY_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent-hover")];
static PRIMARY_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("accent-hover"),
    StyleAtom::Opacity(8_800),
];
static NEUTRAL_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static NEUTRAL_REST: &[StyleAtom] = &[];
static NEUTRAL_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static NEUTRAL_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static NEUTRAL_DOWN: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(8_800),
];
static DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

/// Create a button widget.
#[must_use]
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder::new(label)
}

impl ButtonBuilder {
    /// Create a button builder.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: ButtonTone::Neutral,
            width: 148.0,
            height: 44.0,
            enabled: true,
            on_activate: None,
            mutators: Vec::new(),
        }
    }

    /// Use the primary accent tone.
    #[must_use]
    pub const fn primary(mut self) -> Self {
        self.tone = ButtonTone::Primary;
        self
    }

    /// Use the neutral surface tone.
    #[must_use]
    pub const fn neutral(mut self) -> Self {
        self.tone = ButtonTone::Neutral;
        self
    }

    /// Set whether the button accepts focus and activation.
    #[must_use]
    pub const fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }

    /// Set a fixed logical-pixel width.
    #[must_use]
    pub const fn width_px(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set a fixed logical-pixel size.
    #[must_use]
    pub const fn fixed_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Run a callback when the button is activated by pointer or Enter.
    #[must_use]
    pub fn on_activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.on_activate = Some(Rc::new(activate));
        self
    }

    /// Add utility classes or atoms after the built-in button defaults.
    #[must_use]
    pub fn class(mut self, source: impl Into<ClassSource> + 'static) -> Self {
        let source = source.into();
        self.mutators.push(Box::new(move |node| node.class(source)));
        self
    }

    /// Add struct-built style after the built-in button defaults.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.mutators.push(Box::new(move |node| node.style(style)));
        self
    }

    /// Attach a scoped theme to the button root.
    #[must_use]
    pub fn theme(mut self, theme: Theme) -> Self {
        self.mutators.push(Box::new(move |node| node.theme(theme)));
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let interaction = InteractionState::new(self.enabled);
        let activate = self
            .on_activate
            .as_ref()
            .map_or_else(noop_activate, Rc::clone);
        let mut node = self.base_node(interaction, Rc::clone(&activate));
        for mutate in self.mutators {
            node = mutate(node);
        }
        node
    }

    fn base_node(&self, interaction: InteractionState, activate: Activate) -> NodeBuilder {
        let variant = button_variant(self.tone);
        let pointer_activate = Rc::clone(&activate);
        let key_activate = activate;
        let node = container()
            .role(Role::Button)
            .aria_label(self.label.clone())
            .layout(button_layout(self.width, self.height))
            .class("rounded-md")
            .class(variant.base())
            .style_reactive(move || variant.state(interaction.style_state()))
            .on_pointer_up(move |event| activate_pointer(event, interaction, &pointer_activate))
            .on_key(Key::Enter, move |event| {
                activate_key(event, interaction, &key_activate);
            })
            .child(self.label_node());
        interaction.install(node)
    }

    fn label_node(&self) -> TextBuilder {
        let width = (self.width - 32.0).max(1.0);
        text(self.label.clone())
            .layout(LayoutStyle::block().width_px(width))
            .class(button_text_class(self.tone))
    }
}

impl IntoNode for ButtonBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn button_layout(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle::flex_row()
        .fixed_size(width, height)
        .align_items(AlignItems::Center)
        .justify_content(JustifyContent::Center)
}

const fn button_variant(tone: ButtonTone) -> StyleVariant {
    match tone {
        ButtonTone::Primary => StyleVariant::new(
            PRIMARY_BASE,
            StateStyle::new(
                PRIMARY_REST,
                PRIMARY_FOCUS,
                PRIMARY_HOVER,
                PRIMARY_DOWN,
                DISABLED,
            ),
        ),
        ButtonTone::Neutral => StyleVariant::new(
            NEUTRAL_BASE,
            StateStyle::new(
                NEUTRAL_REST,
                NEUTRAL_FOCUS,
                NEUTRAL_HOVER,
                NEUTRAL_DOWN,
                DISABLED,
            ),
        ),
    }
}

const fn button_text_class(tone: ButtonTone) -> &'static str {
    match tone {
        ButtonTone::Primary => "text-on-accent",
        ButtonTone::Neutral => "text-on-surface",
    }
}
