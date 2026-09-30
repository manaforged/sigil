#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::widgets::ModalBuilder;
use std::time::Duration;

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    let config = AppConfig::new("Pomodoro");
    let config = if std::env::args().any(|arg| arg == "--gui") {
        config.size(480, 360).window()
    } else {
        config.terminal()
    };
    sigil::run(config, app_tree)
}

#[derive(Clone, Copy)]
struct Timer {
    work_min: Signal<i32>,
    break_min: Signal<i32>,
    remaining: Signal<i32>,
    running: Signal<bool>,
    on_break: Signal<bool>,
    sessions: Signal<i32>,
}

impl Timer {
    fn tick(self) {
        if !self.running.peek() {
            return;
        }
        let left = self.remaining.peek() - 1;
        if left > 0 {
            self.remaining.set(left);
        } else if self.on_break.peek() {
            self.on_break.set(false);
            self.remaining.set(self.work_min.peek() * 60);
        } else {
            self.sessions.update(|n| *n += 1);
            self.on_break.set(true);
            self.remaining.set(self.break_min.peek() * 60);
        }
    }

    fn toggle(self) {
        self.running.update(|on| *on = !*on);
    }

    fn reset(self) {
        self.running.set(false);
        self.on_break.set(false);
        self.remaining.set(self.work_min.peek() * 60);
    }
}

fn controls(timer: Timer, settings_open: Signal<bool>) -> NodeBuilder {
    let start_label = derived(move || {
        if timer.running.get() {
            "Pause"
        } else {
            "Start"
        }
        .to_string()
    });
    container()
        .layout(LayoutStyle::flex_row().gap(cols(1.0)))
        .child(
            button(start_label)
                .primary()
                .activate(move || timer.toggle()),
        )
        .child(button("Reset").activate(move || timer.reset()))
        .child(button("Settings").activate(move || settings_open.set(true)))
}

fn settings(timer: Timer, open: Signal<bool>) -> ModalBuilder {
    modal(open)
        .title("Settings")
        .child(field("Work minutes", number_field(timer.work_min, 1..=120)))
        .child(field(
            "Break minutes",
            number_field(timer.break_min, 1..=60),
        ))
        .footer(button("Close").activate(move || open.set(false)))
}

pub(crate) fn app_tree() -> impl IntoNode {
    let timer = Timer {
        work_min: signal(25),
        break_min: signal(5),
        remaining: signal(25 * 60),
        running: signal(false),
        on_break: signal(false),
        sessions: signal(0),
    };
    let settings_open = signal(false);
    let _tick: IntervalId = interval(Duration::from_secs(1), move || timer.tick());
    let clock = derived(move || {
        let s = timer.remaining.get();
        format!("{:02}:{:02}", s / 60, s % 60)
    });
    let phase = derived(move || {
        if timer.on_break.get() {
            "Break"
        } else {
            "Work"
        }
        .to_string()
    });
    let count = derived(move || format!("Sessions: {}", timer.sessions.get()));
    let space = Shortcut::plain(LogicalKey::character(" "));
    let r = Shortcut::plain(LogicalKey::character("r"));
    container()
        .layout(
            LayoutStyle::flex_column()
                .fill()
                .gap(rows(1.0))
                .padding(em(1.0)),
        )
        .shortcut(space, move |_| timer.toggle())
        .shortcut(r, move |_| timer.reset())
        .shortcut(LogicalKey::character("s"), move |_| settings_open.set(true))
        .shortcut(LogicalKey::character("q"), |_| sigil::request_exit())
        .child(text(phase))
        .child(text(clock).class("text-xl"))
        .child(text(count))
        .child(controls(timer, settings_open))
        .child(text("space start/pause  r reset  s settings  q quit"))
        .child(settings(timer, settings_open))
}
