//! Node model and typed extension bag.

use std::any::{Any, TypeId};

use ahash::AHashMap;
use sigil_reactive::OwnerId;
use smallvec::SmallVec;

use crate::{NodeId, Semantics, TextSource};

/// One arena node in the composed UI tree.
#[derive(Debug)]
pub struct Node {
    /// Hot-path structural kind.
    pub kind: NodeKind,
    /// Mandatory accessibility metadata.
    pub semantics: Semantics,
    /// Parent node, if this node is attached below another node.
    pub parent: Option<NodeId>,
    /// Ordered child nodes.
    pub children: SmallVec<[NodeId; 4]>,
    /// Reactive owner active when this node was inserted.
    pub owner: OwnerId,
    pub(crate) extensions: Option<Box<NodeExtensions>>,
}

impl Node {
    pub(crate) fn new(kind: NodeKind, semantics: Semantics, owner: OwnerId) -> Self {
        Self {
            kind,
            semantics,
            parent: None,
            children: SmallVec::new(),
            owner,
            extensions: None,
        }
    }

    /// Attach a typed extension component to this node.
    ///
    /// Returns the previous extension of the same type, if one was
    /// already attached.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
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
        self.extensions.as_mut().and_then(|bag| bag.get_mut::<T>())
    }

    /// Remove a typed extension component from this node.
    pub fn remove_extension<T: 'static>(&mut self) -> Option<T> {
        self.extensions.as_mut().and_then(|bag| bag.remove::<T>())
    }

    /// Return whether this node has allocated extension storage.
    #[must_use]
    pub fn has_extensions(&self) -> bool {
        self.extensions
            .as_ref()
            .is_some_and(|extensions| !extensions.is_empty())
    }
}

/// Closed set of core node kinds.
#[derive(Clone, Debug)]
pub enum NodeKind {
    /// A structural container that may hold children.
    Container,
    /// Text content. Rendering and shaping live in later crates.
    Text {
        /// Static or reactive text source.
        source: TextSource,
    },
    /// A structural grouping node with no visual box of its own.
    Fragment,
}

/// Typed extension storage for downstream crates.
///
/// Empty nodes pay only for `Option<Box<NodeExtensions>>`; the map is
/// allocated on the first attachment.
#[derive(Default)]
pub struct NodeExtensions {
    map: AHashMap<TypeId, Box<dyn Any>>,
}

impl NodeExtensions {
    /// Attach a typed component, returning the previous value of that
    /// same type if one existed.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        let previous = self.map.insert(TypeId::of::<T>(), Box::new(value));
        previous.and_then(Self::downcast_box::<T>)
    }

    /// Borrow a typed component.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref::<T>())
    }

    /// Mutably borrow a typed component.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut::<T>())
    }

    /// Remove a typed component.
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(Self::downcast_box::<T>)
    }

    /// Return whether the extension bag has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
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
            .field("len", &self.map.len())
            .finish()
    }
}
