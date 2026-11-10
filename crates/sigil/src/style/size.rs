/// Size step of a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlSize {
    /// The compact control size.
    Compact,
    /// The default control size.
    Default,
    /// The wide control size.
    Wide,
    /// The grow control size.
    Grow,
}
