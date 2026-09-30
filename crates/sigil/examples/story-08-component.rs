use sigil::prelude::*;

fn main() -> AppResult {
    run("Steppers", steppers)
}

pub(crate) fn steppers() -> impl IntoNode {
    let width = signal(1);
    let height = signal(5);
    column()
        .child(stepper("Width", width))
        .child(stepper("Height", height))
}

fn stepper(label: &'static str, value: Signal<i32>) -> impl IntoNode {
    row()
        .child(text(label))
        .child(button("-").activate(move || value.update(|v| *v -= 1)))
        .child(text(value))
        .child(button("+").activate(move || value.update(|v| *v += 1)))
}
