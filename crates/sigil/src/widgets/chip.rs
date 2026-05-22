//! Styled badge / pill widget for log levels, tags, and status indicators.
//!
//! Two log-tailer / kanban friction logs converged on the same shape:
//! a small rounded container holding a short label, tinted by a semantic
//! tone (success / warn / error / info / neutral). This is the smallest
//! widget that closes that gap.
//!
//! ## Theme token mapping (DS1 scale tokens)
//!
//! Each [`ChipTone`] maps to a Radix-style background/foreground pair on
//! its role scale — step 3 for the background (light-tinted), step 11
//! for the text (high-contrast same-hue). This convention keeps status
//! chips legible against any theme surface and makes the success / warn
//! / error chips visually distinct without crossing into the saturated
//! step-9 brand color (reserved for solid buttons).
//!
//! | `ChipTone` | Background    | Foreground         |
//! |------------|---------------|--------------------|
//! | `Neutral`  | `bg-gray-3`   | `text-gray-12`     |
//! | `Info`     | `bg-info-3`   | `text-info-11`     |
//! | `Success`  | `bg-success-3`| `text-success-11`  |
//! | `Warn`     | `bg-warn-3`   | `text-warn-11`     |
//! | `Error`    | `bg-error-3`  | `text-error-11`    |

use kernel::{NodeBuilder, Role, TextContent, container, text};
use layout::prelude::{LayoutBuilderExt, LayoutStyle};
use style::NodeStyleExt;

/// Semantic tone for a [`chip`] — drives the background + foreground
/// palette choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChipTone {
    /// Default, low-emphasis surface (matches body text on the panel).
    Neutral,
    /// Informational accent (matches the primary accent surface).
    Info,
    /// Positive / success accent.
    Success,
    /// Caution / warn — uses the deeper surface tint.
    Warn,
    /// Error / destructive — uses the deepest surface tint.
    Error,
}

impl ChipTone {
    /// Background + foreground utility-class fragment for this tone.
    /// Each chip uses Radix step-3 as a soft tinted background and
    /// step-11 as the legible same-hue text color.
    const fn class(self) -> &'static str {
        match self {
            Self::Neutral => "bg-gray-3 text-gray-12",
            Self::Info => "bg-info-3 text-info-11",
            Self::Success => "bg-success-3 text-success-11",
            Self::Warn => "bg-warn-3 text-warn-11",
            Self::Error => "bg-error-3 text-error-11",
        }
    }
}

/// Horizontal inner padding in logical pixels.
const CHIP_PADDING_X: f32 = 8.0;
/// Vertical inner padding in logical pixels.
const CHIP_PADDING_Y: f32 = 2.0;

/// Small styled badge / pill — rounded container with a tone-specific
/// background and a short text label.
///
/// ```ignore
/// use sigil::prelude::*;
///
/// chip("ERROR", ChipTone::Error)
/// ```
///
/// The chip uses `Role::Label` for accessibility — screen readers
/// announce the label text exactly as supplied.
#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: ChipTone) -> NodeBuilder {
    container()
        .role(Role::Label)
        .layout(
            LayoutStyle::flex_row()
                .padding_x(CHIP_PADDING_X)
                .padding_y(CHIP_PADDING_Y),
        )
        .class("rounded-sm text-xs")
        .class(tone.class())
        .child(text(label))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use kernel::{Cx, IntoNode, NodeType};
    use reactive::Runtime;

    #[test]
    fn chip_uses_label_role_and_holds_text_child() {
        Runtime::scope("chip-build-static", || {
            let mut cx = Cx::new_root();
            let id = chip("INFO", ChipTone::Info).build(&mut cx);
            let node = cx.get(id).expect("chip node exists");
            // Role::Label is the documented a11y contract — screen
            // readers must announce the chip's text verbatim, not as
            // an interactive widget.
            assert_eq!(node.a11y.role, kernel::Role::Label);
            // The chip should hold exactly one child — the text node.
            assert_eq!(node.children().len(), 1, "chip wraps a single text child");
            let text_child = cx.get(node.children()[0]).expect("chip text child exists");
            assert!(
                matches!(text_child.kind, NodeType::Text { .. }),
                "chip child is the static label text node"
            );
        });
    }

    #[test]
    fn owned_label_reaches_text_child() {
        Runtime::scope("chip-build-owned", || {
            let mut cx = Cx::new_root();
            let id = chip(String::from("ERROR"), ChipTone::Error).build(&mut cx);
            let node = cx.get(id).expect("chip node exists");
            assert_eq!(node.children().len(), 1, "chip wraps one text child");
            let text_child = cx.get(node.children()[0]).expect("chip text child exists");
            match &text_child.kind {
                NodeType::Text { content } => {
                    assert_eq!(
                        content.as_static_str(),
                        Some("ERROR"),
                        "owned label String reaches the text child verbatim"
                    );
                }
                other => panic!("expected NodeType::Text, got {other:?}"),
            }
        });
    }

    #[test]
    fn tone_class_strings_are_distinct() {
        let tones = [
            ChipTone::Neutral,
            ChipTone::Info,
            ChipTone::Success,
            ChipTone::Warn,
            ChipTone::Error,
        ];
        for (i, a) in tones.iter().enumerate() {
            for b in &tones[i + 1..] {
                assert_ne!(a.class(), b.class(), "{a:?} and {b:?} class collide");
            }
        }
    }

    #[test]
    fn every_tone_renders_one_text_child() {
        Runtime::scope("chip-build-all-tones", || {
            let mut cx = Cx::new_root();
            for tone in [
                ChipTone::Neutral,
                ChipTone::Info,
                ChipTone::Success,
                ChipTone::Warn,
                ChipTone::Error,
            ] {
                let id = chip("x", tone).build(&mut cx);
                let node = cx.get(id).expect("chip node exists");
                assert_eq!(
                    node.children().len(),
                    1,
                    "{tone:?} chip should wrap exactly one text child"
                );
            }
        });
    }
}
