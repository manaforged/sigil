use super::parse::Block;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct List {
    pub(crate) start: Option<u64>,
    pub(crate) items: Vec<ListItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskMark {
    Open,
    Done,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ListItem {
    pub(crate) task: Option<TaskMark>,
    pub(crate) blocks: Vec<Block>,
}

impl ListItem {
    pub(crate) const fn plain(blocks: Vec<Block>) -> Self {
        Self { task: None, blocks }
    }
}
