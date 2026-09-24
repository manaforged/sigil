#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - controls", 480, 360), build)
}

fn button_demo() -> NodeBuilder {
    let saves = signal(0_u32);
    container()
        .layout(LayoutStyle::flex_row().gap(cols(1.0)))
        .child(button("Save").primary().activate(move || {
            saves.set(saves.peek() + 1);
        }))
        .child(button("Cancel"))
        .child(button("Delete").destructive())
}

fn icon_button_demo() -> ButtonBuilder {
    icon_button("×", || {}).accessible("Close")
}

fn checkbox_demo() -> CheckboxBuilder {
    let agreed = signal(false);
    checkbox(agreed).label("Accept terms")
}

fn toggle_demo() -> ToggleBuilder {
    let sync = signal(true);
    toggle(sync).label("Sync")
}

fn radio_group_demo() -> RadioGroupBuilder {
    let mode = signal(String::from("auto"));
    radio_group(mode)
        .label("Mode")
        .option("auto", "Automatic")
        .option("manual", "Manual")
}

fn slider_demo() -> SliderBuilder {
    let volume = signal(40.0_f32);
    slider(volume, 0.0..=100.0).step(5.0).label("Volume")
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(button_demo())
            .child(icon_button_demo())
            .child(checkbox_demo())
            .child(toggle_demo())
            .child(radio_group_demo())
            .child(slider_demo()),
    )
}
