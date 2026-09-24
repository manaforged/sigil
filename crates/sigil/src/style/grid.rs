thread_local! {
    static CELL: core::cell::Cell<f32> = const { core::cell::Cell::new(0.0) };
}

pub fn grid(cell: f32) {
    CELL.with(|active| active.set(if cell > 0.0 { cell } else { 0.0 }));
}

pub(crate) fn swap_cell(cell: f32) -> f32 {
    CELL.with(|active| active.replace(if cell > 0.0 { cell } else { 0.0 }))
}

pub(crate) fn cell_size() -> Option<f32> {
    CELL.with(|active| (active.get() > 0.0).then_some(active.get()))
}
