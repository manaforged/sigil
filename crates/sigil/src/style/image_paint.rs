use crate::kernel::{ImageSource, Node};

#[derive(Clone, Default)]
pub(crate) struct ImagePaints(Vec<ImageSource>);

impl ImagePaints {
    pub(crate) fn push(&mut self, source: ImageSource) -> u64 {
        let id = source.id();
        if self.get(id).is_none() {
            self.0.push(source);
        }
        id
    }

    pub(crate) fn get(&self, id: u64) -> Option<&ImageSource> {
        self.0.iter().find(|source| source.id() == id)
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn install(self, node: &mut Node) {
        if self.is_empty() {
            return;
        }
        if let Some(owned) = node.extension_mut::<Self>() {
            for source in self.0 {
                owned.push(source);
            }
        } else {
            let _previous = node.attach(self);
        }
    }
}

pub(crate) fn lookup(node: &Node, id: u64) -> Option<&ImageSource> {
    node.extension::<ImagePaints>()?.get(id)
}

impl std::fmt::Debug for ImagePaints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(ImageSource::id))
            .finish()
    }
}

impl PartialEq for ImagePaints {
    fn eq(&self, other: &Self) -> bool {
        self.0
            .iter()
            .map(ImageSource::id)
            .eq(other.0.iter().map(ImageSource::id))
    }
}

impl Eq for ImagePaints {}
