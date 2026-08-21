//! Interactive dashboard vertical slice for Sigil.
//!
//! Exercises the full public path: tree construction, reactive text,
//! reactive style, layout, pointer/keyboard events, focus, scene
//! emission, and the default `sigil::launch` render loop.

#![allow(missing_docs)]

use sigil::prelude::*;

#[derive(Clone, Copy)]
struct DashboardState {
    hovered_row: Signal<Option<usize>>,
    selected: Signal<usize>,
    running: Signal<bool>,
    queued: Signal<u32>,
    alerts: Signal<bool>,
    auto_retry: Signal<bool>,
    delay_budget: Signal<f32>,
    delay_label: Signal<String>,
    note: Signal<String>,
    submitted_notes: Signal<Vec<String>>,
    submitted_notes_label: Signal<String>,
    status: Signal<String>,
    count_label: Signal<String>,
}

static ROW_BASE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static ROW_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static STRIPE_IDLE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("border")];
static STRIPE_ACTIVE: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - vertical slice", 960, 640), build_app)
}

fn build_app(cx: &mut Cx) -> NodeId {
    let state = DashboardState::new();
    cx.add(app_shell(state))
}

impl DashboardState {
    fn new() -> Self {
        let delay_budget = signal(40.0_f32);
        let delay_label = signal(format_delay(delay_budget.peek()));
        let submitted_notes = signal(Vec::<String>::new());
        let submitted_notes_label = signal(format_notes(&submitted_notes.peek()));
        effect(move || delay_label.set(format_delay(delay_budget.get())));
        effect(move || submitted_notes_label.set(format_notes(&submitted_notes.get())));
        Self {
            hovered_row: signal(None),
            selected: signal(0),
            running: signal(false),
            queued: signal(0),
            alerts: signal(false),
            auto_retry: signal(true),
            delay_budget,
            delay_label,
            note: signal(String::new()),
            submitted_notes,
            submitted_notes_label,
            status: signal(String::from("Ready. Pick a module and queue a run.")),
            count_label: signal(String::from("0 queued")),
        }
    }

    fn queue_run(self) {
        let next = self.queued.peek().saturating_add(1);
        self.queued.set(next);
        self.running.set(true);
        self.count_label.set(format!("{next} queued"));
        self.status
            .set(format!("Run {next} queued for {}.", selected_name(self)));
    }

    fn toggle_pause(self) {
        let next = !self.running.peek();
        self.running.set(next);
        let status = if next { "Resumed" } else { "Paused" };
        self.status.set(format!(
            "{status}: {} remains selected.",
            selected_name(self)
        ));
    }

    fn clear(self) {
        self.running.set(false);
        self.queued.set(0);
        self.count_label.set(String::from("0 queued"));
        self.status.set(String::from("Queue cleared. System idle."));
    }

    fn prime_controls(self) {
        self.status
            .set(format!("Controls primed for {}.", selected_name(self)));
    }

    fn set_alert_status(self, enabled: bool) {
        let status = if enabled { "enabled" } else { "disabled" };
        self.status.set(format!("Restock alerts {status}."));
    }

    fn set_retry_status(self, enabled: bool) {
        let status = if enabled { "enabled" } else { "disabled" };
        self.status.set(format!("Auto retry {status}."));
    }

    fn commit_delay(self, value: f32) {
        self.status
            .set(format!("Delay budget committed at {value:.0}%."));
    }

    fn update_note_status(self, value: &str) {
        self.status.set(format!("Current note: {value}"));
    }

    fn submit_note(self, value: &str) {
        self.submitted_notes
            .update(|notes| notes.push(value.to_owned()));
        self.status.set(format!("Submitted note: {value}"));
    }
}

fn app_shell(state: DashboardState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil vertical slice dashboard")
        .layout(LayoutStyle::flex_row().gap(16.0))
        .class(classes!("w-full h-full p-6 bg-surface"))
        .pointer_move(move |_| set_if_changed(state.hovered_row, None))
        .child(sidebar())
        .child(main_panel(state))
}

fn sidebar() -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(220.0)
                .gap(16.0)
                .padding(16.0),
        )
        .class(classes!("h-full rounded-lg bg-surface-hover"))
        .child(label("SIGIL"))
        .child(label("Vertical slice"))
        .child(label("Layout"))
        .child(label("Text"))
        .child(label("Events"))
        .child(label("Reactive state"))
}

fn main_panel(state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(16.0))
        .class(classes!("h-full"))
        .child(header(state))
        .child(workspace(state))
        .child(status_bar(state))
}

fn header(state: DashboardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(88.0)
                .gap(12.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .padding(16.0),
        )
        .class(classes!("w-full rounded-lg bg-surface-hover"))
        .child(label("Automation control"))
        .child(header_actions(state))
}

fn header_actions(state: DashboardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .gap(12.0)
                .align_items(AlignItems::Center),
        )
        .child(
            button("Queue run")
                .primary()
                .activate(move || state.queue_run()),
        )
        .child(button("Pause").activate(move || state.toggle_pause()))
        .child(button("Clear").activate(move || state.clear()))
}

fn workspace(state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().grow(1.0).gap(16.0))
        .class(classes!("w-full"))
        .child(module_list(state))
        .child(controls_panel(state))
        .child(run_summary(state))
}

fn module_list(state: DashboardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(380.0)
                .gap(12.0)
                .padding(16.0),
        )
        .class(classes!("h-full rounded-lg bg-surface-hover"))
        .child(label("Modules"))
        .child(module_row(
            0,
            "Target monitor",
            "Order flow and stock feed",
            state,
        ))
        .child(module_row(
            1,
            "Fanatics queue",
            "Session health and retry loop",
            state,
        ))
        .child(module_row(
            2,
            "Amazon watcher",
            "Live data stream and alerts",
            state,
        ))
}

fn controls_panel(state: DashboardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .width(240.0)
                .gap(12.0)
                .padding(16.0),
        )
        .class(classes!("h-full rounded-lg bg-surface-hover"))
        .child(label("Controls"))
        .child(
            button("Prime controls")
                .primary()
                .width(180.0)
                .activate(move || state.prime_controls()),
        )
        .child(
            checkbox(state.alerts)
                .label("Restock alerts")
                .tone(Tone::Primary)
                .change(move |enabled| state.set_alert_status(enabled)),
        )
        .child(
            toggle(state.auto_retry)
                .label("Auto retry")
                .change(move |enabled| state.set_retry_status(enabled)),
        )
        .child(note_control(state))
        .child(delay_control(state))
}

fn note_control(state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).padding(12.0))
        .class(classes!("w-full rounded-md bg-surface"))
        .child(label("Notes"))
        .child(
            text_field(state.note)
                .placeholder("Add note")
                .change(move |value| state.update_note_status(value))
                .submit(move |value| state.submit_note(value)),
        )
        .child(label("Submitted notes"))
        .child(label(state.submitted_notes_label))
}

fn delay_control(state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).padding(12.0))
        .class(classes!("w-full rounded-md bg-surface"))
        .child(label("Delay budget"))
        .child(
            slider(state.delay_budget, 0.0..=100.0)
                .step(5.0)
                .change(move |value| state.commit_delay(value)),
        )
}

fn run_summary(state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).gap(16.0).padding(16.0))
        .class(classes!("h-full rounded-lg bg-surface-hover"))
        .child(label("Proof points"))
        .child(summary_row("Reactive text", state.count_label))
        .child(summary_row("Selected module", state.status))
        .child(label(
            "Resize the window, click rows, tab to buttons, press Enter.",
        ))
}

fn status_bar(state: DashboardState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(56.0)
                .gap(16.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .padding(16.0),
        )
        .class(classes!("w-full rounded-lg bg-surface-hover"))
        .child(label(state.status).layout(LayoutStyle::block().grow(1.0)))
        .child(label(state.delay_label).layout(LayoutStyle::block().width(160.0)))
}

fn module_row(
    index: usize,
    title: &'static str,
    detail: &'static str,
    state: DashboardState,
) -> NodeBuilder {
    container()
        .role(Role::Button)
        .label(title)
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(72.0)
                .gap(12.0)
                .align_items(AlignItems::Center)
                .padding(12.0),
        )
        .class(classes!("w-full rounded-md"))
        .style_reactive(move || row_style(state, index))
        .pointer_move(move |event| hover_row(event, state, index))
        .pointer_down(move |event| select_row(event, state, index))
        .key(Key::Enter, move |event| select_row_key(event, state, index))
        .child(row_stripe(index, state))
        .child(row_copy(title, detail))
}

fn row_copy(title: &'static str, detail: &'static str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(4.0))
        .child(label(title))
        .child(label(detail))
}

fn row_stripe(index: usize, state: DashboardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(8.0, 48.0))
        .class(classes!("rounded-sm"))
        .style_reactive(move || stripe_style(state, index))
}

fn summary_row(label_text: &'static str, value: Signal<String>) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(6.0).padding(12.0))
        .class(classes!("w-full rounded-md bg-surface"))
        .child(label(label_text))
        .child(label(value))
}

fn row_style(state: DashboardState, index: usize) -> &'static [StyleAtom] {
    if state.selected.get() == index || state.hovered_row.get() == Some(index) {
        ROW_HOVER
    } else {
        ROW_BASE
    }
}

fn stripe_style(state: DashboardState, index: usize) -> &'static [StyleAtom] {
    if state.selected.get() == index {
        STRIPE_ACTIVE
    } else {
        STRIPE_IDLE
    }
}

fn hover_row(event: &mut PointerEvent, state: DashboardState, index: usize) {
    set_if_changed(state.hovered_row, Some(index));
    event.stop_propagation();
}

fn select_row(event: &mut PointerEvent, state: DashboardState, index: usize) {
    select_module(state, index);
    event.stop_propagation();
}

fn select_row_key(event: &mut KeyEvent, state: DashboardState, index: usize) {
    select_module(state, index);
    event.stop_propagation();
}

fn select_module(state: DashboardState, index: usize) {
    set_if_changed(state.selected, index);
    state
        .status
        .set(format!("{} selected.", module_title(index)));
}

fn selected_name(state: DashboardState) -> &'static str {
    module_title(state.selected.peek())
}

const fn module_title(index: usize) -> &'static str {
    match index {
        0 => "Target monitor",
        1 => "Fanatics queue",
        2 => "Amazon watcher",
        _ => "Unknown module",
    }
}

fn format_delay(value: f32) -> String {
    format!("Delay budget {value:.0}%")
}

fn format_notes(notes: &[String]) -> String {
    if notes.is_empty() {
        String::from("No notes submitted.")
    } else {
        format!("Submitted notes: {}", notes.join(" | "))
    }
}

fn set_if_changed<T>(signal: Signal<T>, next: T)
where
    T: Clone + PartialEq + 'static,
{
    if signal.peek() != next {
        signal.set(next);
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
