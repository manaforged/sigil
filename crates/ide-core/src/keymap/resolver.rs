use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use super::key::Key;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ContextSet(BTreeSet<Cow<'static, str>>);

impl ContextSet {
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn insert(&mut self, tag: impl Into<Cow<'static, str>>) {
        self.0.insert(tag.into());
    }

    pub fn remove(&mut self, tag: &str) -> bool {
        self.0.remove(tag)
    }

    #[must_use]
    pub fn contains(&self, tag: &str) -> bool {
        self.0.contains(tag)
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::exhaustive_enums)]
pub enum Resolution {
    Action(Cow<'static, str>),
    Pending,
    Unbound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Binding {
    pub keys: Vec<Key>,
    pub action: Cow<'static, str>,
    pub required_context: BTreeSet<Cow<'static, str>>,
}

#[derive(Debug, Default, Clone)]
pub struct Bindings {
    list: Vec<Binding>,
}

impl Bindings {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

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

    #[must_use]
    pub const fn len(&self) -> usize {
        self.list.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Binding> {
        self.list.iter()
    }
}

#[derive(Debug, Default)]
pub struct Resolver {
    pending: Vec<Key>,
}

impl Resolver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    #[must_use]
    pub const fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn feed(&mut self, bindings: &Bindings, context: &ContextSet, key: Key) -> Resolution {
        self.pending.push(key);
        let outcome = classify(&self.pending, bindings, context);
        match outcome {
            Resolution::Pending => {}
            Resolution::Action(_) | Resolution::Unbound => self.pending.clear(),
        }
        outcome
    }

    #[must_use]
    pub fn resolve_sequence(bindings: &Bindings, context: &ContextSet, keys: &[Key]) -> Resolution {
        if keys.is_empty() {
            return Resolution::Unbound;
        }
        classify(keys, bindings, context)
    }
}

fn classify(prefix: &[Key], bindings: &Bindings, context: &ContextSet) -> Resolution {
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
