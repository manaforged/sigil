use core::error::Error;
use core::hash::Hash;

pub trait A11yBridge {
    type Error: Error + Send + Sync + 'static;

    type Update;

    type Action;

    type NodeId: Copy + Eq + Hash;

    fn publish(&mut self, update: Self::Update) -> Result<(), Self::Error>;

    fn take_action(&mut self) -> Option<(Self::NodeId, Self::Action)>;
}
