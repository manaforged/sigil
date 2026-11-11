thread_local! {
    static CELL: core::cell::Cell<f32> = const { core::cell::Cell::new(0.0) };
}

/// Sets the size of the cell that spacing values snap to.
#[cfg(feature = "kittest")]
pub fn cell(cell: f32) {
    CELL.set(snapped(cell));
}

pub(crate) fn swap_cell(cell: f32) -> f32 {
    CELL.replace(snapped(cell))
}

const fn snapped(cell: f32) -> f32 {
    if cell > 0.0 { cell } else { 0.0 }
}

pub(crate) fn cell_size() -> Option<f32> {
    CELL.with(|active| (active.get() > 0.0).then_some(active.get()))
}
