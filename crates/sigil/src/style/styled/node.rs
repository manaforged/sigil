use crate::kernel::NodeBuilder;
use crate::style::{ClassSource, StateStyle, Style, StyleVariant};
use crate::widgets::interaction::InteractionState;

pub(crate) fn attach_style(builder: NodeBuilder, style: Style) -> NodeBuilder {
    if style.is_empty() {
        return builder;
    }
    let mut style = style;
    let overlay = style.take_state_overlay();
    let builder = if overlay.is_empty() {
        builder
    } else {
        let interaction = InteractionState::new(true);
        let builder = builder.stateful_style(
            StyleVariant::new(&[], StateStyle::EMPTY),
            overlay,
            move || interaction.style_state(),
        );
        interaction.observe_style(builder)
    };
    let base_atoms = style.into_atoms();
    if base_atoms.is_empty() {
        return builder;
    }
    builder.class(ClassSource::Owned(Box::new(base_atoms)))
}
