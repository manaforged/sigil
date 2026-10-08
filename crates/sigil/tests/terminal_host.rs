#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines
)]

use sigil::events::{Key, ShellEvent};
use sigil::kernel::IntoNode;
use sigil::kernel::{container, text};
use sigil::prelude::{LayoutBuilderExt, Paint};
use sigil::reactive::{derived, signal};
use sigil::widgets::button;

use sigil::terminal::{key_down, present as try_present};

fn present<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
    events: &[ShellEvent],
) -> String {
    try_present(tree, cols, rows, events).expect("layout")
}

fn step(
    host: &mut sigil::Host,
    router: &mut sigil::events::EventRouter,
    events: impl IntoIterator<Item = ShellEvent>,
    cols: u16,
    rows: u16,
) -> sigil::terminal::Grid {
    sigil::terminal::step(host, router, events, cols, rows).expect("layout")
}

fn painted<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
) -> sigil::terminal::Grid {
    sigil::reactive::Runtime::scope("terminal-painted", || {
        let mut host = sigil::Host::cells();
        host.mount(tree);
        let mut router = sigil::events::EventRouter::new();
        drop(step(&mut host, &mut router, [], cols, rows));
        step(&mut host, &mut router, [], cols, rows)
    })
}

fn solid(row: &[sigil::terminal::Cell]) -> usize {
    row.iter()
        .filter(|cell| cell.text.trim().is_empty() && cell.bg != sigil::terminal::Ink::Bare)
        .count()
}

fn fill_cells(grid: &sigil::terminal::Grid) -> usize {
    (0..grid.rows()).map(|row| solid(grid.row(row))).sum()
}

fn counter() -> impl IntoNode {
    let count = signal(0_i32);
    let label = derived(move || format!("Count: {}", count.get()));
    container()
        .child(text(label))
        .child(button("Increment").activate(move || {
            count.set(count.peek() + 1);
        }))
}

#[path = "terminal_host/cells.rs"]
mod cells;
#[path = "terminal_host/frames.rs"]
mod frames;
#[path = "terminal_host/keys.rs"]
mod keys;
#[path = "terminal_host/paint.rs"]
mod paint;
#[path = "terminal_host/select.rs"]
mod select;
#[path = "terminal_host/table.rs"]
mod table;
#[path = "terminal_host/transforms.rs"]
mod transforms;
#[path = "terminal_host/wrap.rs"]
mod wrap;
