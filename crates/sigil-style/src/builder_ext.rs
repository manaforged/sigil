//! Builder extension trait for installing styles.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_core::{Node, NodeBuilder};
use smallvec::SmallVec;

use crate::{
    Style, StyleAtom, StyleBinding, StyleReactiveEffect, Theme, merge_classes, parse_class_string,
    parser::ParseError,
};

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

    /// Attach a signal-driven static atom slice to this node.
    #[must_use]
    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static;
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
        !matches!(node.kind, sigil_core::NodeType::Fragment),
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
    sigil_reactive::effect_named("sigil-style::style_reactive", move || {
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
    let atoms = parse_class_string(input).unwrap_or_else(|err| diagnostic::class_parse(&err));
    Box::leak(atoms.into_vec().into_boxed_slice())
}

fn smallvec16(atoms: SmallVec<[StyleAtom; 32]>) -> SmallVec<[StyleAtom; 16]> {
    atoms.into_iter().collect()
}

mod diagnostic {
    #![allow(clippy::panic)]

    use super::ParseError;

    #[cold]
    #[track_caller]
    pub(super) fn class_parse(err: &ParseError) -> ! {
        panic!("{err}");
    }
}
