use crate::kernel::IntoNode;

pub trait RowRender<T>: 'static {
    type Row: IntoNode + 'static;

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
