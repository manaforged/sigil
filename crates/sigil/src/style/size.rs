/// Size step of a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ControlSize {
    /// The small step: a control 24 px high in a window.
    Compact,
    /// The standard step: a control 28 px high in a window.
    Default,
    /// The large step: a control 36 px high in a window.
    Wide,
}
