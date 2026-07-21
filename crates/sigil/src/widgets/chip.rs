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
//! its role scale — step 3 for the background (light-tinted), and the
//! darkest same-hue text step clearing WCAG AA 4.5:1 against it (see
//! [`ChipTone::class`]). This convention keeps status chips legible
//! against any theme surface and makes the success / warn / error chips
//! visually distinct without crossing into the saturated step-9 brand
//! color (reserved for solid buttons).
//!
//! | `ChipTone` | Background    | Foreground         |
//! |------------|---------------|--------------------|
//! | `Neutral`  | `bg-gray-3`   | `text-gray-12`     |
//! | `Info`     | `bg-info-3`   | `text-info-12`     |
//! | `Success`  | `bg-success-3`| `text-success-12`  |
//! | `Warn`     | `bg-warn-3`   | `text-warn-12`     |
//! | `Error`    | `bg-error-3`  | `text-error-11`    |

use sigil_kernel::TextContent;

mod builder;
pub use builder::ChipBuilder;

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
    /// Each chip uses Radix step-3 as a soft tinted background with the
    /// darkest same-hue text step that clears WCAG AA 4.5:1 at the
    /// chip's `text-xs` size: step-11 where it measures ≥4.5:1 against
    /// step-3 (error 4.70:1), step-12 for the hues where step-11 falls
    /// short (info 4.26:1, success 4.21:1, warn 4.08:1 — ui-review gate
    /// 2026-07-12).
    const fn class(self) -> &'static str {
        match self {
            Self::Neutral => "bg-gray-3 text-gray-12",
            Self::Info => "bg-info-3 text-info-12",
            Self::Success => "bg-success-3 text-success-12",
            Self::Warn => "bg-warn-3 text-warn-12",
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
/// chip(status, ChipTone::Warn).tone_signal(level_signal)
/// ```
///
/// The chip uses `Role::Label` for accessibility — screen readers
/// announce the label text exactly as supplied. `.tone_signal(...)`
/// retints reactively without a remount.
#[must_use]
pub fn chip(label: impl Into<TextContent>, tone: ChipTone) -> ChipBuilder {
    ChipBuilder::new(label, tone)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sigil_kernel::{Cx, IntoNode, NodeType};
    use sigil_reactive::Runtime;

    #[test]
    fn chip_uses_label_role_and_holds_text_child() {
        Runtime::scope("chip-build-static", || {
            let mut cx = Cx::new_root();
            let id = chip("INFO", ChipTone::Info).build(&mut cx);
            let node = cx.get(id).expect("chip node exists");
            // Role::Label is the documented a11y contract — screen
            // readers must announce the chip's text verbatim, not as
            // an interactive widget.
            assert_eq!(node.a11y.role, sigil_kernel::Role::Label);
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
                NodeType::Text { content, .. } => {
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

    /// The tone pairings must HOLD WCAG AA 4.5:1, not just match token
    /// strings — a palette change can silently reintroduce the failure
    /// the 2026-07-12 ui-review gate caught (the reviewer adversarial finding).
    #[test]
    fn every_tone_clears_wcag_aa_contrast() {
        use sigil_style::Rgba8;
        use sigil_style::scale::ScaleRole;
        use sigil_style::theme::Theme;

        // keep luminance arithmetic bit-stable for goldens
        #[allow(clippy::suboptimal_flops)]
        fn luminance(c: Rgba8) -> f64 {
            let ch = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.039_28 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b)
        }

        let theme = Theme::default();
        // (tone, role, text step) — bg is always step 3, matching
        // `ChipTone::class`. Error stays step 11 (measures 4.70:1).
        let cases = [
            (ChipTone::Neutral, ScaleRole::Gray, 12),
            (ChipTone::Info, ScaleRole::Info, 12),
            (ChipTone::Success, ScaleRole::Success, 12),
            (ChipTone::Warn, ScaleRole::Warn, 12),
            (ChipTone::Error, ScaleRole::Error, 11),
        ];
        for (tone, role, text_step) in cases {
            let scale = theme.scale(role);
            let (bg, fg) = (scale.step(3), scale.step(text_step));
            let (lb, lf) = (luminance(bg), luminance(fg));
            let ratio = (lb.max(lf) + 0.05) / (lb.min(lf) + 0.05);
            assert!(
                ratio >= 4.5,
                "{tone:?}: step-{text_step} on step-3 measures {ratio:.2}:1, below WCAG AA 4.5:1"
            );
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
