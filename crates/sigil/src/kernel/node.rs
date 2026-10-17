use std::any::{Any, TypeId};

use crate::reactive::OwnerId;
use smallvec::SmallVec;

use crate::kernel::delta::NodeTag;
use crate::kernel::{A11yProps, NodeId, TextContent};

#[derive(Debug, Clone, Copy)]
pub struct TopLayer;

#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub key: u64,
}

#[derive(Debug)]
pub struct Node {
    pub kind: NodeType,
    pub a11y: A11yProps,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: SmallVec<[NodeId; 4]>,
    pub owner: OwnerId,
    pub(crate) extensions: Option<Box<NodeExtensions>>,
    pub(crate) version: u32,
    tag: Option<NodeTag>,
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
            tag: None,
        }
    }

    pub(crate) const fn tagged(mut self, tag: NodeTag) -> Self {
        self.tag = Some(tag);
        self
    }

    #[must_use]
    pub const fn tag(&self) -> Option<NodeTag> {
        self.tag
    }

    #[inline]
    pub(crate) const fn bump_version(&mut self) {
        self.version = self.version.wrapping_add(1);
    }

    #[must_use]
    #[inline]
    pub const fn version(&self) -> u32 {
        self.version
    }

    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        self.bump_version();
        self.attach_untracked(value)
    }

    pub fn attach_untracked<T: 'static>(&mut self, value: T) -> Option<T> {
        self.extensions
            .get_or_insert_with(|| Box::new(NodeExtensions::default()))
            .attach(value)
    }

    #[must_use]
    pub fn get_extension<T: 'static>(&self) -> Option<&T> {
        self.extensions.as_ref().and_then(|bag| bag.get::<T>())
    }

    pub fn get_extension_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.bump_version();
        self.get_extension_mut_untracked()
    }

    pub fn get_extension_mut_untracked<T: 'static>(&mut self) -> Option<&mut T> {
        self.extensions.as_mut().and_then(|bag| bag.get_mut::<T>())
    }

    pub fn remove_extension<T: 'static>(&mut self) -> Option<T> {
        self.bump_version();
        self.extensions.as_mut().and_then(|bag| bag.remove::<T>())
    }

    #[must_use]
    pub const fn parent(&self) -> Option<NodeId> {
        self.parent
    }

    #[must_use]
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }

    #[cfg(any(test, feature = "kittest"))]
    #[must_use]
    pub fn has_extensions(&self) -> bool {
        self.extensions
            .as_ref()
            .is_some_and(|extensions| !extensions.is_empty())
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum NodeType {
    Container,
    Text {
        content: TextContent,
        wrap: bool,
        ellipsis: bool,
    },
    Image {
        source: crate::kernel::ImageSource,
        fit: crate::kernel::ObjectFit,
    },
    Fragment,
}

type ExtensionEntry = (TypeId, Box<dyn Any>);

#[derive(Default)]
pub struct NodeExtensions {
    entries: SmallVec<[ExtensionEntry; 4]>,
}

impl NodeExtensions {
    pub fn attach<T: 'static>(&mut self, value: T) -> Option<T> {
        if let Some(slot) = self.get_mut::<T>() {
            return Some(core::mem::replace(slot, value));
        }
        self.entries.push((TypeId::of::<T>(), Box::new(value)));
        None
    }

    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let tid = TypeId::of::<T>();
        self.entries
            .iter_mut()
            .find(|entry| entry.0 == tid)
            .and_then(|entry| entry.1.downcast_mut::<T>())
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        let tid = TypeId::of::<T>();
        let position = self.entries.iter().position(|entry| entry.0 == tid)?;
        let (_, boxed) = self.entries.swap_remove(position);
        boxed.downcast::<T>().ok().map(|value| *value)
    }

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
