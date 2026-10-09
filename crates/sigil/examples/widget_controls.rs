#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{
    ButtonBuilder, CheckboxBuilder, RadioGroupBuilder, SliderBuilder, ToggleBuilder,
};

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

pub(crate) fn checkbox_demo() -> CheckboxBuilder {
    checkbox(signal(true)).label("Enabled")
}

pub(crate) fn checkbox_unchecked_demo() -> CheckboxBuilder {
    checkbox(signal(false)).label("Enabled")
}

pub(crate) fn toggle_demo() -> ToggleBuilder {
    toggle(signal(true)).label("Locked")
}

pub(crate) fn toggle_off_demo() -> ToggleBuilder {
    toggle(signal(false)).label("Locked")
}

pub(crate) fn radio_group_demo() -> RadioGroupBuilder {
    radio_group(signal(String::from("auto")))
        .option("auto", "Automatic")
        .option("manual", "Manual")
}

pub(crate) fn slider_demo() -> SliderBuilder {
    slider(signal(40.0), 0.0..=100.0).step(5.0)
}

fn build() -> impl IntoNode {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
        .child(
            container()
                .layout(LayoutStyle::flex_row().gap(cols(1.0)))
                .child(button_demo())
                .child(button_secondary_demo())
                .child(button_ghost_demo())
                .child(button_pending_demo()),
        )
        .child(icon_button_demo())
        .child(checkbox_demo())
        .child(checkbox_unchecked_demo())
        .child(toggle_demo())
        .child(toggle_off_demo())
        .child(radio_group_demo())
        .child(slider_demo())
}
