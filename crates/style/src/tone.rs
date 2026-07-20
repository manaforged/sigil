//! Semantic tone vocabulary shared across all widgets.
//!
//! A `Tone` is the WHAT — "this UI element is a destructive action",
//! "this is informational", "this is the primary CTA". A widget
//! variant ([`crate::variant_kit`]) is the HOW — solid fill vs. soft
//! tinted vs. outline. The pair (tone, variant) plus the runtime
//! interaction state resolves to concrete atoms via the theme's
//! scales.
//!
//! Tones are extensible: adding a new brand color is two changes —
//! add a variant to this enum + a [`ColorScale`](crate::scale::ColorScale)
//! field to [`Theme`](crate::Theme). No widget code touches.

use crate::scale::ScaleRole;

/// Semantic tone — the color identity of a UI element.
///
/// Used by widgets to declare intent (`Tone::Error` for a destructive
/// action, `Tone::Primary` for a CTA, `Tone::Neutral` for a default
/// surface control). The widget resolves the tone against the theme's
/// scales at apply time via [`ToneContext`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Tone {
    /// Neutral surface tone — maps to the gray scale.
    #[default]
    Neutral,
    /// Primary action tone — maps to the accent scale.
    Primary,
    /// Informational tone — maps to the info scale.
    Info,
    /// Positive / success tone — maps to the success scale.
    Success,
    /// Caution / warning tone — maps to the warn scale.
    Warn,
    /// Destructive / error tone — maps to the error scale.
    Error,
}

impl Tone {
    /// The theme [`ScaleRole`] this tone resolves through.
    #[must_use]
    pub const fn scale_role(self) -> ScaleRole {
        match self {
            Self::Neutral => ScaleRole::Gray,
            Self::Primary => ScaleRole::Accent,
            Self::Info => ScaleRole::Info,
            Self::Success => ScaleRole::Success,
            Self::Warn => ScaleRole::Warn,
            Self::Error => ScaleRole::Error,
        }
    }
}

/// Per-node tone context.
///
/// Widgets attach one of these to set the resolution tone for any
/// [`StyleAtom::BgToneStep`](crate::StyleAtom) /
/// [`StyleAtom::FgToneStep`](crate::StyleAtom) /
/// [`StyleAtom::FgOnSolid`](crate::StyleAtom) atoms found on the
/// node's style binding.
///
/// Inheritance: the resolver looks for `ToneContext` on the current
/// node only — tones don't propagate to children (each widget owns
/// its own tone). Widgets that need to forward a tone to a subtree
/// install the same extension on the child.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ToneContext {
    /// The tone to resolve this node's tone-step atoms against.
    pub tone: Tone,
}

impl ToneContext {
    /// Build a context for the given tone.
    #[must_use]
    pub const fn new(tone: Tone) -> Self {
        Self { tone }
    }
}

impl From<Tone> for ToneContext {
    fn from(tone: Tone) -> Self {
        Self::new(tone)
    }
}
