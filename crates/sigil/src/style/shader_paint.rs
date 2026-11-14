use smallvec::SmallVec;

use crate::kernel::Node;
use crate::reactive::SendableSignal;
use crate::scene::Shader;
use crate::scene::canvas::watch;

#[derive(Clone, Default)]
pub(crate) struct ShaderPaints {
    list: Vec<Shader>,
    versions: SmallVec<[SendableSignal<u64>; 1]>,
}

impl ShaderPaints {
    pub(crate) const fn new() -> Self {
        Self {
            list: Vec::new(),
            versions: SmallVec::new_const(),
        }
    }

    pub(crate) fn push(&mut self, shader: Shader) -> u64 {
        let id = shader.id();
        if self.get(id).is_none() {
            self.list.push(shader);
        }
        id
    }

    pub(crate) fn get(&self, id: u64) -> Option<&Shader> {
        self.list.iter().find(|shader| shader.id() == id)
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub(crate) fn version(&self) -> u64 {
        self.versions
            .iter()
            .filter_map(|version| version.into_signal().try_peek())
            .fold(0, u64::wrapping_add)
    }

    pub(crate) fn install(mut self, node: &mut Node) {
        if self.is_empty() {
            return;
        }
        let live: Vec<Shader> = self
            .list
            .iter()
            .filter(|shader| shader.is_live())
            .cloned()
            .collect();
        let version = (!live.is_empty())
            .then(|| watch(move || live.iter().for_each(Shader::track)).into_sendable());
        if let Some(owned) = node.extension_mut::<Self>() {
            for shader in self.list {
                owned.push(shader);
            }
            owned.versions.extend(version);
        } else {
            self.versions.extend(version);
            let _previous = node.attach(self);
        }
    }
}

pub(crate) fn lookup(node: &Node, id: u64) -> Option<&Shader> {
    node.extension::<ShaderPaints>()?.get(id)
}

impl std::fmt::Debug for ShaderPaints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.list.iter().map(Shader::id))
            .finish()
    }
}

impl PartialEq for ShaderPaints {
    fn eq(&self, other: &Self) -> bool {
        self.list
            .iter()
            .map(Shader::id)
            .eq(other.list.iter().map(Shader::id))
    }
}

impl Eq for ShaderPaints {}
