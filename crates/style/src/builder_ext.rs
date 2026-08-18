//! Builder extension trait for installing styles.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_kernel::{Node, NodeBuilder, TextBuilder};
use sigil_reactive::Signal;
use smallvec::SmallVec;

use std::cell::Cell;
use std::rc::Rc;

use crate::{
    FocusRing, ReactiveStateSlot, StateSlot, Style, StyleAtom, StyleBinding, StyleOverlay,
    StyleReactiveEffect, StyleState, StyleVariant, Theme, Tone, ToneContext, merge_classes,
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

    /// Attach a per-node [`ToneContext`] — selects which scale on the
    /// theme the [`StyleAtom::BgToneStep`] /
    /// [`StyleAtom::FgToneStep`] / [`StyleAtom::FgOnSolid`] /
    /// [`StyleAtom::BorderToneStep`] atoms on this node resolve
    /// through.
    ///
    /// DS2.5 — collapses the
    /// `.mutate(|node| { let _ = node.attach(ToneContext::new(tone)); })`
    /// pattern that the early DS2.5 widgets had to write by hand.
    #[must_use]
    fn tone(self, tone: Tone) -> Self;

    /// Attach focus-ring semantics to this node.
    #[must_use]
    fn focus_ring(self, ring: FocusRing) -> Self;

    /// Attach a signal-driven static atom slice to this node.
    #[must_use]
    fn style_reactive<F>(self, f: F) -> Self
    where
        F: Fn() -> &'static [StyleAtom] + 'static;

    /// Attach a per-instance user state overlay plus the slot cell the
    /// widget's reactive style effect uses to publish the current
    /// interaction state.
    ///
    /// Most widget authors should reach for
    /// [`stateful_style`](Self::stateful_style) instead — this method
    /// is the low-level escape hatch when a widget needs to construct
    /// the slot cell itself.
    ///
    /// DS2 Phase 2 — the overlay's atoms apply AFTER the widget's
    /// reactive slot atoms, so last-atom-wins resolution gives user
    /// `.hover()` / `.focus()` / `.pressed()` / `.disabled()` atoms
    /// priority per property.
    #[must_use]
    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self;

    /// Install a reactive style variant plus per-instance user overlay
    /// in one call. Handles the slot-cell wiring internally so widget
    /// authors don't see `ReactiveStateSlot`.
    ///
    /// `state_fn` is invoked inside the reactive style effect every
    /// time a tracked signal flips; it must return the current
    /// [`StyleState`]. The helper writes the resulting slot into the
    /// cell so the resolver can pick the matching overlay slice, then
    /// returns the variant's atoms for the same state through the
    /// reactive channel.
    ///
    /// DS2.5 — collapses the four-step ritual (`Rc::new(Cell::...)`,
    /// `Rc::clone`, `style_reactive` + cell write, `state_overlay`)
    /// into one call. Widget authors that forget to wire the cell
    /// today silently lose hover / focus / pressed overlays; this
    /// helper makes that impossible.
    #[must_use]
    fn stateful_style<F>(self, variant: StyleVariant, overlay: StyleOverlay, state_fn: F) -> Self
    where
        Self: Sized,
        F: Fn() -> StyleState + 'static,
    {
        let slot_cell: ReactiveStateSlot = Rc::new(Cell::new(StateSlot::Rest));
        let cell_for_closure = Rc::clone(&slot_cell);
        self.style_reactive(move || {
            let state = state_fn();
            cell_for_closure.set(state.slot());
            variant.state(state)
        })
        .state_overlay(overlay, slot_cell)
    }

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
    /// Multiple reactive installs on the same node compose (T0): each
    /// `class_when` / `style_reactive` appends a channel, and channels
    /// resolve in install order, so later installs win per property.
    /// Use multiple atoms in one class string when multiple toggles
    /// must compose inside a single source.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use sigil_kernel::container;
    /// use sigil_reactive::signal;
    /// use sigil_style::NodeStyleExt;
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

    /// Show the node only while `signal` is true (`hidden` when false).
    ///
    /// The node stays in the tree. Prefer this over
    /// [`NodeBuilder::children_reactive`] for page chrome that should
    /// keep identity.
    #[must_use]
    fn visible_when(self, signal: Signal<bool>) -> Self
    where
        Self: Sized,
    {
        let hidden = cached_class_string("hidden");
        self.style_reactive(move || if signal.get() { EMPTY_ATOMS } else { hidden })
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

    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    fn tone(self, tone: Tone) -> Self {
        self.mutate(move |node| install_tone(node, tone))
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

    fn state_overlay(self, overlay: StyleOverlay, slot: ReactiveStateSlot) -> Self {
        self.mutate(move |node| install_state_overlay(node, overlay, slot))
    }

    fn tone(self, tone: Tone) -> Self {
        self.mutate(move |node| install_tone(node, tone))
    }
}

fn install_tone(node: &mut Node, tone: Tone) {
    debug_assert_not_fragment(node, "tone");
    let _previous = node.attach(ToneContext::new(tone));
}

fn install_state_overlay(node: &mut Node, overlay: StyleOverlay, slot: ReactiveStateSlot) {
    debug_assert_not_fragment(node, "state_overlay");
    if let Some(binding) = node.get_extension_mut_untracked::<StyleBinding>() {
        binding.set_state_overlay(overlay, slot);
        return;
    }
    let mut binding = StyleBinding::default();
    binding.set_state_overlay(overlay, slot);
    let _previous = node.attach_untracked(binding);
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
        !matches!(node.kind, sigil_kernel::NodeType::Fragment),
        "{op}() on a Fragment: fragments have no paint box; see DR-015"
    );
}

fn install_theme(node: &mut Node, theme: Theme) {
    debug_assert_not_fragment(node, "theme");
    let _previous = node.attach(theme);
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
        binding.add_reactive(atoms);
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
            target: "sigil_style::class",
            input,
            error = %err,
            "unknown class token ignored",
        );
    }
}
