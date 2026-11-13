use crate::clipboard::clipboard;
use crate::geometry::Rect;
use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, signal};
use crate::style::Style;

use crate::widgets::interaction::InteractionState;
use crate::widgets::shared::{TextInputProps, caller_style};

use super::handlers::{TextEditRuntime, TextState, attach_key_handlers, noop_change};
use super::ime::attach_ime_handlers;
use super::render::{build_base_node, content_padding, edit_layout};

/// The key that runs `TextFieldBuilder::submit` in multiline mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubmitKey {
    /// Ctrl+Enter submits and Enter inserts a newline.
    #[default]
    CtrlEnter,
    /// Enter submits; Shift+Enter, Alt+Enter, and Ctrl+J insert a newline.
    Enter,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::widgets) struct MultilineConfig {
    pub(in crate::widgets) submit_key: SubmitKey,
    pub(in crate::widgets) content_padding: Option<(f32, f32)>,
    pub(in crate::widgets) chrome: bool,
}

impl Default for MultilineConfig {
    fn default() -> Self {
        Self {
            submit_key: SubmitKey::CtrlEnter,
            content_padding: None,
            chrome: true,
        }
    }
}

pub(in crate::widgets) fn build_multiline(
    input: TextInputProps,
    state: TextState,
    caret_rect: Option<Signal<Option<Rect>>>,
    config: MultilineConfig,
    mut style_accumulator: Style,
) -> NodeBuilder {
    let interaction = InteractionState::new(true).disabled(input.disabled);
    let runtime = TextEditRuntime::new(
        state,
        input.disabled,
        input.change.unwrap_or_else(noop_change),
        input.submit.unwrap_or_else(noop_change),
        input.clipboard.unwrap_or_else(clipboard),
        input.caret_visible.unwrap_or_else(|| signal(true)),
        config.content_padding.unwrap_or_else(content_padding),
    );
    if let Some(target) = caret_rect {
        runtime.ime.report_caret_rect(target);
    }
    let overlay = style_accumulator.take_state_overlay();
    let node = build_base_node(
        &runtime,
        &input.placeholder,
        interaction,
        overlay,
        config.chrome,
    );
    let node = interaction.install(attach_key_handlers(
        node,
        runtime.clone(),
        config.submit_key,
    ));
    let node = attach_ime_handlers(node, &runtime);
    caller_style(node, style_accumulator).with(edit_layout())
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
