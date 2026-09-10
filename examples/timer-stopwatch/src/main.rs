use std::time::Instant;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - timer-stopwatch", 480, 480), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
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
            .class(classes!("bg-surface"))
            .child(text(display_text).class(classes!("text-on-surface text-xl font-mono")))
            .child(text(status_text).class(classes!("text-on-surface text-sm")))
            .child(button_row(start_at, elapsed_ms, laps))
            .child(lap_list(laps)),
    )
}

fn lap_list(laps: Signal<Vec<u64>>) -> NodeBuilder {
    let snapshot = laps.peek();
    let mut col = container().layout(LayoutStyle::flex_column().gap(2.0));
    if snapshot.is_empty() {
        return col
            .child(text("(no laps yet)").class(classes!("text-on-surface text-sm font-mono")));
    }
    for (idx, ms) in snapshot.iter().enumerate() {
        let n = idx + 1;
        let formatted = format_ms(*ms);
        col = col.child(
            kv_row(
                format!("lap {n}"),
                text(formatted).class(classes!("font-mono")),
            )
            .class(classes!("text-on-surface text-sm")),
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
#[path = "main_tests.rs"]
mod tests;
