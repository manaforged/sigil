pub(crate) mod row_sealed {
    use crate::kernel::IntoNode;

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed<T>: 'static {
        type Row: IntoNode + 'static;
        fn row(&self, index: usize, item: &T) -> Self::Row;
    }

    impl<T, F, N> Sealed<T> for F
    where
        F: Fn(usize, &T) -> N + 'static,
        N: IntoNode + 'static,
    {
        type Row = N;

        fn row(&self, index: usize, item: &T) -> N {
            self(index, item)
        }
    }
}

/// Row renderer of a virtual list.
pub trait RowRender<T>: row_sealed::Sealed<T> + 'static {}

impl<T, R: row_sealed::Sealed<T> + 'static> RowRender<T> for R {}
