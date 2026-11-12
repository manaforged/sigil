use std::any::{Any, TypeId};

use crate::reactive::Owner;
use smallvec::SmallVec;

use crate::kernel::delta::NodeTag;
use crate::kernel::{A11yProps, NodeId, TextContent};

/// Marker of a node that paints above the tree.
#[derive(Debug, Clone, Copy)]
pub struct TopLayer;

/// Marker of a host-drawn surface.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    /// The key.
    pub key: u64,
}

/// A node in the tree, as `NodeBuilder::mutate` sees it.
#[derive(Debug)]
pub struct Node {
    /// The kind.
    #[cfg(feature = "kittest")]
    pub kind: NodeType,
    #[cfg(not(feature = "kittest"))]
    pub(crate) kind: NodeType,
    /// The a11y.
    #[cfg(feature = "kittest")]
    pub a11y: A11yProps,
    #[cfg(not(feature = "kittest"))]
    pub(crate) a11y: A11yProps,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: SmallVec<[NodeId; 4]>,
    pub(crate) owner: Owner,
    pub(crate) component: Option<Owner>,
    pub(crate) extensions: Option<Box<NodeExtensions>>,
    pub(crate) version: u32,
    tag: Option<NodeTag>,
}

impl Node {
    pub(crate) fn new(kind: NodeType, a11y: A11yProps, owner: Owner) -> Self {
        Self {
            kind,
            a11y,
            parent: None,
            children: SmallVec::new(),
            owner,
            component: None,
            extensions: None,
            version: 1,
            tag: None,
        }
    }

    pub(crate) const fn tagged(mut self, tag: NodeTag) -> Self {
        self.tag = Some(tag);
        self
    }

    #[must_use]
    pub(crate) const fn tag(&self) -> Option<NodeTag> {
        self.tag
    }

    #[inline]
    pub(crate) const fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    kittest_pub! {
        /// Returns the version.
        #[must_use]
        #[inline]
        pub const fn version(&self) -> u32 {
            self.version
        }
    }

    /// Stores an extension value and returns the old value of that type.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        self.bump_version();
        self.attach_untracked(value)
    }

    /// Stores an extension value without marking the node changed.
    pub(crate) fn attach_untracked<T: 'static>(&mut self, value: T) -> Option<T> {
        self.extensions
            .get_or_insert_with(|| Box::new(NodeExtensions::default()))
            .attach(value)
    }

    /// Returns the get extension.
    #[must_use]
    pub fn get_extension<T: 'static>(&self) -> Option<&T> {
        self.extensions.as_ref().and_then(|bag| bag.get::<T>())
    }

    /// Returns the extension of type `T` for writing.
    pub fn get_extension_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.bump_version();
        self.get_extension_mut_untracked()
    }

    /// Returns the extension of type `T` for writing without marking the node changed.
    pub(crate) fn get_extension_mut_untracked<T: 'static>(&mut self) -> Option<&mut T> {
        self.extensions.as_mut().and_then(|bag| bag.get_mut::<T>())
    }

    /// Removes and returns the extension of type `T`.
    pub fn remove_extension<T: 'static>(&mut self) -> Option<T> {
        self.bump_version();
        self.extensions.as_mut().and_then(|bag| bag.remove::<T>())
    }

    /// Returns the parent.
    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    /// Returns the children.
    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn has_extensions(&self) -> bool {
        self.extensions
            .as_ref()
            .is_some_and(|extensions| !extensions.is_empty())
    }
}

/// Container, text, image, or fragment.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum NodeType {
    /// The container node type.
    Container,
    /// The text node type.
    Text {
        /// The content.
        content: TextContent,
        /// Whether wrap.
        wrap: bool,
        /// Whether ellipsis.
        ellipsis: bool,
    },
    /// The image node type.
    Image {
        /// The source.
        source: crate::kernel::ImageSource,
        /// The fit.
        fit: crate::kernel::ObjectFit,
    },
    /// The fragment node type.
    Fragment,
}

type ExtensionEntry = (TypeId, Box<dyn Any>);

/// Typed data attached to a node.
#[derive(Default)]
pub struct NodeExtensions {
    entries: SmallVec<[ExtensionEntry; 4]>,
}

impl NodeExtensions {
    /// Stores a value and returns the old value of that type.
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        if let Some(slot) = self.get_mut::<T>() {
            return Some(core::mem::replace(slot, value));
        }
        self.entries.push((TypeId::of::<T>(), Box::new(value)));
        None
    }

    /// Returns the get.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_ref::<T>())
    }

    /// Returns the value of type `T` for writing.
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter_mut()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_mut::<T>())
    }

    /// Removes and returns the value of type `T`.
    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        let tid = TypeId::of::<T>();
        let position = self.entries.iter().position(|entry| entry.0 == tid)?;
        let (_, boxed) = self.entries.swap_remove(position);
        boxed.downcast::<T>().ok().map(|value| *value)
    }

    #[cfg(feature = "kittest")]
    #[doc(hidden)]
    /// Returns whether this node extensions is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl core::fmt::Debug for NodeExtensions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("NodeExtensions")
            .field("len", &self.entries.len())
            .finish()
    }
}
