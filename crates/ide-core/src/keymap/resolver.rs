//! [`Resolver`] — chord-aware binding lookup with layered precedence.
//!
//! Bindings are organised as a trie keyed by [`Key`] sequence. The
//! resolver carries a small piece of state — the *pending* prefix of
//! keys that have matched a non-terminal trie node but not yet hit a
//! terminal. Callers push one [`Key`] at a time via
//! [`Resolver::feed`]; each call returns the new [`Resolution`].
//!
//! Bindings ship with an optional [`ContextSet`] predicate — a plain
//! set of strings (e.g. `{"editor", "focused"}`). The brief notes the
//! full `when`-expression evaluator is a followup; the first cut is
//! "all required contexts are active" (set superset match).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use super::key::Key;

/// A live set of context tags (e.g. `{"editor", "focused"}`).
///
/// Bindings only fire when every tag in their context predicate is
/// present in the resolver's [`ContextSet`].
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContextSet(BTreeSet<Cow<'static, str>>);

impl ContextSet {
    /// Empty context — no tags active.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    /// Activate `tag`. Idempotent.
    pub fn insert(&mut self, tag: impl Into<Cow<'static, str>>) {
        self.0.insert(tag.into());
    }

    /// Deactivate `tag`. Returns whether it was present.
    pub fn remove(&mut self, tag: &str) -> bool {
        self.0.remove(tag)
    }

    /// Whether `tag` is active.
    #[must_use]
    pub fn contains(&self, tag: &str) -> bool {
        self.0.contains(tag)
    }

    /// Build a [`ContextSet`] from an iterator of tags.
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'static, str>>,
    {
        let mut set = Self::new();
        for tag in iter {
            set.insert(tag);
        }
        set
    }
}

/// The outcome of feeding a single [`Key`] into [`Resolver::feed`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum Resolution {
    /// The pending sequence resolved to an action. The resolver's
    /// pending buffer is cleared.
    Action(Cow<'static, str>),
    /// The pending sequence is a strict prefix of one-or-more bindings;
    /// the caller should wait for the next key.
    Pending,
    /// No binding matches the pending sequence. The resolver's pending
    /// buffer is cleared.
    Unbound,
}

/// One installed binding. Stored inside [`Bindings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    pub keys: Vec<Key>,
    pub action: Cow<'static, str>,
    pub required_context: BTreeSet<Cow<'static, str>>,
}

/// Container for installed bindings.
///
/// A sorted list backed by a stable insertion order. Layered config sits
/// one level up ([`super::config`]); this type is the resolved-and-
/// flattened view the resolver operates on.
#[derive(Debug, Default, Clone)]
pub struct Bindings {
    list: Vec<Binding>,
}

impl Bindings {
    /// Empty binding set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a binding. Later inserts with the same `keys` + context
    /// override earlier ones (user-overrides-default semantics).
    pub fn insert<S>(
        &mut self,
        keys: Vec<Key>,
        action: S,
        required_context: impl IntoIterator<Item = Cow<'static, str>>,
    ) where
        S: Into<Cow<'static, str>>,
    {
        let action = action.into();
        let required_context: BTreeSet<_> = required_context.into_iter().collect();
        // Replace any prior binding with the same key-sequence + context.
        if let Some(slot) = self
            .list
            .iter_mut()
            .find(|b| b.keys == keys && b.required_context == required_context)
        {
            slot.action = action;
            return;
        }
        self.list.push(Binding {
            keys,
            action,
            required_context,
        });
    }

    /// Number of installed bindings (counting overrides as one entry).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no bindings are installed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.list.iter()
    }
}

/// The stateful side of binding resolution.
///
/// Construct with [`Resolver::new`]. Feed [`Key`] events in via
/// [`Resolver::feed`]. The resolver carries a small pending-prefix
/// buffer between calls so multi-key chords (`"g d"`) resolve across
/// successive events.
#[derive(Debug, Default)]
pub struct Resolver {
    pending: Vec<Key>,
}

impl Resolver {
    /// New resolver with an empty pending buffer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the pending buffer without consulting any binding set.
    /// Useful when the editor wants to abandon a half-typed chord
    /// (e.g. focus change).
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Current pending prefix length (0 when idle).
    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Feed one [`Key`] and return the resulting [`Resolution`].
    pub fn feed(&mut self, bindings: &Bindings, context: &ContextSet, key: Key) -> Resolution {
        self.pending.push(key);
        let outcome = classify(&self.pending, bindings, context);
        match outcome {
            Resolution::Pending => {}
            // Terminal outcomes flush the pending buffer.
            Resolution::Action(_) | Resolution::Unbound => self.pending.clear(),
        }
        outcome
    }

    /// One-shot resolution of a complete key sequence. Equivalent to
    /// calling [`Resolver::feed`] for each key, but does not mutate
    /// any resolver state. Convenient for tests and for callers who
    /// already buffer events upstream.
    #[must_use]
    pub fn resolve_sequence(bindings: &Bindings, context: &ContextSet, keys: &[Key]) -> Resolution {
        if keys.is_empty() {
            return Resolution::Unbound;
        }
        classify(keys, bindings, context)
    }
}

fn classify(prefix: &[Key], bindings: &Bindings, context: &ContextSet) -> Resolution {
    // First scan: does any context-matching binding match the prefix
    // exactly? Last-match-wins so user overrides default.
    let mut exact: Option<&Cow<'static, str>> = None;
    let mut has_extension = false;
    for b in bindings.iter() {
        if !context_satisfied(&b.required_context, context) {
            continue;
        }
        if b.keys == prefix {
            exact = Some(&b.action);
            continue;
        }
        if b.keys.len() > prefix.len() && b.keys.starts_with(prefix) {
            has_extension = true;
        }
    }
    if let Some(action) = exact {
        // Exact + extension is ambiguous — the editor convention is
        // that an exact match wins on the current key. The user can
        // still chord *past* the exact action by binding the longer
        // sequence to a different action, in which case the shorter
        // action will fire on this resolve and the next key starts a
        // fresh prefix. That's the standard chord-vs-prefix idiom.
        return Resolution::Action(action.clone());
    }
    if has_extension {
        return Resolution::Pending;
    }
    Resolution::Unbound
}

fn context_satisfied(required: &BTreeSet<Cow<'static, str>>, available: &ContextSet) -> bool {
    required.iter().all(|tag| available.contains(tag))
}

/// Sorted ordered view useful for "which key fires this action?" UIs.
/// Not on the I-T8 critical path; flagged as a followup in the brief.
#[doc(hidden)]
#[must_use]
pub(super) fn _reverse_index(bindings: &Bindings) -> BTreeMap<Cow<'static, str>, Vec<Vec<Key>>> {
    let mut map: BTreeMap<Cow<'static, str>, Vec<Vec<Key>>> = BTreeMap::new();
    for b in bindings.iter() {
        map.entry(b.action.clone())
            .or_default()
            .push(b.keys.clone());
    }
    map
}
