//! Dogfood example — a press-to-record stopwatch.
//!
//! Reactive state:
//! - `start_at: Signal<Option<Instant>>` — when the current run began.
//!   `None` means "not running" (either reset or stopped).
//! - `elapsed_ms: Signal<u64>` — frozen elapsed time captured at every
//!   Stop / Lap press. The display reads this, since Sigil has no
//!   per-frame tick hook for a continuously-rolling display (see
//!   FRICTION.md).
//! - `laps: Signal<Vec<u64>>` — captured lap times.
//!
//! Layout: vertical column with a big monospaced digit row at the top,
//! a Start/Stop/Lap/Reset button row, and a lap-list below.

use std::time::Instant;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - timer-stopwatch", 480, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    // Each Signal::new must run inside a reactive scope; constructing
    // them here under sigil::launch's tree-build callback is the
    // documented spot.
    let start_at: Signal<Option<Instant>> = signal(None);
    let elapsed_ms: Signal<u64> = signal(0_u64);
    let laps: Signal<Vec<u64>> = signal(Vec::<u64>::new());

    let display_text = derived(move || format_ms(elapsed_ms.get()));
    let status_text = derived(move || {
        if start_at.get().is_some() {
            String::from("running — tap Stop or Lap")
        } else if elapsed_ms.get() == 0 {
            String::from("idle — tap Start")
        } else {
            String::from("stopped")
        }
    });

    cx.add(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(480.0, 480.0)
                    .padding(16.0)
                    .gap(12.0),
            )
            .class("bg-surface")
            .child(text(display_text).class("text-on-surface text-xl font-mono"))
            .child(text(status_text).class("text-on-surface text-sm"))
            .child(button_row(start_at, elapsed_ms, laps))
            .child(lap_list(laps)),
    )
}

fn lap_list(laps: Signal<Vec<u64>>) -> NodeBuilder {
    let snapshot = laps.peek();
    let mut col = container().layout(LayoutStyle::flex_column().gap(2.0));
    if snapshot.is_empty() {
        return col.child(text("(no laps yet)").class("text-on-surface text-sm font-mono"));
    }
    for (idx, ms) in snapshot.iter().enumerate() {
        let n = idx + 1;
        let formatted = format_ms(*ms);
        col = col.child(
            kv_row(format!("lap {n}"), text(formatted).class("font-mono"))
                .class("text-on-surface text-sm"),
        );
    }
    col
}

fn button_row(
    start_at: Signal<Option<Instant>>,
    elapsed_ms: Signal<u64>,
    laps: Signal<Vec<u64>>,
) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(button("Start").primary().activate(move || {
            if start_at.peek().is_none() {
                start_at.set(Some(Instant::now()));
            }
        }))
        .child(button("Stop").activate(move || {
            if let Some(at) = start_at.peek() {
                let delta = elapsed_ms.peek() + ms_since(at);
                elapsed_ms.set(delta);
                start_at.set(None);
            }
        }))
        .child(button("Lap").activate(move || {
            let snapshot = current_elapsed(start_at, elapsed_ms);
            let mut next = laps.peek();
            next.push(snapshot);
            laps.set(next);
        }))
        .child(button("Reset").activate(move || {
            start_at.set(None);
            elapsed_ms.set(0);
            laps.set(Vec::new());
        }))
}

fn current_elapsed(start_at: Signal<Option<Instant>>, elapsed_ms: Signal<u64>) -> u64 {
    let base = elapsed_ms.peek();
    start_at.peek().map_or(base, |at| base + ms_since(at))
}

fn ms_since(at: Instant) -> u64 {
    let now = Instant::now();
    if now <= at {
        return 0;
    }
    let dur = now.duration_since(at);
    u64::try_from(dur.as_millis()).unwrap_or(u64::MAX)
}

fn format_ms(ms: u64) -> String {
    let total_secs = ms / 1_000;
    let millis = ms % 1_000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}.{millis:03}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use sigil::testing::with_cx;

    #[test]
    fn tree_builds_without_panic() {
        with_cx("timer-stopwatch-smoke", |cx| {
            let root = app_tree(cx);
            let mut count = 0_usize;
            cx.visit_preorder(root, |_, _| count += 1);
            assert!(count >= 5, "expected at least 5 nodes, got {count}");
        });
    }

    #[test]
    fn format_ms_pads_correctly() {
        assert_eq!(format_ms(0), "00:00.000");
        assert_eq!(format_ms(1_234), "00:01.234");
        assert_eq!(format_ms(61_500), "01:01.500");
    }

    #[test]
    fn lap_list_handles_empty_and_filled() {
        // `lap_list` is the production lap renderer. Build it under a
        // reactive scope and assert it produces a populated container.
        with_cx("timer-stopwatch-lap-list", |cx| {
            let laps: Signal<Vec<u64>> = signal(Vec::new());
            let empty_root = cx.add(lap_list(laps));
            assert!(
                cx.first_child(empty_root).is_some(),
                "empty lap_list still renders the placeholder child"
            );

            laps.set(vec![1_500, 3_000]);
            let filled_root = cx.add(lap_list(laps));
            let mut count = 0_usize;
            cx.visit_preorder(filled_root, |_, _| count += 1);
            assert!(
                count >= 3,
                "two laps should produce at least 3 nodes, got {count}"
            );
        });
    }
}
