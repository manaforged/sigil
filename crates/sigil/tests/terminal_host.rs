#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::indexing_slicing,
    clippy::cast_precision_loss
)]

use sigil::events::{Event, Key};
use sigil::kernel::IntoNode;
use sigil::kernel::{container, text};
use sigil::prelude::Paint;
use sigil::prelude::Widget;
use sigil::reactive::{memo, signal};
use sigil::widgets::button;

use sigil_kittest::Harness;

fn present<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
    events: &[Event],
) -> String {
    Harness::present(tree, cols, rows, events).expect("layout")
}

fn step(
    host: &mut sigil::Host,
    events: impl IntoIterator<Item = Event>,
    cols: u16,
    rows: u16,
) -> sigil::terminal::Grid {
    host.grid(events, cols, rows).expect("layout")
}

fn painted<N: IntoNode>(
    tree: impl Fn() -> N + 'static,
    cols: u16,
    rows: u16,
) -> sigil::terminal::Grid {
    sigil::reactive::Owner::scope("terminal-painted", || {
        let mut host = sigil::Host::new(sigil::kernel::HostMode::Cell);
        host.mount(tree);
        drop(step(&mut host, [], cols, rows));
        step(&mut host, [], cols, rows)
    })
}

fn solid(row: &[sigil::terminal::Cell]) -> usize {
    row.iter()
        .filter(|cell| cell.text().trim().is_empty() && cell.bg != sigil::terminal::Ink::Bare)
        .count()
}

fn fill_cells(grid: &sigil::terminal::Grid) -> usize {
    (0..grid.rows()).map(|row| solid(grid.row(row))).sum()
}

fn counter() -> impl IntoNode {
    let count = signal(0_i32);
    let label = memo(move || format!("Count: {}", count.get()));
    container()
        .child(text(label))
        .child(button("Increment").activate(move || {
            count.set(count.peek() + 1);
        }))
}

#[path = "terminal_host/borders.rs"]
mod borders;
#[path = "terminal_host/cells.rs"]
mod cells;
#[path = "terminal_host/emit.rs"]
mod emit;
#[path = "terminal_host/faults.rs"]
mod faults;
#[path = "terminal_host/frames.rs"]
mod frames;
#[path = "terminal_host/keys.rs"]
mod keys;
#[path = "terminal_host/layers.rs"]
mod layers;
#[path = "terminal_host/layout.rs"]
mod layout;
#[path = "terminal_host/live.rs"]
mod live;
#[path = "terminal_host/oracle.rs"]
mod oracle;
#[path = "terminal_host/paint.rs"]
mod paint;
#[path = "terminal_host/patching.rs"]
mod patching;
#[path = "terminal_host/precise.rs"]
mod precise;
#[path = "terminal_host/retained.rs"]
mod retained;
#[path = "terminal_host/select.rs"]
mod select;
#[path = "terminal_host/table.rs"]
mod table;
#[path = "terminal_host/transforms.rs"]
mod transforms;
#[path = "terminal_host/wrap.rs"]
mod wrap;
