use sigil::prelude::*;

fn main() -> AppResult {
    run("Pages", pages)
}

pub(crate) fn pages() -> impl IntoNode {
    tabs().tab("Home", home).tab("Settings", settings)
}

fn home() -> impl IntoNode {
    let visits = signal(0);
    row()
        .child(text(move || format!("Visits: {}", visits.get())))
        .child(button("Visit").activate(move || visits.update(|n| *n += 1)))
}

fn settings() -> impl IntoNode {
    let dark = signal(false);
    toggle(dark).label("Dark mode")
}
