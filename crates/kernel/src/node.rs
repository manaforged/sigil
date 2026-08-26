//! Node model and typed extension bag.

use std::any::{Any, TypeId};

use sigil_reactive::OwnerId;
use smallvec::SmallVec;

use crate::{A11yProps, NodeId, TextContent};

/// Marker extension: paint this node's subtree on the top layer.
///
/// The scene walk defers a `TopLayer` subtree to the end of emission, so
/// it renders above every sibling that follows it in tree order. This is
/// the primitive for floating UI (dropdown popups, tooltips) that must
/// not be overdrawn by later content. Attach at build time with
/// `NodeBuilder::with(TopLayer)`. Layout is unaffected — only paint
/// order changes; hit-testing already resolves through layout geometry.
#[derive(Debug, Clone, Copy)]
pub struct TopLayer;

/// Layout hole for a platform view (`NSView`, HWND, GTK).
///
/// The GPU leaves this node's rect empty. Attach with
/// `NodeBuilder::with(Surface { key })`. After layout the host reports
/// `(key, rect)` so the shell can place the native view.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    /// App-chosen key. Distinct holes need distinct keys.
    pub key: u64,
}

/// One arena node in the composed UI tree.
#[derive(Debug)]
pub struct Node {
    /// Hot-path structural kind.
    pub kind: NodeType,
    /// Mandatory accessibility metadata.
    pub a11y: A11yProps,
    /// Parent node, if this node is attached below another node.
    ///
    /// Crate-private: structural pointers are mutated only by [`Cx`]
    /// (B3 — review 2026-05-17). External readers use
    /// [`Node::parent`] / [`Node::children`].
    pub(crate) parent: Option<NodeId>,
    /// Ordered child nodes.
    ///
    /// Crate-private for the same reason as [`Node::parent`] above.
    pub(crate) children: SmallVec<[NodeId; 4]>,
    /// Reactive owner active when this node was inserted.
    pub owner: OwnerId,
    pub(crate) extensions: Option<Box<NodeExtensions>>,
    /// Monotonic per-node version — bumped on every mutation
    /// (`attach` / `remove_extension` / structural reparent / child
    /// list change / accessibility update). Consumers like
    /// `sigil_layout::LayoutEngine` cache `(NodeId, version)` and
    /// fast-path when a re-sync sees no version change. Wraps at
    /// `u32::MAX` (a stale wrapped value still differs from a current
    /// reading because we compare for equality, not ordering).
    pub(crate) version: u32,
}

impl Node {
    pub(crate) fn new(kind: NodeType, a11y: A11yProps, owner: OwnerId) -> Self {
        Self {
            kind,
            a11y,
            parent: None,
            children: SmallVec::new(),
            owner,
            extensions: None,
            version: 1,
        }
    }

    /// Bump the per-node version. Called from every `&mut self` mutation
    /// site so external watchers can cheap-out on unchanged nodes.
    #[inline]
    pub(crate) const fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    /// Per-node version — bumped on every structural or extension
    /// mutation. Layout / renderer / a11y consumers compare against a
    /// cached prior value to short-circuit when a node is unchanged.
    #[must_use]
    #[inline]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Attach a typed extension component to this node.
    ///
    /// Returns the previous extension of the same type, if one was
    /// already attached.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        self.bump_version();
        self.extensions
            .get_or_insert_with(|| Box::new(NodeExtensions::default()))
            .attach(value)
    }

    /// Attach a typed extension WITHOUT bumping the node's version.
    ///
    /// `version` tracks layout-*input* changes, and downstream invalidation
    /// (the layout engine's input signature) folds it in. Use this only for
    /// layout *output* written back onto the node — a resolved
    /// `LayoutResult`, say — whose presence must not invalidate the very
    /// cache it came from. Attaching a real input this way would strand a
    /// stale layout; attaching output this way keeps the input signature
    /// stable across a compute so a following unchanged frame is a cache hit.
    pub fn attach_untracked<T: 'static>(&mut self, value: T) -> Option<T> {
        self.extensions
            .get_or_insert_with(|| Box::new(NodeExtensions::default()))
            .attach(value)
    }

    /// Borrow a typed extension component from this node.
    #[must_use]
    pub fn get_extension<T: 'static>(&self) -> Option<&T> {
        self.extensions.as_ref().and_then(|bag| bag.get::<T>())
    }

    /// Mutably borrow a typed extension component from this node.
    pub fn get_extension_mut<T: 'static>(&mut self) -> Option<&mut T> {
        // Conservative: bump on any mut access — the caller may
        // change the value. Callers who only need read should use
        // `get_extension`. Skipping the bump here would let typed
        // mutations bypass the version stamp and silently desync the
        // engine cache.
        self.bump_version();
        self.extensions.as_mut().and_then(|bag| bag.get_mut::<T>())
    }

    /// Mutably borrow an extension without bumping `version`.
    ///
    /// Paint-only writes (hover overlay, slot cell) use this so layout
    /// can cache the last input signature.
    pub fn get_extension_mut_untracked<T: 'static>(&mut self) -> Option<&mut T> {
        self.extensions.as_mut().and_then(|bag| bag.get_mut::<T>())
    }

    /// Remove a typed extension component from this node.
    pub fn remove_extension<T: 'static>(&mut self) -> Option<T> {
        self.bump_version();
        self.extensions.as_mut().and_then(|bag| bag.remove::<T>())
    }

    /// Return this node's parent, if any.
    ///
    /// Read-only accessor — structural pointers are mutated only by
    /// the kernel's `Cx` (see B3, 2026-05-17).
    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Return this node's children in document order.
    ///
    /// Read-only accessor — structural pointers are mutated only by
    /// the kernel's `Cx` (see B3, 2026-05-17).
    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    /// Return whether this node has allocated extension storage.
    #[must_use]
    pub fn has_extensions(&self) -> bool {
        self.extensions
            .as_ref()
            .is_some_and(|extensions| !extensions.is_empty())
    }
}

/// Closed set of core node types.
#[derive(Clone, Debug)]
pub enum NodeType {
    /// A structural node with its own layout box.
    ///
    /// Use `Container` for any node that participates in layout as an
    /// independent box — flex rows, grids, panels, buttons, cards.
    /// The layout engine assigns it a position and size, and its
    /// children are laid out inside it.
    Container,
    /// Text content. Rendering and shaping live in later crates.
    Text {
        /// Static or reactive text content.
        content: TextContent,
        /// When `true` (the default), the text wraps to its containing
        /// block's width. When `false`, it stays one line. Set via
        /// [`crate::TextBuilder::wrap`] / [`crate::TextBuilder::nowrap`].
        wrap: bool,
        /// When `true`, the text stays on one line and is truncated with a
        /// trailing ellipsis (`…`) if it is given less width than it needs
        /// (CSS `text-overflow: ellipsis`). Mutually exclusive with `wrap`.
        /// Set via [`crate::TextBuilder::ellipsize`].
        ellipsis: bool,
    },
    /// Raster image content. A leaf whose intrinsic size is its pixel
    /// dimensions; the renderer uploads the pixels to a GPU texture and
    /// samples them into the node's laid-out rect.
    Image {
        /// Decoded pixels plus a stable content id for texture caching.
        source: crate::ImageSource,
        /// How the pixels map into the laid-out box (CSS `object-fit`).
        fit: crate::ObjectFit,
    },
    /// A grouping node with **no layout box of its own**.
    ///
    /// A `Fragment`'s children are laid out as if they were direct
    /// children of the Fragment's parent — the Fragment itself takes
    /// up no space and has no border, padding, or background.
    ///
    /// Use `Fragment` when you need to group siblings for logical or
    /// conditional rendering without adding an extra layout level.
    /// For example, conditionally rendering two sibling labels as one
    /// unit without wrapping them in a visible container.
    Fragment,
}

type ExtensionEntry = (TypeId, Box<dyn Any>);

/// Typed extension storage for downstream crates.
///
/// Empty nodes pay only for `Option<Box<NodeExtensions>>`; the bag is
/// allocated on the first attachment. Storage is a
/// `SmallVec<[ExtensionEntry; 4]>` rather than an `AHashMap` — hot-path
/// nodes attach 1-3 extensions (`LayoutStyle` always, `LayoutResult`
/// after first layout, occasional event handlers / `StyleBinding`), and
/// a linear scan over ≤4 inline entries is faster than hashing a
/// `TypeId` + probing an `AHashMap`. The inline storage also skips the
/// per-node bucket allocation that dominated `create_10k` mount cost
/// (~1 µs / node saved at 50k nodes).
#[derive(Default)]
pub struct NodeExtensions {
    entries: SmallVec<[ExtensionEntry; 4]>,
}

impl NodeExtensions {
    /// Attach a typed component, returning the previous value of that
    /// same type if one existed.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        let tid = TypeId::of::<T>();
        for entry in &mut self.entries {
            if entry.0 == tid {
                let previous = core::mem::replace(&mut entry.1, Box::new(value));
                return Self::downcast_box::<T>(previous);
            }
        }
        self.entries.push((tid, Box::new(value)));
        None
    }

    /// Borrow a typed component.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_ref::<T>())
    }

    /// Mutably borrow a typed component.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter_mut()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_mut::<T>())
    }

    /// Remove a typed component.
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        let tid = TypeId::of::<T>();
        let position = self.entries.iter().position(|entry| entry.0 == tid)?;
        let (_, boxed) = self.entries.swap_remove(position);
        Self::downcast_box::<T>(boxed)
    }

    /// Return whether the extension bag has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn downcast_box<T: 'static>(value: Box<dyn Any>) -> Option<T> {
        value
            .downcast::<T>()
            .map_or_else(|_| None, |value| Some(*value))
    }
}

impl core::fmt::Debug for NodeExtensions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NodeExtensions")
            .field("len", &self.entries.len())
            .finish()
    }
}
