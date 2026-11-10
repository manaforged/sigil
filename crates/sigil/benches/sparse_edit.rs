#![allow(
    unused_crate_dependencies,
    clippy::expect_used,
    reason = "the benchmark reports measurements and uses a subset of package dependencies"
)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Bencher, BenchmarkId, Criterion, criterion_group, criterion_main};
use sigil::Host;
use sigil::app::FrameHooks;
use sigil::kernel::HostMode;
use sigil::prelude::*;
use sigil::reactive::Owner;

const VIEW: (f32, f32) = (1280.0, 800.0);
const SIZES: [u32; 3] = [1_000, 3_000, 10_000];
const REBUILD_SIZES: [u32; 2] = [1_000, 3_000];

#[derive(Clone)]
struct Thread {
    id: u32,
    title: Signal<String>,
}

fn thread(id: u32) -> Thread {
    Thread {
        id,
        title: signal(format!("thread {id}")),
    }
}

fn thread_row(thread: &Thread) -> NodeBuilder {
    row()
        .child(text(thread.title))
        .child(text("12 replies"))
        .child(button("Open"))
}

struct Inbox {
    host: Host,
    all: Vec<Thread>,
    shown: Signal<Vec<Thread>>,
    extra: Thread,
}

#[derive(Clone, Copy)]
enum Phase {
    Apply,
    Emit,
    Access,
}

impl Phase {
    const ALL: [(Self, &'static str); 3] = [
        (Self::Apply, "apply"),
        (Self::Emit, "emit"),
        (Self::Access, "access"),
    ];
}

impl Inbox {
    fn new(rows: u32) -> Self {
        let all: Vec<Thread> = (0..rows).map(thread).collect();
        let shown = signal(all.clone());
        let mut host = Host::new(HostMode::Pixel);
        host.mount(move || {
            column().child(scroll(for_each(
                shown,
                |thread: &Thread| thread.id,
                thread_row,
            )))
        });
        let mut inbox = Self {
            host,
            all,
            shown,
            extra: thread(u32::MAX),
        };
        inbox.settle();
        inbox
    }

    fn settle(&mut self) {
        self.host
            .frame(VIEW, 0.0, FrameHooks::default())
            .layout
            .expect("layout");
    }

    fn label(&self, turn: u64) {
        self.all[self.all.len() / 2]
            .title
            .set(format!("thread edit {turn}"));
    }

    fn equal(&self, _turn: u64) {
        let title = self.all[self.all.len() / 2].title;
        title.set(title.peek());
    }

    fn append(&self, turn: u64) {
        let extra = self.extra.clone();
        self.shown.update(|shown| {
            if turn.is_multiple_of(2) {
                shown.push(extra);
            } else {
                shown.pop();
            }
        });
    }

    fn filter(&self, turn: u64) {
        let step = if turn.is_multiple_of(2) { 2 } else { 1 };
        self.shown.update(|shown| {
            shown.clear();
            shown.extend(self.all.iter().step_by(step).cloned());
        });
    }

    fn turn(&mut self, edit: fn(&Self, u64), turn: u64, phase: Phase) -> Duration {
        let apply = Instant::now();
        edit(self, turn);
        self.settle();
        let apply = apply.elapsed();
        let emit = Instant::now();
        black_box(self.host.emit(VIEW));
        let emit = emit.elapsed();
        let access = Instant::now();
        black_box(self.host.accessibility_tree());
        let access = access.elapsed();
        match phase {
            Phase::Apply => apply,
            Phase::Emit => emit,
            Phase::Access => access,
        }
    }
}

fn measure(b: &mut Bencher<'_>, rows: u32, edit: fn(&Inbox, u64), phase: Phase) {
    Owner::scope("sparse-edit", || {
        let mut inbox = Inbox::new(rows);
        let mut turn = 0;
        b.iter_custom(|iters| {
            (0..iters)
                .map(|_| {
                    turn += 1;
                    inbox.turn(edit, turn, phase)
                })
                .sum()
        });
    });
}

fn case(c: &mut Criterion, name: &str, sizes: &[u32], edit: fn(&Inbox, u64)) {
    for (phase, label) in Phase::ALL {
        let mut group = c.benchmark_group(format!("sparse_edit/{name}/{label}"));
        group.sample_size(10);
        group.warm_up_time(Duration::from_millis(500));
        group.measurement_time(Duration::from_secs(2));
        for &rows in sizes {
            group.bench_with_input(BenchmarkId::from_parameter(rows), &rows, |b, &rows| {
                measure(b, rows, edit, phase);
            });
        }
        group.finish();
    }
}

fn sparse_edit(c: &mut Criterion) {
    case(c, "label", &SIZES, Inbox::label);
    case(c, "equal", &SIZES, Inbox::equal);
    case(c, "append", &SIZES, Inbox::append);
    case(c, "filter", &REBUILD_SIZES, Inbox::filter);
}

criterion_group!(benches, sparse_edit);
criterion_main!(benches);
