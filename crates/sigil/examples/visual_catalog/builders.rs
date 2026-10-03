use super::spec::Build;
use super::{chrome, content, controls, data, inputs, overlays, plumbing};

const BUILDERS: [&[(&str, Build)]; 7] = [
    plumbing::BUILDERS,
    controls::BUILDERS,
    inputs::BUILDERS,
    data::BUILDERS,
    content::BUILDERS,
    chrome::BUILDERS,
    overlays::BUILDERS,
];

pub(crate) fn builder(id: &str) -> Option<Build> {
    BUILDERS
        .iter()
        .flat_map(|table| table.iter())
        .find(|(name, _)| *name == id)
        .map(|(_, build)| *build)
}
