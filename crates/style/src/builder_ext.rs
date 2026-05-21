//! Builder extension trait for installing styles.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use kernel::{Node, NodeBuilder, TextBuilder};
use reactive::Signal;
use smallvec::SmallVec;

use crate::{
    FocusRing, Style, StyleAtom, StyleBinding, StyleReactiveEffect, Theme, merge_classes,
    parse_class_string, parser::ParseError,
};

static EMPTY_ATOMS: &[StyleAtom] = &[];

/// Source accepted by [`NodeStyleExt::class`].
pub enum ClassSource {
    /// Already-static atoms, usually from `classes!`.
    Static(&'static [StyleAtom]),
    /// Owned atoms, usually from [`Style`].
    Owned(Box<SmallVec<[StyleAtom; 16]>>),
    /// Raw utility-class literal parsed through the runtime parser.
    RawString(&'static str),
}

impl From<&'static [StyleAtom]> for ClassSource {
    fn from(value: &'static [StyleAtom]) -> Self {
        Self::Static(value)
    }
}

impl<const N: usize> From<&'static [StyleAtom; N]> for ClassSource {
    fn from(value: &'static [StyleAtom; N]) -> Self {
        Self::Static(value.as_slice())
    }
}

impl From<&'static str> for ClassSource {
    fn from(value: &'static str) -> Self {
        Self::RawString(value)
    }
}

impl From<Style> for ClassSource {
    fn from(value: Style) -> Self {
        Self::Owned(Box::new(value.into_atoms()))
    }
}

/// Adds style methods to [`NodeBuilder`].
pub trait NodeStyleExt {
    /// Attach utility classes or atoms to this node.
    #[must_use]
    fn class(self, source: impl Into<ClassSource>) -> Self;

    /// Attach a struct-built style to this node.
    #[must_use]
    fn style(self, style: Style) -> Self;

    /// Attach a theme to this node.
    #[must_use]
    fn theme(self, theme: Theme) -> Self;

    /// Attach focus-ring semantics to this node.
    #[must_use]
    fn focus_ring(self, ring: FocusRing) -> Self;

    /// Attach a signal-driven static atom slice to this node.
    #[must_use]
    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static;

    /// Conditionally attach a utility-class literal when `signal` is `true`.
    ///
    /// The class string is parsed once on first install and cached for
    /// the lifetime of the program. Whenever `signal` flips, the node's
    /// reactive style slice is rewritten to either the parsed atoms
    /// (signal `true`) or an empty slice (signal `false`).
    ///
    /// This is a thin wrapper over [`style_reactive`](Self::style_reactive)
    /// that gives callers a one-line reactive opt-in to a class without
    /// writing the `if signal.get()` boilerplate.
    ///
    /// Same caveat as [`style_reactive`](Self::style_reactive): a later
    /// `class_when` / `style_reactive` call on the same node replaces
    /// the previous reactive slot. Use multiple atoms in one class
    /// string when multiple toggles must compose.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use kernel::container;
    /// use reactive::signal;
    /// use style::NodeStyleExt;
    ///
    /// let highlighted = signal(false);
    /// let node = container().class_when(highlighted, "bg-accent");
    /// // `node` carries `bg-accent` only while `highlighted` is true.
    /// ```
    #[must_use]
    fn class_when(self, signal: Signal<bool>, class: &'static str) -> Self
    where
        Self: Sized,
    {
        let atoms = cached_class_string(class);
        self.style_reactive(move || if signal.get() { atoms } else { EMPTY_ATOMS })
    }
}

impl NodeStyleExt for NodeBuilder {
    fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.mutate(move |node| install_class(node, source))
    }

    fn style(self, style: Style) -> Self {
        self.class(style)
    }

    fn theme(self, theme: Theme) -> Self {
        self.mutate(move |node| install_theme(node, theme))
    }

    fn focus_ring(self, ring: FocusRing) -> Self {
        self.mutate(move |node| install_focus_ring(node, ring))
    }

    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.mutate(move |node| install_reactive(node, f))
    }
}

impl NodeStyleExt for TextBuilder {
    fn class(self, source: impl Into<ClassSource>) -> Self {
        let source = source.into();
        self.mutate(move |node| install_class(node, source))
    }

    fn style(self, style: Style) -> Self {
        self.class(style)
    }

    fn theme(self, theme: Theme) -> Self {
        self.mutate(move |node| install_theme(node, theme))
    }

    fn focus_ring(self, ring: FocusRing) -> Self {
        self.mutate(move |node| install_focus_ring(node, ring))
    }

    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static,
    {
        self.mutate(move |node| install_reactive(node, f))
    }
}

fn install_class(node: &mut Node, source: ClassSource) {
    debug_assert_not_fragment(node, "class");
    let incoming = binding_from_source(source);
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        let existing = binding.all_atoms();
        let incoming_atoms = incoming.all_atoms();
        let merged = merge_classes(&existing, &incoming_atoms, 0);
        binding.replace_owned(smallvec16(merged));
        return;
    }
    let _previous = node.attach(incoming);
}

fn debug_assert_not_fragment(node: &Node, op: &'static str) {
    debug_assert!(
        !matches!(node.kind, kernel::NodeType::Fragment),
        "{op}() on a Fragment: fragments have no paint box; see DR-015"
    );
}

fn install_theme(node: &mut Node, theme: Theme) {
    debug_assert_not_fragment(node, "theme");
    let _previous = node.attach(theme);
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.invalidate_theme();
    }
}

fn install_focus_ring(node: &mut Node, ring: FocusRing) {
    debug_assert_not_fragment(node, "focus_ring");
    let _previous = node.attach(ring);
}

fn install_reactive<F>(node: &mut Node, f: F)
where
    F: Fn() -> &'static [StyleAtom] + 'static,
{
    debug_assert_not_fragment(node, "style_reactive");
    let reactive = StyleReactiveEffect::new();
    let atoms = reactive.atoms();
    if let Some(binding) = node.get_extension_mut::<StyleBinding>() {
        binding.set_reactive(atoms);
    } else {
        let _previous = node.attach(StyleBinding::reactive_atoms(atoms));
    }

    let effect_state = reactive.clone();
    reactive::effect_named("sigil-style::style_reactive", move || {
        effect_state.set_slice(f());
    });
    let _previous = node.attach(reactive);
}

fn binding_from_source(source: ClassSource) -> StyleBinding {
    match source {
        ClassSource::Static(atoms) => StyleBinding::static_atoms(atoms),
        ClassSource::Owned(atoms) => StyleBinding::owned_atoms(*atoms),
        ClassSource::RawString(input) => StyleBinding::static_atoms(cached_class_string(input)),
    }
}

fn cached_class_string(input: &'static str) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<usize, &'static [StyleAtom]>>> = OnceLock::new();

    let key = input.as_ptr() as usize;
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut cache) = cache.lock() else {
        return parse_and_leak(input);
    };
    if let Some(atoms) = cache.get(&key).copied() {
        return atoms;
    }
    let atoms = parse_and_leak(input);
    cache.insert(key, atoms);
    atoms
}

fn parse_and_leak(input: &'static str) -> &'static [StyleAtom] {
    let mut atoms: SmallVec<[StyleAtom; 16]> = SmallVec::new();
    for token in input.split(' ').filter(|token| !token.is_empty()) {
        match parse_class_string(token) {
            Ok(parsed) => atoms.extend(parsed),
            Err(err) => diagnostic::class_parse(input, &err),
        }
    }
    Box::leak(atoms.into_vec().into_boxed_slice())
}

fn smallvec16(atoms: SmallVec<[StyleAtom; 32]>) -> SmallVec<[StyleAtom; 16]> {
    atoms.into_iter().collect()
}

mod diagnostic {
    use super::ParseError;
    use crate::merge::bump_unknown_class_count;

    #[cold]
    #[track_caller]
    pub(super) fn class_parse(input: &str, err: &ParseError) {
        bump_unknown_class_count();
        tracing::warn!(
            target: "style::class",
            input,
            error = %err,
            "unknown class token ignored",
        );
    }
}
