#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::ButtonBuilder;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - controls").size(480, 360), build)
}

pub(crate) fn button_demo() -> ButtonBuilder {
    button("Save changes").primary()
}

pub(crate) fn button_secondary_demo() -> ButtonBuilder {
    button("Cancel")
}

pub(crate) fn button_ghost_demo() -> ButtonBuilder {
    button("Details").ghost()
}

pub(crate) fn button_pending_demo() -> ButtonBuilder {
    button("Saving").primary().pending(signal(true))
}

pub(crate) fn icon_button_demo() -> ButtonBuilder {
    icon_button("×", || {}).accessible("Reset")
}

pub(crate) fn checkbox_demo(disabled: Signal<bool>) -> impl IntoNode {
    checkbox(signal(true)).label("Enabled").disabled(disabled)
}

pub(crate) fn checkbox_unchecked_demo(disabled: Signal<bool>) -> impl IntoNode {
    checkbox(signal(false)).label("Enabled").disabled(disabled)
}

pub(crate) fn toggle_demo(disabled: Signal<bool>) -> impl IntoNode {
    toggle(signal(true)).label("Locked").disabled(disabled)
}

pub(crate) fn toggle_off_demo(disabled: Signal<bool>) -> impl IntoNode {
    toggle(signal(false)).label("Locked").disabled(disabled)
}

pub(crate) fn radio_group_demo(disabled: Signal<bool>) -> impl IntoNode {
    radio_group(signal(String::from("auto")))
        .option("auto", "Automatic")
        .option("manual", "Manual")
        .disabled(disabled)
}

pub(crate) fn slider_demo(disabled: Signal<bool>) -> impl IntoNode {
    slider(signal(40.0), 0.0..=100.0)
        .step(5.0)
        .disabled(disabled)
}

fn build() -> impl IntoNode {
    column()
        .role(Role::Main)
        .gap(rows(1.0))
        .padding(cols(2.0))
        .child(
            row()
                .gap(cols(1.0))
                .align_items(AlignItems::Stretch)
                .child(button_demo())
                .child(button_secondary_demo())
                .child(button_ghost_demo())
                .child(button_pending_demo()),
        )
        .child(icon_button_demo())
        .child(checkbox_demo(signal(false)))
        .child(checkbox_unchecked_demo(signal(false)))
        .child(toggle_demo(signal(false)))
        .child(toggle_off_demo(signal(false)))
        .child(radio_group_demo(signal(false)))
        .child(slider_demo(signal(false)))
}
