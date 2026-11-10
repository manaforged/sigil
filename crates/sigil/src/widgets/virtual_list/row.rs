use crate::kernel::IntoNode;

/// Row renderer of a virtual list.
pub trait RowRender<T>: 'static {
    /// The node type of one row.
    type Row: IntoNode + 'static;

    /// Renders the row of `item` at `index`.
    fn row(&self, index: usize, item: &T) -> Self::Row;
}

impl<T, F, N> RowRender<T> for F
where
    F: Fn(usize, &T) -> N + 'static,
    N: IntoNode + 'static,
{
    type Row = N;

    fn row(&self, index: usize, item: &T) -> N {
        self(index, item)
    }
}
